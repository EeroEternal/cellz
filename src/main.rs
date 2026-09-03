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
    let storage = Arc::new(LocalBlobStore::new(&config.storage_dir));
    let manager = Arc::new(CellManager::new(&config.data_dir, storage));

    // Background task to periodically renew leases for all active in-memory cells
    let lease_manager = Arc::clone(&manager);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(20));
        loop {
            interval.tick().await;
            lease_manager.renew_active_leases().await;
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
