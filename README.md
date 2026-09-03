# cellz

> **Lightweight, Distributed Agent State & Scheduling Daemon**  
> Inspired by Cloudflare Durable Objects and `denoland/celld`, built in 100% Rust.

[![CI](https://github.com/EeroEternal/cellz/actions/workflows/ci.yml/badge.svg)](https://github.com/EeroEternal/cellz/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](Cargo.toml)

---

## ⚡ Why cellz?

Modern AI Coding Agents (such as [Zene](https://github.com/ParaTensor/zene)) face critical state management bottlenecks when scaling from local CLI to distributed multi-user Cloud environments:

1. **Lock Contention in Shared DBs**: Running hundreds of concurrent agent loops with frequent event dual-writes against a single centralized SQLite or Postgres leads to severe write contention.
2. **Context Growth & Serialization Overhead**: Storing full session histories as monolithic JSON files degrades latency as conversation events accumulate.
3. **Multi-Node Failover & Brain-Split**: Moving an ongoing Agent turn across workers requires complex lease coordination and distributed snapshotting.

**`cellz` solves this with the "Cell" (Durable Object) architecture:**
- **Per-Cell Isolated SQLite**: Each agent session is a dedicated Cell backed by its own private SQLite in `WAL` mode. Sessions operate independently with sub-millisecond local commits.
- **Event Sourcing + Projection**: Monotonic append-only event logs coupled with instant materialized chat views.
- **S3 / Blob Storage Durability**: Snapshots and distributed single-writer leases managed seamlessly via local filesystem or S3-compatible buckets.
- **Native Real-Time Mesh**: Built-in Server-Sent Events (SSE) and WebSocket streaming for token generation, tool execution logs, and human-in-the-loop approvals.

---

## 🏛️ Architecture

```text
               ┌────────────────────────────────────────────────────────┐
               │              Client / Web UI / Coding Agent            │
               └──────────────────────────┬─────────────────────────────┘
                                          │ HTTP / SSE / WebSocket
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
                      │  └────────────────┬─────────────────┘  │
                      └───────────────────┼────────────────────┘
                                          │
                     ┌────────────────────┴────────────────────┐
                     ▼                                         ▼
        ┌─────────────────────────┐               ┌─────────────────────────┐
        │  Local Cell SQLite DBs  │               │   Blob Storage Engine   │
        │   `data/cells/{id}.db`  │               │ (Local FS / S3 Storage) │
        │  (Microsecond Latency)  │               │  (Snapshots & Leases)   │
        └─────────────────────────┘               └─────────────────────────┘
```

For in-depth architectural details, refer to [Architecture Specification](docs/architecture.md).

---

## 🚀 Quick Start

### 1. Run the Daemon

```bash
# Build and run with default settings (port 8080)
cargo run --release
```

### 2. Configuration Options

`cellz` is configured via environment variables:

| Environment Variable | Default Value | Description |
| :--- | :--- | :--- |
| `CELLZ_HOST` | `0.0.0.0` | Bind IP address |
| `CELLZ_PORT` | `8080` | Listen port |
| `CELLZ_DATA_DIR` | `./data/cells` | Local SQLite databases storage path |
| `CELLZ_STORAGE_DIR` | `./data/storage` | Snapshot backup & lease directory |
| `CELLZ_LEASE_TTL` | `60` | Lease lock expiry duration in seconds |

### 3. Create a Session & Append Events

```bash
# 1. Create a new Cell
curl -X POST http://localhost:8080/api/v1/cells \
  -H "Content-Type: application/json" \
  -d '{"id": "agent-007", "name": "Refactor Agent"}'

# 2. Append a user message
curl -X POST http://localhost:8080/api/v1/cells/agent-007/events \
  -H "Content-Type: application/json" \
  -d '{
    "turn_id": "turn-1",
    "event_type": "user_message",
    "payload": { "content": "Add unit tests for cell/db.rs" }
  }'

# 3. Retrieve projected messages
curl http://localhost:8080/api/v1/cells/agent-007/messages

# 4. Subscribe to real-time events via SSE
curl -N -H "Accept: text/event-stream" http://localhost:8080/api/v1/cells/agent-007/stream
```

Full API documentation and request/response payloads are available in [API Reference](docs/api.md).

---

## 🧪 Testing & Quality Gates

Run full test suite:

```bash
cargo test --workspace
```

Run linter:

```bash
cargo clippy --all-targets -- -D warnings
```

---

## 📚 Documentation Index

- [Architecture Specification](docs/architecture.md): Deep dive into Per-Cell SQLite, Actor lifecycle, and lease management.
- [API Reference](docs/api.md): Complete REST, SSE, and WebSocket endpoints specification.
- [Admin UI Kit](admin/README.md): Admin console UI framework and component catalog.

---

## 📄 License

Apache-2.0 / MIT
