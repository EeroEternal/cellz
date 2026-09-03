use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn get(&self, path: &str) -> Result<Vec<u8>>;
    async fn put(&self, path: &str, data: Vec<u8>) -> Result<()>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Try to acquire a single-writer lease. Returns true if acquired or successfully renewed by same holder.
    async fn acquire_lease(&self, key: &str, holder: &str, ttl_secs: u64) -> Result<bool>;

    /// Renew an active lease held by holder. Returns false if lease has been stolen or expired.
    async fn renew_lease(&self, key: &str, holder: &str, ttl_secs: u64) -> Result<bool>;

    /// Release an active lease held by holder.
    async fn release_lease(&self, key: &str, holder: &str) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LeaseRecord {
    holder: String,
    expires_at: DateTime<Utc>,
}

/// Local Filesystem implementation of BlobStore.
/// Zero external infrastructure needed; uses atomic writes and lock files.
#[derive(Debug, Clone)]
pub struct LocalBlobStore {
    root_dir: PathBuf,
}

impl LocalBlobStore {
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
        }
    }

    fn full_path(&self, relative: &str) -> PathBuf {
        let cleaned = relative.trim_start_matches('/');
        self.root_dir.join(cleaned)
    }

    fn lease_path(&self, key: &str) -> PathBuf {
        self.root_dir.join("leases").join(format!("{}.lease", key))
    }
}

#[async_trait]
impl BlobStore for LocalBlobStore {
    async fn get(&self, path: &str) -> Result<Vec<u8>> {
        let target = self.full_path(path);
        fs::read(&target)
            .await
            .with_context(|| format!("Failed to read blob at {:?}", target))
    }

    async fn put(&self, path: &str, data: Vec<u8>) -> Result<()> {
        let target = self.full_path(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).await?;
        }
        let temp_path = target.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        fs::write(&temp_path, data).await?;
        fs::rename(&temp_path, &target).await?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let target = self.full_path(path);
        if target.exists() {
            fs::remove_file(target).await?;
        }
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let target = self.full_path(path);
        Ok(tokio::fs::try_exists(target).await.unwrap_or(false))
    }

    async fn acquire_lease(&self, key: &str, holder: &str, ttl_secs: u64) -> Result<bool> {
        let l_path = self.lease_path(key);
        if let Some(parent) = l_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let now = Utc::now();
        let existing_lease = fs::read(&l_path)
            .await
            .ok()
            .and_then(|bytes| serde_json::from_slice::<LeaseRecord>(&bytes).ok());
        if let Some(record) = existing_lease
            && record.expires_at > now
            && record.holder != holder
        {
            // Lease held by another node
            return Ok(false);
        }

        let record = LeaseRecord {
            holder: holder.to_string(),
            expires_at: now + Duration::seconds(ttl_secs as i64),
        };
        let data = serde_json::to_vec(&record)?;
        let temp_path = l_path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        fs::write(&temp_path, data).await?;
        fs::rename(&temp_path, &l_path).await?;
        Ok(true)
    }

    async fn renew_lease(&self, key: &str, holder: &str, ttl_secs: u64) -> Result<bool> {
        let l_path = self.lease_path(key);
        let now = Utc::now();
        let Ok(bytes) = fs::read(&l_path).await else {
            return Ok(false);
        };

        let record: LeaseRecord = serde_json::from_slice(&bytes)?;
        if record.holder != holder || record.expires_at <= now {
            return Ok(false);
        }

        let renewed = LeaseRecord {
            holder: holder.to_string(),
            expires_at: now + Duration::seconds(ttl_secs as i64),
        };
        let data = serde_json::to_vec(&renewed)?;
        let temp_path = l_path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        fs::write(&temp_path, data).await?;
        fs::rename(&temp_path, &l_path).await?;
        Ok(true)
    }

    async fn release_lease(&self, key: &str, holder: &str) -> Result<()> {
        let l_path = self.lease_path(key);
        let existing_lease = fs::read(&l_path)
            .await
            .ok()
            .and_then(|bytes| serde_json::from_slice::<LeaseRecord>(&bytes).ok());
        if let Some(record) = existing_lease
            && record.holder == holder
        {
            fs::remove_file(&l_path).await.ok();
        }
        Ok(())
    }
}
