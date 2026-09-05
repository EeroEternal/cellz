# cellz Architecture Specification

`cellz` is a **per-cell state and stream plane** for isolated actors, built in 100% Rust. Each **Cell** is an isolated SQLite unit with an append-only event log, optional message projection, generic KV, named checkpoints, a single-writer CAS lease, and blob snapshots (local disk or S3-compatible object storage).

It is not a worker runtime, not an agent framework, not a model gateway, and not a job scheduler. The actor runs outside; the cell stores and streams state. An AI agent is one kind of client.

---

## 1. Product Boundary

This section is the standing constraint for new features. If a change does not serve the state or stream plane of a cell, it does not belong in `cellz`. Do not add agent-specific schema, protocols, or runtimes.

### 1.1 In scope

| Surface | What exists in this crate |
| --- | --- |
| Isolation | One SQLite file per cell, dedicated OS thread, one `rusqlite` connection |
| Write model | Append-only `events` log; `event_type` is an opaque string |
| Projection | `user_message` / `agent_message` / `system_message` dual-write to `messages` |
| Workspace | Generic `kv_state` (key → JSON text). No typed Todos / Compactions tables |
| Time travel | Named checkpoints (sequence markers). Restore **truncates** `events` after that sequence and appends `rewound` |
| Durability | WAL checkpoint → snapshot bytes → `BlobStore` (local FS; `s3` feature for S3 / R2) |
| Single writer | CAS lease (`PutMode::Create` / `If-Match` on S3; atomic file create locally) + TTL heartbeat |
| Stream | In-process broadcast bus; HTTP SSE / WebSocket with `Last-Event-ID` / `?since=` replay |
| Delivery | One product, two packages: default = Axum daemon; `default-features = false` = in-process core |

### 1.2 Out of scope

| Not this crate | Goes elsewhere |
| --- | --- |
| Execute user code, JS/TS Workers, V8/isolates | The client process, or a runtime like `celld` |
| LLM calls, provider routing, API keys | Model gateway |
| Agent loops, tool runtimes, prompt assembly, workflows | The actor that owns the cell |
| Job queue, cron, multi-cell scheduling | Orchestrator above cells |
| Cell fork / branch (copy-on-write session) | Not implemented; restore is in-place truncate |
| Roll back of `messages` / `kv_state` on restore | Restore currently truncates **events only** |
| Admin / console UI | Removed; not part of the crate |
| Cloudflare Durable Objects API compatibility | Explicit non-goal |

### 1.3 Convention vs enforcement

Clients **may** use any event names (`turn_start`, `tool_call`, `order_placed`, …). The server **does not** validate them, enumerate them, or interpret payloads except:

- message projection for the three `*_message` types (convenience for chat-shaped logs)
- appending `rewound` after a checkpoint restore

`Todos` and `Compactions` in older copy were examples of KV keys, not first-class stores. Do not grow a typed agent protocol inside the cell.

### 1.4 vs Durable Objects / `celld`

Shared idea: isolate one unit of state, single writer, hibernate to blob storage.

Hard stop: `celld` and Durable Objects **run user code** inside the unit. `cellz` does not. Any client talks HTTP / SSE / WebSocket (or embeds `CellManager`). There is no isolate, no worker script, no DO-compatible API, no agent schema.

---

## 2. System Overview

```text
               ┌────────────────────────────────────────────────────────┐
               │              Client / actor (any process)              │
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
                      │  │  - One OS thread + rusqlite conn  │  │
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

---

## 3. Core Pillars

### 3.1 Cell-as-an-Actor (Per-Session Isolation)
- **Granular Sharding**: Instead of a monolithic central database with heavy concurrent lock contention, each isolated **Cell** is one unit of state. Typical mapping is one client actor (session, workflow instance, …) per cell.
- **Dedicated SQLite**: Every Cell maintains its own SQLite database file (`data/cells/<cell_id>.db`) operating in `WAL` (Write-Ahead-Logging) mode with synchronous normal durability.
- **Actor Lifecycle**:
  - `Active`: Actor runs on a dedicated OS thread owning a single `rusqlite` connection, serving queries with sub-millisecond local commits.
  - `Idle / Evicted`: When inactive, the actor performs a WAL checkpoint, pushes a snapshot to `BlobStore`, releases its lease, and drops from memory.
  - `Auto-Recovery`: A query to an evicted cell automatically acquires the single-writer lease, downloads the latest snapshot from storage (if not present locally), and boots the actor on demand.

### 3.2 Event Sourcing & Message Materialization
- Every write that mutates a cell's history is an immutable, strictly-sequenced `EventRecord`. `event_type` is an opaque string; the crate does not ship an event-type enum.
- **Projection** (the only payload interpretation): `user_message`, `agent_message`, and `system_message` dual-write to the `messages` table so chat-shaped retrieval does not replay the log. This is a convenience, not an agent protocol.
- **Restore**: `restore_checkpoint` deletes `events` with `sequence > checkpoint.sequence` and appends a `rewound` event. It does not fork a branch and does not rebuild `messages` or `kv_state`.
- All other event names are client conventions. See [Product Boundary](#1-product-boundary).

### 3.3 Distributed Single-Writer Guarantee & Durability
- `BlobStore` abstraction decouples state storage from the compute node:
  - **Single-Writer Lease**: Uses atomic conditional locks with TTL (default 60s) to prevent brain-split when multiple `cellz` nodes run in a cluster.
  - **Background Heartbeat**: A background worker in `cellz` automatically renews leases every 20 seconds for all currently active cells.
  - **Snapshotting**: Explicit WAL checkpoints flush SQLite dirty pages into clean database files, which are mirrored to local cold storage or S3 buckets.

### 3.4 Real-time Observability (SSE & WebSocket)
- In-memory `tokio::sync::broadcast` event bus inside each Cell Actor.
- Any event appended triggers zero-latency pushes to:
  - **Server-Sent Events (SSE)** via `GET /api/v1/cells/:id/stream`.
  - **WebSocket** via `GET /api/v1/cells/:id/ws` for full-duplex interactive sessions.

### 3.5 Cargo features

The crate is split so in-process embedders (e.g. gitcell) do not compile the HTTP or S3 stacks:

| Feature | Default | Surface |
| --- | --- | --- |
| *(always on)* | — | `CellDb` / `CellActor` / `CellManager`, events, messages, KV, checkpoints, `LocalBlobStore` |
| `server` | yes | Axum REST + SSE + WebSocket (`create_router`) |
| `s3` | no | `S3BlobStore` (`object_store` aws) |

Event sourcing is the cell write model and is not optional. `default-features = false` is the light embed path.

The core keeps a tokio mailbox so `CellHandle` stays async. SQL does not run on the tokio worker: the actor thread owns the `rusqlite` connection and processes mailbox messages with `blocking_recv`. Do not disable tokio in the core.

S3 / R2 is compile-time optional. `CELLZ_STORAGE_BACKEND=s3` without `--features s3` is a configuration error, not a silent fallback to local disk.

---

## 4. Schema Reference (Per-Cell SQLite)

Each `.sqlite` file contains 5 core tables:

```sql
-- Cell metadata and current event sequence
CREATE TABLE IF NOT EXISTS cell_meta (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    event_sequence INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}'
);

-- Append-only event store
CREATE TABLE IF NOT EXISTS events (
    sequence INTEGER PRIMARY KEY,
    id TEXT NOT NULL UNIQUE,
    turn_id TEXT,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Materialized messages cache
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    name TEXT,
    tool_call_id TEXT,
    created_at TEXT NOT NULL
);

-- Key-Value store for cell workspace / state
CREATE TABLE IF NOT EXISTS kv_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Named sequence markers for in-place event-log rewind
CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    sequence INTEGER NOT NULL,
    label TEXT NOT NULL,
    created_at TEXT NOT NULL
);
```
