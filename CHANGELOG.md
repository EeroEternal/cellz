# Changelog

## 0.2.0 — 2026-09-04

Embed- and compile-size focused release. The public HTTP API is unchanged; the Rust crate surface and default install are not.

### Added
- Cargo features: `server` (default, Axum HTTP / SSE / WebSocket) and `s3` (`S3BlobStore` / `object_store`).
- In-process embed path: `cellz = { version = "0.2", default-features = false }` compiles per-cell SQLite, events, KV, checkpoints, and `LocalBlobStore` without Axum or S3.
- Each cell actor runs on a dedicated OS thread that owns a single `rusqlite` connection. `CellHandle` stays async (tokio mailbox).

### Changed
- Replaced `sqlx` with `rusqlite` (bundled SQLite). `Error::Database` now wraps `rusqlite::Error`.
- `CellDb` APIs are synchronous; all callers go through the actor mailbox.
- `CellActor::spawn` takes `(cell_id, db_path, initial_name)` and starts the thread itself.
- Trimmed `tokio` features (`full` → `rt-multi-thread` / `macros` / `fs` / `sync` / `time` / `io-util`).
- Binary `cellz` requires the `server` feature (still the crate default).

### Removed
- Unused direct `reqwest` dependency.
- S3 / R2 is no longer compiled into `cargo install cellz` unless `--features s3`.
  Rebuild with `cargo install cellz --features s3` (and `CELLZ_STORAGE_BACKEND=s3`) to restore remote snapshots.

### Migration from 0.1
- Embedders that only need cells: `default-features = false`.
- Embedders that need `create_router`: keep default features (or `features = ["server"]`).
- S3 / R2 users: add `features = ["s3"]` (daemon: `cargo install cellz --features s3`).
- Match on `cellz::Error::Database`: the inner type is `rusqlite::Error`, not `sqlx::Error`.
