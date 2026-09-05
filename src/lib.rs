//! SQLite-per-cell state and stream plane for isolated actors.
//!
//! Each cell is a single-writer SQLite unit with event sourcing, message
//! projection, generic KV, and checkpoints. `cellz` does not execute user code,
//! call models, or specialize in AI agents — an agent is one kind of client.
//! Enable `server` (default) for lossless SSE / WebSocket replay over HTTP.
//! Enable `s3` for durable leases and snapshots on S3 / Cloudflare R2; the
//! default blob backend is the local filesystem.
//!
//! # Install the daemon
//!
//! ```text
//! cargo install cellz
//! cellz
//! ```
//!
//! S3 / R2 support: `cargo install cellz --features s3`.
//!
//! # Embed as a library
//!
//! In-process core — no HTTP stack, no object_store:
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use cellz::cell::CellManager;
//! use cellz::config::Config;
//! use cellz::storage::LocalBlobStore;
//!
//! # #[tokio::main]
//! # async fn main() -> anyhow::Result<()> {
//! let config = Config::default();
//! let storage = Arc::new(LocalBlobStore::new(&config.storage_dir));
//! let manager = CellManager::new(
//!     &config.data_dir,
//!     storage,
//!     config.lease_ttl_secs,
//! );
//! let _handle = manager
//!     .create_cell(Some("cell-1".into()), Some("demo".into()), None)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ```toml
//! # gitcell / in-process embed
//! cellz = { version = "0.2", default-features = false }
//!
//! # HTTP daemon API (default)
//! cellz = "0.2"
//!
//! # + S3 / R2 snapshots
//! cellz = { version = "0.2", features = ["s3"] }
//! ```

pub mod cell;
pub mod config;
pub mod error;
pub mod model;
pub mod storage;

#[cfg(feature = "server")]
pub mod api;
#[cfg(feature = "server")]
pub mod server;

pub use config::Config;
pub use error::{Error, Result};

#[cfg(feature = "server")]
pub use server::create_router;
