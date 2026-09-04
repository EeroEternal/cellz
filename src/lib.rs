//! SQLite-per-session, event-sourced state server for AI agents.
//!
//! `cellz` is a language-agnostic state and stream plane. Each session is a
//! single-writer SQLite cell with event sourcing, message projection, KV state,
//! checkpoints, and lossless SSE / WebSocket replay. Durable leases and
//! snapshots can live on the local filesystem or on S3 / Cloudflare R2.
//!
//! # Install the daemon
//!
//! ```text
//! cargo install cellz
//! cellz
//! ```
//!
//! # Embed as a library
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use cellz::cell::CellManager;
//! use cellz::config::Config;
//! use cellz::server::create_router;
//! use cellz::storage::LocalBlobStore;
//!
//! let config = Config::default();
//! let storage = Arc::new(LocalBlobStore::new(&config.storage_dir));
//! let manager = Arc::new(CellManager::new(
//!     &config.data_dir,
//!     storage,
//!     config.lease_ttl_secs,
//! ));
//! let _app = create_router(manager);
//! ```

pub mod api;
pub mod cell;
pub mod config;
pub mod error;
pub mod model;
pub mod server;
pub mod storage;

pub use config::Config;
pub use error::{Error, Result};
pub use server::create_router;
