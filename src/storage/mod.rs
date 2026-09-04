//! Local filesystem blob storage with CAS leases.
//!
//! Enable the `s3` feature for S3 / Cloudflare R2.

pub mod blob;

pub use blob::{BlobStore, LocalBlobStore};

#[cfg(feature = "s3")]
pub use blob::S3BlobStore;
