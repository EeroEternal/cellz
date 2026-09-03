use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub data_dir: PathBuf,
    pub storage_dir: PathBuf,
    pub lease_ttl_secs: u64,
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

        Self {
            host,
            port,
            data_dir,
            storage_dir,
            lease_ttl_secs,
        }
    }
}
