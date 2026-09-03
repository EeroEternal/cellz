# cellz Architecture Specification

`cellz` is a lightweight, distributed Agent State & Scheduling Daemon inspired by Cloudflare Durable Objects and `denoland/celld`, built in 100% Rust.

It provides **isolated per-session SQLite storage**, **append-only event sourcing**, **single-writer lease guarantees**, and **pluggable blob snapshot persistence** (local disk or S3-compatible object storage).

---

## 1. System Overview

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

---

## 2. Core Pillars

### 2.1 Cell-as-an-Actor (Per-Session Isolation)
- **Granular Sharding**: Instead of a monolithic central database with heavy concurrent lock contention, each Agent session corresponds to an isolated **Cell**.
- **Dedicated SQLite**: Every Cell maintains its own SQLite database file (`data/cells/<cell_id>.db`) operating in `WAL` (Write-Ahead-Logging) mode with synchronous normal durability.
- **Actor Lifecycle**:
  - `Active`: Actor is loaded in memory, holding SQLite pool connections and serving queries with sub-millisecond response times.
  - `Idle / Evicted`: When inactive, the actor performs a WAL checkpoint, pushes a snapshot to `BlobStore`, releases its lease, and drops from memory.
  - `Auto-Recovery`: A query to an evicted cell automatically acquires the single-writer lease, downloads the latest snapshot from storage (if not present locally), and boots the actor on demand.

### 2.2 Event Sourcing & Message Materialization
- Every interaction in the Agent lifecycle is recorded as an immutable, strictly-sequenced `EventRecord`:
  - Lifecycle: `turn_start`, `turn_end`, `step_start`, `step_end`
  - Messages: `user_message`, `agent_message`, `system_message`
  - Tool execution: `tool_call`, `tool_result`
  - Human in the loop: `approval_requested`, `approval_responded`
  - State manipulation: `checkpoint`, `rewound`, `compaction`
- **Projection**: Message events automatically dual-write to the `messages` table for instant conversation retrieval without costly event replay overhead.

### 2.3 Distributed Single-Writer Guarantee & Durability
- `BlobStore` abstraction decouples state storage from the compute node:
  - **Single-Writer Lease**: Uses atomic conditional locks with TTL (default 60s) to prevent brain-split when multiple `cellz` nodes run in a cluster.
  - **Background Heartbeat**: A background worker in `cellz` automatically renews leases every 20 seconds for all currently active cells.
  - **Snapshotting**: Explicit WAL checkpoints flush SQLite dirty pages into clean database files, which are mirrored to local cold storage or S3 buckets.

### 2.4 Real-time Observability (SSE & WebSocket)
- In-memory `tokio::sync::broadcast` event bus inside each Cell Actor.
- Any event appended triggers zero-latency pushes to:
  - **Server-Sent Events (SSE)** via `GET /api/v1/cells/:id/stream`.
  - **WebSocket** via `GET /api/v1/cells/:id/ws` for full-duplex interactive sessions.

---

## 3. Schema Reference (Per-Cell SQLite)

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

-- Key-Value store for agent workspace / state
CREATE TABLE IF NOT EXISTS kv_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Historical checkpoints for rewind & branching
CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    sequence INTEGER NOT NULL,
    label TEXT NOT NULL,
    created_at TEXT NOT NULL
);
```
