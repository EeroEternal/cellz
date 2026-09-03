use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Result};
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
}

impl CellManager {
    pub fn new(cells_dir: impl AsRef<Path>, storage: Arc<dyn BlobStore>) -> Self {
        Self {
            node_id: Uuid::new_v4().to_string(),
            cells_dir: cells_dir.as_ref().to_path_buf(),
            storage,
            active_cells: Arc::new(RwLock::new(HashMap::new())),
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

        // 3. Acquire distributed single-writer lease (TTL 60s)
        let acquired = self.storage.acquire_lease(cell_id, &self.node_id, 60).await?;
        if !acquired {
            bail!("Cell '{}' is currently leased by another active node", cell_id);
        }

        // 4. Check if local database exists, otherwise try restoring from BlobStore
        let db_path = self.cell_db_path(cell_id);
        if !fs::try_exists(&db_path).await.unwrap_or(false) {
            let blob_key = format!("cells/{}.db", cell_id);
            if self.storage.exists(&blob_key).await.unwrap_or(false) {
                info!("Restoring cell '{}' from storage snapshot '{}'", cell_id, blob_key);
                let bytes = self.storage.get(&blob_key).await?;
                if let Some(parent) = db_path.parent() {
                    fs::create_dir_all(parent).await?;
                }
                fs::write(&db_path, bytes).await?;
            }
        }

        // 5. Open SQLite DB and spawn actor
        let db = CellDb::open(cell_id, &db_path, None).await?;
        let handle = CellActor::spawn(cell_id, db);
        active.insert(cell_id.to_string(), handle.clone());

        info!("Cell '{}' successfully activated on node '{}'", cell_id, self.node_id);
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

        let acquired = self.storage.acquire_lease(&cell_id, &self.node_id, 60).await?;
        if !acquired {
            bail!("Failed to acquire lease for new cell '{}'", cell_id);
        }

        let db_path = self.cell_db_path(&cell_id);
        let default_name = name.as_deref().unwrap_or(&cell_id);
        let db = CellDb::open(&cell_id, &db_path, Some(default_name)).await?;

        if let Some(meta) = metadata {
            db.set_kv("__meta", &meta).await?;
        }

        let handle = CellActor::spawn(&cell_id, db);
        active.insert(cell_id.clone(), handle.clone());

        info!("Created and activated new cell '{}'", cell_id);
        Ok(handle)
    }

    /// Persist SQLite database snapshot to BlobStore.
    pub async fn backup_cell(&self, cell_id: &str) -> Result<()> {
        let db_path = self.cell_db_path(cell_id);
        if !fs::try_exists(&db_path).await.unwrap_or(false) {
            bail!("Cell database file not found for '{}'", cell_id);
        }

        // If cell is active, trigger a WAL checkpoint first
        if let Some(handle) = self.active_cells.read().await.get(cell_id) {
            let _ = handle.checkpoint_wal().await;
        }

        let bytes = fs::read(&db_path).await?;
        let blob_key = format!("cells/{}.db", cell_id);
        self.storage.put(&blob_key, bytes).await?;
        info!("Cell '{}' snapshot successfully backed up to '{}'", cell_id, blob_key);
        Ok(())
    }

    /// Evict cell from memory: checkpoints WAL, backs up to storage, stops actor, and releases lease.
    pub async fn evict_cell(&self, cell_id: &str) -> Result<()> {
        let handle = {
            let mut active = self.active_cells.write().await;
            active.remove(cell_id)
        };

        if let Some(h) = handle {
            info!("Evicting cell '{}' from memory", cell_id);
            let _ = self.backup_cell(cell_id).await;
            h.shutdown().await;
            let _ = self.storage.release_lease(cell_id, &self.node_id).await;
        }
        Ok(())
    }

    /// List all cells on this node (both active in-memory and on-disk).
    pub async fn list_cells(&self) -> Result<Vec<CellMeta>> {
        fs::create_dir_all(&self.cells_dir).await?;
        let mut entries = fs::read_dir(&self.cells_dir).await?;
        let mut results = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("db") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };

            // Check if already in active memory
            let active_handle = self.active_cells.read().await.get(stem).cloned();
            if let Some(handle) = active_handle
                && let Ok(meta) = handle.get_meta().await
            {
                results.push(meta);
                continue;
            }

            // Otherwise query read-only metadata from sqlite file
            if let Ok(db) = CellDb::open(stem, &path, None).await {
                if let Ok(meta) = db.get_meta().await {
                    results.push(meta);
                }
                db.close().await;
            }
        }

        Ok(results)
    }

    /// Background task to renew leases for all active in-memory cells.
    pub async fn renew_active_leases(&self) {
        let active = self.active_cells.read().await;
        for (cell_id, _) in active.iter() {
            if let Err(e) = self.storage.renew_lease(cell_id, &self.node_id, 60).await {
                warn!("Failed to renew lease for cell '{}': {}", cell_id, e);
            }
        }
    }
}
