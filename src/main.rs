use std::sync::Arc;
use std::time::Duration;

use cellz::cell::CellManager;
use cellz::config::Config;
use cellz::error::Result;
use cellz::server::create_router;
use cellz::storage::LocalBlobStore;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::default();
    let storage: Arc<dyn cellz::storage::BlobStore> = if config.storage_backend == "s3" {
        let endpoint = config.s3_endpoint.as_deref().unwrap_or_default();
        let bucket = config.s3_bucket.as_deref().unwrap_or_default();
        let access_key = config.s3_access_key_id.as_deref().unwrap_or_default();
        let secret_key = config.s3_secret_access_key.as_deref().unwrap_or_default();
        let region = config.s3_region.as_deref();

        info!("📦 Initializing S3/R2 BlobStore for bucket '{}' at '{}'", bucket, endpoint);
        Arc::new(cellz::storage::S3BlobStore::new(
            endpoint,
            bucket,
            access_key,
            secret_key,
            region,
        )?)
    } else {
        info!("📂 Initializing Local Filesystem BlobStore at {:?}", config.storage_dir);
        Arc::new(LocalBlobStore::new(&config.storage_dir))
    };
    let manager = Arc::new(CellManager::new(&config.data_dir, storage, config.lease_ttl_secs));

    // Background task to periodically renew leases for all active in-memory cells
    let lease_manager = Arc::clone(&manager);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(20));
        loop {
            interval.tick().await;
            lease_manager.renew_active_leases().await;
        }
    });

    // Background task to evict idle cells from memory
    let idle_manager = Arc::clone(&manager);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            idle_manager.evict_idle_cells(Duration::from_secs(300)).await;
        }
    });

    let app = create_router(manager);

    let addr = format!("{}:{}", config.host, config.port);
    info!("🚀 cellz daemon running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}
