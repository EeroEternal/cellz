use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, bail};
use serde_json::Value;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::cell::actor::{CellActor, CellHandle};
use crate::cell::db::CellDb;
use crate::model::state::CellMeta;
use crate::storage::BlobStore;

pub struct CellManager {
    node_id: String,
    cells_dir: PathBuf,
    storage: Arc<dyn BlobStore>,
    active_cells: Arc<RwLock<HashMap<String, CellHandle>>>,
    lease_ttl_secs: u64,
}

impl CellManager {
    pub fn new(
        cells_dir: impl AsRef<Path>,
        storage: Arc<dyn BlobStore>,
        lease_ttl_secs: u64,
    ) -> Self {
        Self {
            node_id: Uuid::new_v4().to_string(),
            cells_dir: cells_dir.as_ref().to_path_buf(),
            storage,
            active_cells: Arc::new(RwLock::new(HashMap::new())),
            lease_ttl_secs,
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn cell_db_path(&self, cell_id: &str) -> PathBuf {
        self.cells_dir.join(format!("{}.db", cell_id))
    }

    /// Retrieve an active cell or activate it from local disk / blob storage.
    pub async fn get_or_activate(&self, cell_id: &str) -> Result<CellHandle> {
        // 1. Fast path: check in-memory map
        {
            let active = self.active_cells.read().await;
            if let Some(handle) = active.get(cell_id) {
                return Ok(handle.clone());
            }
        }

        // 2. Slow path: write lock to ensure single activation
        let mut active = self.active_cells.write().await;
        if let Some(handle) = active.get(cell_id) {
            return Ok(handle.clone());
        }

        // 3. Acquire distributed single-writer lease
        let acquired = self
            .storage
            .acquire_lease(cell_id, &self.node_id, self.lease_ttl_secs)
            .await?;
        if !acquired {
            bail!(
                "Cell '{}' is currently leased by another active node",
                cell_id
            );
        }

        // 4. Check if remote snapshot is newer than local database
        let db_path = self.cell_db_path(cell_id);
        let blob_key = format!("cells/{}.db", cell_id);
        let meta_key = format!("cells/{}.meta.json", cell_id);

        let remote_meta: Option<CellMeta> = if self.storage.exists(&meta_key).await.unwrap_or(false)
        {
            if let Ok(bytes) = self.storage.get(&meta_key).await {
                serde_json::from_slice(&bytes).ok()
            } else {
                None
            }
        } else {
            None
        };

        let should_download = if !fs::try_exists(&db_path).await.unwrap_or(false) {
            self.storage.exists(&blob_key).await.unwrap_or(false)
        } else if let Some(ref r_meta) = remote_meta {
            match CellDb::peek_meta(&db_path) {
                Ok(local) => r_meta.event_sequence > local.event_sequence,
                Err(_) => true,
            }
        } else {
            false
        };

        if should_download {
            info!(
                "Restoring cell '{}' from storage snapshot '{}'",
                cell_id, blob_key
            );
            let bytes = self.storage.get(&blob_key).await?;
            if let Some(parent) = db_path.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::write(&db_path, bytes).await?;
        }

        // 5. Spawn dedicated actor thread that owns the rusqlite connection
        let handle = CellActor::spawn(cell_id, &db_path, None)?;
        active.insert(cell_id.to_string(), handle.clone());

        info!(
            "Cell '{}' successfully activated on node '{}'",
            cell_id, self.node_id
        );
        Ok(handle)
    }

    /// Create a new cell and immediately activate it.
    pub async fn create_cell(
        &self,
        id: Option<String>,
        name: Option<String>,
        metadata: Option<Value>,
    ) -> Result<CellHandle> {
        let cell_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());

        let mut active = self.active_cells.write().await;
        if active.contains_key(&cell_id) {
            bail!("Cell with id '{}' is already active", cell_id);
        }

        let acquired = self
            .storage
            .acquire_lease(&cell_id, &self.node_id, self.lease_ttl_secs)
            .await?;
        if !acquired {
            bail!("Failed to acquire lease for new cell '{}'", cell_id);
        }

        let db_path = self.cell_db_path(&cell_id);
        let default_name = name.unwrap_or_else(|| cell_id.clone());
        let handle = CellActor::spawn(&cell_id, &db_path, Some(default_name))?;

        if let Some(meta) = metadata {
            handle.set_kv("__meta", meta).await?;
        }

        active.insert(cell_id.clone(), handle.clone());

        info!("Created and activated new cell '{}'", cell_id);
        Ok(handle)
    }

    /// Persist SQLite database snapshot to BlobStore serialized within actor mailbox.
    pub async fn backup_cell(&self, cell_id: &str) -> Result<()> {
        let (bytes, meta) = if let Some(handle) = self.active_cells.read().await.get(cell_id) {
            let bytes = handle.backup().await?;
            let meta = handle.get_meta().await?;
            (bytes, meta)
        } else {
            let db_path = self.cell_db_path(cell_id);
            if !fs::try_exists(&db_path).await.unwrap_or(false) {
                bail!("Cell database file not found for '{}'", cell_id);
            }
            CellDb::snapshot_file(cell_id, &db_path)?
        };

        let blob_key = format!("cells/{}.db", cell_id);
        self.storage.put(&blob_key, bytes).await?;

        let meta_key = format!("cells/{}.meta.json", cell_id);
        let meta_json = serde_json::to_vec(&meta)?;
        self.storage.put(&meta_key, meta_json).await?;

        info!(
            "Cell '{}' snapshot successfully backed up to '{}'",
            cell_id, blob_key
        );
        Ok(())
    }

    /// Evict cell from memory: checkpoints WAL, backs up to storage, stops actor, and releases lease.
    pub async fn evict_cell(&self, cell_id: &str) -> Result<()> {
        let handle = {
            let mut active = self.active_cells.write().await;
            active.remove(cell_id)
        };

        if let Some(handle) = handle {
            // Backup before shutdown
            let _ = self.backup_cell(cell_id).await;
            handle.shutdown().await;
        }

        // Release lease
        self.storage.release_lease(cell_id, &self.node_id).await?;
        info!(
            "Evicted cell '{}' and released lease on node '{}'",
            cell_id, self.node_id
        );
        Ok(())
    }

    /// List all cells on this node.
    pub async fn list_cells(&self) -> Result<Vec<CellMeta>> {
        let mut results = Vec::new();

        let mut read_dir = match fs::read_dir(&self.cells_dir).await {
            Ok(rd) => rd,
            Err(_) => return Ok(results),
        };

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("db") {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();

            // If active in memory, query live actor
            if let Some(handle) = self.active_cells.read().await.get(stem)
                && let Ok(meta) = handle.get_meta().await
            {
                results.push(meta);
                continue;
            }

            // Otherwise query read-only metadata from sqlite file
            if let Ok(meta) = CellDb::peek_meta(&path) {
                results.push(meta);
            }
        }

        Ok(results)
    }

    /// Background task to renew leases for all active in-memory cells, fencing cells on failure.
    pub async fn renew_active_leases(&self) {
        let mut to_fence = Vec::new();
        {
            let active = self.active_cells.read().await;
            for (cell_id, handle) in active.iter() {
                match self
                    .storage
                    .renew_lease(cell_id, &self.node_id, self.lease_ttl_secs)
                    .await
                {
                    Ok(true) => {}
                    Ok(false) | Err(_) => {
                        warn!(
                            "Lease renewal failed for cell '{}'. Fencing cell to prevent split-brain.",
                            cell_id
                        );
                        to_fence.push((cell_id.clone(), handle.clone()));
                    }
                }
            }
        }

        if !to_fence.is_empty() {
            let mut active = self.active_cells.write().await;
            for (cell_id, handle) in to_fence {
                active.remove(&cell_id);
                handle.fence().await;
            }
        }
    }

    /// Evict cells that have been idle for longer than `max_idle`.
    pub async fn evict_idle_cells(&self, max_idle: std::time::Duration) {
        let mut to_evict = Vec::new();
        {
            let active = self.active_cells.read().await;
            for (cell_id, handle) in active.iter() {
                if let Ok(idle) = handle.idle_duration().await
                    && idle > max_idle
                {
                    to_evict.push(cell_id.clone());
                }
            }
        }

        for cell_id in to_evict {
            info!("Evicting idle cell '{}'", cell_id);
            let _ = self.evict_cell(&cell_id).await;
        }
    }
}
