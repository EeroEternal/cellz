use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub data_dir: PathBuf,
    pub storage_dir: PathBuf,
    pub lease_ttl_secs: u64,
    pub storage_backend: String,
    pub s3_endpoint: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_access_key_id: Option<String>,
    pub s3_secret_access_key: Option<String>,
    pub s3_region: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let host = std::env::var("CELLZ_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("CELLZ_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        let data_dir = std::env::var("CELLZ_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/cells"));
        let storage_dir = std::env::var("CELLZ_STORAGE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/storage"));
        let lease_ttl_secs = std::env::var("CELLZ_LEASE_TTL")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(60);

        let s3_endpoint = std::env::var("CELLZ_S3_ENDPOINT").ok();
        let s3_bucket = std::env::var("CELLZ_S3_BUCKET").ok();
        let s3_access_key_id = std::env::var("CELLZ_S3_ACCESS_KEY_ID")
            .or_else(|_| std::env::var("AWS_ACCESS_KEY_ID"))
            .ok();
        let s3_secret_access_key = std::env::var("CELLZ_S3_SECRET_ACCESS_KEY")
            .or_else(|_| std::env::var("AWS_SECRET_ACCESS_KEY"))
            .ok();
        let s3_region = std::env::var("CELLZ_S3_REGION")
            .or_else(|_| std::env::var("AWS_REGION"))
            .ok();

        let storage_backend = std::env::var("CELLZ_STORAGE_BACKEND").unwrap_or_else(|_| {
            if s3_bucket.is_some() || s3_endpoint.is_some() {
                "s3".to_string()
            } else {
                "local".to_string()
            }
        });

        Self {
            host,
            port,
            data_dir,
            storage_dir,
            lease_ttl_secs,
            storage_backend,
            s3_endpoint,
            s3_bucket,
            s3_access_key_id,
            s3_secret_access_key,
            s3_region,
        }
    }
}
