# cellz API Reference

All HTTP API endpoints are prefixed with `/api/v1` (except `/health`).

---

## Health Check

### `GET /health`
Returns the operational status of the `cellz` daemon.

**Response:**
```json
{
  "status": "ok",
  "service": "cellz",
  "version": "0.1.0"
}
```

---

## Cell Management

### `POST /api/v1/cells`
Create and immediately activate a new Cell.

**Request Body:**
```json
{
  "id": "optional-custom-cell-id",
  "name": "My Agent Session",
  "metadata": {
    "project": "zene",
    "model": "claude-3-7-sonnet"
  }
}
```

**Response (201 Created):**
```json
{
  "cell": {
    "id": "optional-custom-cell-id",
    "name": "My Agent Session",
    "status": "active",
    "event_sequence": 0,
    "created_at": "2026-09-03T18:00:00Z",
    "updated_at": "2026-09-03T18:00:00Z",
    "metadata": {}
  }
}
```

---

### `GET /api/v1/cells`
List all cells on the current node (active in-memory and persisted on disk).

**Response (200 OK):**
```json
{
  "cells": [
    {
      "id": "session-1",
      "name": "Agent 1",
      "status": "active",
      "event_sequence": 42,
      "created_at": "2026-09-03T18:00:00Z",
      "updated_at": "2026-09-03T18:05:00Z",
      "metadata": {}
    }
  ]
}
```

---

### `GET /api/v1/cells/{id}`
Retrieve metadata and status of a specific Cell. If the Cell is inactive, it will automatically be loaded from local disk or blob storage.

**Response (200 OK):**
```json
{
  "cell": {
    "id": "session-1",
    "name": "Agent 1",
    "status": "active",
    "event_sequence": 42,
    "created_at": "2026-09-03T18:00:00Z",
    "updated_at": "2026-09-03T18:05:00Z",
    "metadata": {}
  }
}
```

---

## Event Sourcing & Messaging

### `POST /api/v1/cells/{id}/events`
Append a new event to the Cell's append-only log. If the event is a message (`user_message`, `agent_message`, `system_message`), it is automatically projected to the `messages` table.

**Request Body:**
```json
{
  "turn_id": "turn-101",
  "event_type": "user_message",
  "payload": {
    "content": "Refactor the authentication module to use Argon2"
  }
}
```

**Response (201 Created):**
```json
{
  "event": {
    "sequence": 1,
    "id": "f51bb123-...",
    "cell_id": "session-1",
    "turn_id": "turn-101",
    "event_type": "user_message",
    "payload": {
      "content": "Refactor the authentication module to use Argon2"
    },
    "created_at": "2026-09-03T18:06:00Z"
  }
}
```

---

### `GET /api/v1/cells/{id}/events`
Query event history with optional pagination parameters.

**Query Parameters:**
- `since`: Integer (sequence offset). Only events with `sequence > since` are returned.
- `limit`: Integer (maximum events to return, default 500).

**Response (200 OK):**
```json
{
  "events": [
    {
      "sequence": 1,
      "id": "f51bb123-...",
      "cell_id": "session-1",
      "turn_id": "turn-101",
      "event_type": "user_message",
      "payload": { "content": "Hello" },
      "created_at": "2026-09-03T18:06:00Z"
    }
  ]
}
```

---

### `GET /api/v1/cells/{id}/messages`
Retrieve all materialized chat messages in chronological order.

**Response (200 OK):**
```json
{
  "messages": [
    {
      "id": "f51bb123-...",
      "role": "user",
      "content": "Refactor the authentication module to use Argon2",
      "created_at": "2026-09-03T18:06:00Z"
    },
    {
      "id": "a92cc456-...",
      "role": "assistant",
      "content": "I will inspect the existing auth files first.",
      "created_at": "2026-09-03T18:06:02Z"
    }
  ]
}
```

---

## Agent State (KV Store)

### `GET /api/v1/cells/{id}/kv`
Fetch all key-value entries stored in the Cell.

**Response (200 OK):**
```json
{
  "kv": {
    "current_plan": ["step 1", "step 2"],
    "approval_pending": false
  }
}
```

---

### `POST /api/v1/cells/{id}/kv`
Set or update a specific key-value entry.

**Request Body:**
```json
{
  "key": "current_plan",
  "value": {
    "steps": ["audit", "refactor", "verify"],
    "completed": 1
  }
}
```

**Response (200 OK):**
```json
{
  "status": "ok"
}
```

---

## Checkpoints & Rewind

### `POST /api/v1/cells/{id}/checkpoints`
Create a named checkpoint snapshot at the current event sequence.

**Request Body:**
```json
{
  "label": "before_destructive_refactor"
}
```

**Response (201 Created):**
```json
{
  "checkpoint": {
    "id": "cp-8812-...",
    "cell_id": "session-1",
    "sequence": 14,
    "label": "before_destructive_refactor",
    "created_at": "2026-09-03T18:10:00Z"
  }
}
```

---

### `POST /api/v1/cells/{id}/restore`
Roll back the Cell state to the event sequence represented by a checkpoint.

**Request Body:**
```json
{
  "checkpoint_id": "cp-8812-..."
}
```

**Response (200 OK):**
```json
{
  "status": "restored",
  "target_sequence": 14
}
```

---

## Storage & Lifecycle

### `POST /api/v1/cells/{id}/backup`
Flush WAL checkpoint and upload the SQLite database file to `BlobStore`.

**Response (200 OK):**
```json
{
  "status": "backed_up"
}
```

---

### `POST /api/v1/cells/{id}/evict`
Flush WAL, push backup to `BlobStore`, terminate in-memory actor, and release distributed lease.

**Response (200 OK):**
```json
{
  "status": "evicted"
}
```

---

## Real-Time Streams

### `GET /api/v1/cells/{id}/stream` (Server-Sent Events)
Subscribe to live event stream. Every newly appended event (`EventRecord`) is pushed as an SSE `data` event.

**Header:** `Accept: text/event-stream`

**Stream Output Example:**
```text
data: {"sequence":15,"id":"...","cell_id":"session-1","turn_id":"turn-102","event_type":"tool_call","payload":{"tool":"run_command"},"created_at":"2026-09-03T18:15:00Z"}
```

---

### `GET /api/v1/cells/{id}/ws` (WebSocket)
Establish a full-duplex interactive WebSocket connection.

- **Server pushes**:
  ```json
  {
    "type": "event",
    "data": { ... } // EventRecord
  }
  ```
- **Client commands**:
  ```json
  {
    "action": "append_event",
    "turn_id": "turn-103",
    "event_type": "user_message",
    "payload": { "content": "continue" }
  }
  ```
  ```json
  {
    "action": "ping"
  }
  ```
