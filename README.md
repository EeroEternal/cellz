# cellz

> **SQLite-per-cell state and stream plane**  
> Language-agnostic event log, message projection, and SSE/WebSocket replay. Atomic CAS leasing, local or S3 / R2 snapshots, sub-millisecond local commits. Does not execute user code. Built in 100% Rust.

[![CI](https://github.com/EeroEternal/cellz/actions/workflows/ci.yml/badge.svg)](https://github.com/EeroEternal/cellz/actions)
[![crates.io](https://img.shields.io/crates/v/cellz.svg)](https://crates.io/crates/cellz)
[![docs.rs](https://docs.rs/cellz/badge.svg)](https://docs.rs/cellz)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](Cargo.toml)

---

## Positioning

**cellz is a per-cell state and stream plane for isolated actors.** A cell does not know whether its client is an AI agent, a workflow, a UI, or another service. Compute stays outside. The cell stores history, optionally projects chat-shaped events, and streams it back. Full boundary: [Architecture §1](docs/architecture.md#1-product-boundary).

| In this crate | Not this crate |
| --- | --- |
| Per-cell SQLite, event log, message projection, generic KV | User-code isolates / JS Workers (`celld`, Durable Objects) |
| Named checkpoints; in-place event-log rewind | Cell fork / branch; application logic |
| CAS single-writer lease + blob snapshots | Job scheduler, cron, multi-cell orchestration |
| REST / SSE / WebSocket, or in-process `CellManager` | LLM gateway, API keys, Admin UI |

- **No user code in the cell.** Any client in Rust, Python, TypeScript, Go, … attaches over HTTP / SSE / WebSocket, or embeds the crate with `default-features = false`.
- **Event types are client strings.** Only `user_message` / `agent_message` / `system_message` are projected into `messages` (a convenience for chat-shaped logs, not an agent schema). KV is generic — there is no Todos table.
- **Atomic single-writer CAS leases.** S3 `PutMode::Create` / `If-Match` ETag, or atomic OS filesystem locks. Prevents split-brain across nodes.
- **Dedicated actor thread.** One OS thread and one `rusqlite` connection per cell. Snapshots (`wal_checkpoint(TRUNCATE)`) go through the mailbox so WAL writes and backup reads cannot race. `CellHandle` stays async.
- **Lossless reconnect.** SSE / WebSocket honor `Last-Event-ID` and `?since=` — replay missed events, then live broadcast.

[denoland/celld](https://github.com/denoland/celld) self-hosts Durable Objects by running JS/TS Workers. cellz takes isolation and single-writer hibernation from that design and **stops before the isolate**. An AI agent is one kind of client, not the product.

---

## 🏛️ Architecture

```text
               ┌────────────────────────────────────────────────────────┐
               │              Client / actor (any process)              │
               └──────────────────────────┬─────────────────────────────┘
                                          │ HTTP / SSE (Last-Event-ID) / WS
                                          ▼
                      ┌────────────────────────────────────────┐
                      │                 cellz                  │
                      │      (Axum REST & Real-time Mesh)      │
                      │                                        │
                      │  ┌──────────────────────────────────┐  │
                      │  │       Session Cell (Actor)       │  │
                      │  │  - Monotonic Event Sourcing       │  │
                      │  │  - Materialized Message Projection│  │
                      │  │  - Key-Value State Machine        │  │
                      │  │  - Dedicated SQLite (WAL Mode)    │  │
                      │  │  - Mailbox-Serialized Snapshots   │  │
                      │  └────────────────┬─────────────────┘  │
                      └───────────────────┼────────────────────┘
                                          │
                     ┌────────────────────┴────────────────────┐
                     ▼                                         ▼
        ┌─────────────────────────┐               ┌─────────────────────────┐
        │  Local Cell SQLite DBs  │               │   Blob Storage Engine   │
        │   `data/cells/{id}.db`  │               │ (Local FS / S3 / R2)    │
        │  (Microsecond Latency)  │               │ (CAS Leases & Snapshots)│
        └─────────────────────────┘               └─────────────────────────┘
```

For in-depth architectural details, refer to [Architecture Specification](docs/architecture.md).

---

## 🚀 Quick Start

### Install from crates.io

```bash
cargo install cellz
cellz
```

Embed the Axum router in your own process:

```toml
# HTTP daemon API (default)
cellz = "0.2"

# In-process core only — no Axum / object_store (gitcell, embed)
cellz = { version = "0.2", default-features = false }

# + S3 / Cloudflare R2 snapshots
cellz = { version = "0.2", features = ["s3"] }
```

```rust
use std::sync::Arc;
use cellz::cell::CellManager;
use cellz::config::Config;
use cellz::storage::LocalBlobStore;

let config = Config::default();
let storage = Arc::new(LocalBlobStore::new(&config.storage_dir));
let manager = CellManager::new(
    &config.data_dir,
    storage,
    config.lease_ttl_secs,
);
```

With the default `server` feature, `cellz::create_router(manager)` exposes the HTTP / SSE / WebSocket API.

### Cargo features

| Feature | Default | What it enables |
| :--- | :---: | :--- |
| *(core, always on)* | — | Per-cell SQLite, event sourcing, messages, KV, checkpoints, `LocalBlobStore` |
| `server` | yes | Axum HTTP + SSE + WebSocket daemon (`create_router`) |
| `s3` | no | `S3BlobStore` via `object_store` (S3 / Cloudflare R2) |

Event sourcing is the write model of a cell and is not feature-gated.

### 1. Run the Daemon from source

```bash
cargo install cellz                 # local filesystem storage
cargo install cellz --features s3   # + S3 / Cloudflare R2

# Local filesystem storage (from source)
cargo run --release

# Or with Cloudflare R2 / S3
export CELLZ_STORAGE_BACKEND="s3"
export CELLZ_S3_ENDPOINT="https://<account_id>.r2.cloudflarestorage.com"
export CELLZ_S3_BUCKET="zene-cells"
export CELLZ_S3_ACCESS_KEY_ID="<your_key>"
export CELLZ_S3_SECRET_ACCESS_KEY="<your_secret>"
cargo run --release --features s3
```

### 2. Configuration Options

`cellz` is configured via environment variables:

| Environment Variable | Default Value | Description |
| :--- | :--- | :--- |
| `CELLZ_HOST` | `0.0.0.0` | Bind IP address |
| `CELLZ_PORT` | `8080` | Listen port |
| `CELLZ_DATA_DIR` | `./data/cells` | Local SQLite databases storage path |
| `CELLZ_STORAGE_DIR` | `./data/storage` | Snapshot backup & lease directory (local backend) |
| `CELLZ_LEASE_TTL` | `60` | Lease lock expiry duration in seconds |
| `CELLZ_STORAGE_BACKEND` | `local` | Storage backend (`local` or `s3`) |
| `CELLZ_S3_ENDPOINT` | None | S3 / Cloudflare R2 endpoint URL |
| `CELLZ_S3_BUCKET` | None | S3 / Cloudflare R2 bucket name |
| `CELLZ_S3_ACCESS_KEY_ID`| None | S3 Access Key ID |
| `CELLZ_S3_SECRET_ACCESS_KEY` | None | S3 Secret Access Key |
| `CELLZ_S3_REGION` | `auto` | S3 Region (`auto` for Cloudflare R2) |

### 3. Create a Session & Append Events

```bash
# 1. Create a new Cell
curl -X POST http://localhost:8080/api/v1/cells \
  -H "Content-Type: application/json" \
  -d '{"id": "cell-007", "name": "demo"}'

# 2. Append a user message
curl -X POST http://localhost:8080/api/v1/cells/cell-007/events \
  -H "Content-Type: application/json" \
  -d '{
    "turn_id": "turn-1",
    "event_type": "user_message",
    "payload": { "content": "Add unit tests for cell/db.rs" }
  }'

# 3. Retrieve projected messages
curl http://localhost:8080/api/v1/cells/cell-007/messages

# 4. Subscribe to real-time events via SSE
curl -N -H "Accept: text/event-stream" http://localhost:8080/api/v1/cells/cell-007/stream
```

Full API documentation and request/response payloads are available in [API Reference](docs/api.md).

---

## 🧪 Testing & Quality Gates

Run full test suite:

```bash
cargo test --workspace --all-features
cargo check --no-default-features
```

Run linter:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

---

## 📚 Documentation Index

- [Architecture Specification](docs/architecture.md): Product boundary, per-cell SQLite, dedicated actor thread, cargo features, and lease management.
- [API Reference](docs/api.md): Complete REST, SSE, and WebSocket endpoints specification.
- [Changelog](CHANGELOG.md): Released crate versions.

---

## 📄 License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
