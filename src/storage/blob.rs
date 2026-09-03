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
        use tokio::io::AsyncWriteExt;
        let l_path = self.lease_path(key);
        if let Some(parent) = l_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let now = Utc::now();
        let record = LeaseRecord {
            holder: holder.to_string(),
            expires_at: now + Duration::seconds(ttl_secs as i64),
        };
        let data = serde_json::to_vec(&record)?;

        // 1. Try atomic create first (O_CREAT | O_EXCL)
        let open_res = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&l_path)
            .await;

        if let Ok(mut file) = open_res {
            file.write_all(&data).await?;
            file.flush().await?;
            return Ok(true);
        }

        // 2. File already exists. Check if expired or held by same holder.
        let existing_bytes = fs::read(&l_path).await.ok();
        let existing_lease = existing_bytes
            .and_then(|bytes| serde_json::from_slice::<LeaseRecord>(&bytes).ok());

        if let Some(record) = existing_lease
            && record.expires_at > now
            && record.holder != holder
        {
            return Ok(false);
        }

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

/// S3 / Cloudflare R2 implementation of BlobStore.
#[derive(Debug)]
pub struct S3BlobStore {
    store: std::sync::Arc<object_store::aws::AmazonS3>,
}

impl S3BlobStore {
    pub fn new(
        endpoint: &str,
        bucket: &str,
        access_key_id: &str,
        secret_access_key: &str,
        region: Option<&str>,
    ) -> Result<Self> {
        let store = object_store::aws::AmazonS3Builder::new()
            .with_endpoint(endpoint)
            .with_bucket_name(bucket)
            .with_access_key_id(access_key_id)
            .with_secret_access_key(secret_access_key)
            .with_region(region.unwrap_or("auto"))
            .build()
            .context("Failed to build S3/R2 client")?;

        Ok(Self {
            store: std::sync::Arc::new(store),
        })
    }

    fn obj_path(path: &str) -> object_store::path::Path {
        let cleaned = path.trim_start_matches('/');
        object_store::path::Path::from(cleaned)
    }

    fn lease_obj_path(key: &str) -> object_store::path::Path {
        object_store::path::Path::from(format!("leases/{}.lease", key))
    }
}

#[async_trait]
impl BlobStore for S3BlobStore {
    async fn get(&self, path: &str) -> Result<Vec<u8>> {
        use object_store::ObjectStore;
        let op = Self::obj_path(path);
        let res = self
            .store
            .get(&op)
            .await
            .with_context(|| format!("Failed to fetch S3 object at {}", path))?;
        let bytes = res.bytes().await.context("Failed to stream S3 object bytes")?;
        Ok(bytes.to_vec())
    }

    async fn put(&self, path: &str, data: Vec<u8>) -> Result<()> {
        use object_store::ObjectStore;
        let op = Self::obj_path(path);
        self.store
            .put(&op, data.into())
            .await
            .with_context(|| format!("Failed to put S3 object at {}", path))?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        use object_store::ObjectStore;
        let op = Self::obj_path(path);
        self.store
            .delete(&op)
            .await
            .with_context(|| format!("Failed to delete S3 object at {}", path))?;
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        use object_store::ObjectStore;
        let op = Self::obj_path(path);
        match self.store.head(&op).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn acquire_lease(&self, key: &str, holder: &str, ttl_secs: u64) -> Result<bool> {
        use object_store::{ObjectStore, PutMode, PutOptions, UpdateVersion};
        let lp = Self::lease_obj_path(key);
        let now = Utc::now();

        let record = LeaseRecord {
            holder: holder.to_string(),
            expires_at: now + Duration::seconds(ttl_secs as i64),
        };
        let data = serde_json::to_vec(&record)?;

        // 1. Try atomic create first (If-None-Match: *)
        let create_opts = PutOptions {
            mode: PutMode::Create,
            ..Default::default()
        };

        match self.store.put_opts(&lp, data.clone().into(), create_opts).await {
            Ok(_) => return Ok(true),
            Err(object_store::Error::AlreadyExists { .. }) => {}
            Err(e) => return Err(e.into()),
        }

        // 2. Object already exists. Read current lease with ETag for CAS update.
        let get_res = match self.store.get(&lp).await {
            Ok(res) => res,
            Err(object_store::Error::NotFound { .. }) => return Ok(false),
            Err(e) => return Err(e.into()),
        };

        let e_tag = get_res.meta.e_tag.clone();
        let bytes = get_res.bytes().await?;
        let existing: LeaseRecord = match serde_json::from_slice(&bytes) {
            Ok(r) => r,
            Err(_) => return Ok(false),
        };

        // If held by someone else and still valid -> deny
        if existing.expires_at > now && existing.holder != holder {
            return Ok(false);
        }

        // Lease expired or held by same holder: atomically update using ETag
        let update_opts = PutOptions {
            mode: PutMode::Update(UpdateVersion {
                e_tag,
                version: None,
            }),
            ..Default::default()
        };

        match self.store.put_opts(&lp, data.into(), update_opts).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::Precondition { .. }) => {
                // Another node raced and won
                Ok(false)
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn renew_lease(&self, key: &str, holder: &str, ttl_secs: u64) -> Result<bool> {
        use object_store::{ObjectStore, PutMode, PutOptions, UpdateVersion};
        let lp = Self::lease_obj_path(key);
        let now = Utc::now();

        let get_res = match self.store.get(&lp).await {
            Ok(res) => res,
            Err(_) => return Ok(false),
        };

        let e_tag = get_res.meta.e_tag.clone();
        let bytes = match get_res.bytes().await {
            Ok(b) => b,
            Err(_) => return Ok(false),
        };

        let record: LeaseRecord = match serde_json::from_slice(&bytes) {
            Ok(r) => r,
            Err(_) => return Ok(false),
        };

        if record.holder != holder || record.expires_at <= now {
            return Ok(false);
        }

        let renewed = LeaseRecord {
            holder: holder.to_string(),
            expires_at: now + Duration::seconds(ttl_secs as i64),
        };
        let data = serde_json::to_vec(&renewed)?;

        let update_opts = PutOptions {
            mode: PutMode::Update(UpdateVersion {
                e_tag,
                version: None,
            }),
            ..Default::default()
        };

        match self.store.put_opts(&lp, data.into(), update_opts).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::Precondition { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn release_lease(&self, key: &str, holder: &str) -> Result<()> {
        use object_store::ObjectStore;
        let lp = Self::lease_obj_path(key);
        let existing_lease: Option<LeaseRecord> = match self.store.get(&lp).await {
            Ok(res) => {
                let bytes = res.bytes().await.ok();
                bytes.and_then(|b| serde_json::from_slice(&b).ok())
            }
            Err(_) => None,
        };

        if let Some(record) = existing_lease
            && record.holder == holder
        {
            self.store.delete(&lp).await.ok();
        }
        Ok(())
    }
}
