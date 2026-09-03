# cellz Architecture Design

`cellz` is a lightweight, distributed Agent State & Scheduling Daemon inspired by Cloudflare Durable Objects and `denoland/celld`, built in 100% Rust.

## Core Pillars

1. **Cell-as-an-Actor (Per-Session Isolation)**
   - Every Agent session is mapped to a dedicated **Cell**.
   - Each Cell maintains its own private SQLite database in WAL mode (`data/cells/<cell_id>.db`).
   - Zero lock contention between different agent sessions.

2. **Event Sourcing & Projection**
   - Every interaction (user prompt, LLM chunk, tool call, tool result, approval request, compaction, fork) is recorded as an immutable, sequenced event.
   - Live state (messages, KV state, active turns) is projected in memory and queryable via SQL.

3. **Pluggable Durability (Blob Storage)**
   - `BlobStore` abstraction: Local Filesystem (zero external dependency) and S3-compatible object storage (AWS S3, MinIO, Garage, Cloudflare R2).
   - Automated checkpoint snapshotting and single-writer lease arbitration.

4. **Real-time Observability & Control**
   - Native Server-Sent Events (SSE) and WebSocket support for token streaming and real-time tool execution tracking.
   - REST API for lifecycle management (create, query, event append, checkpoint, rewind, evict).
