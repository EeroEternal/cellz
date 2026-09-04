//! Local filesystem and S3/R2 blob storage with CAS leases.

pub mod blob;

pub use blob::{BlobStore, LocalBlobStore, S3BlobStore};
