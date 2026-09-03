# Architecture & Domain Boundaries

This document defines the system architectural model, core component boundaries, and data flow principles.

## 1. Architecture Overview

```text
[ Client / Admin Console ]
           │
           ▼
[ Axum HTTP / WS Transport Layer ]
           │
           ▼
[ Domain Layer / Dispatcher / Service Logic ]
           │
           ▼
[ Storage Layer (Sqlx / Cache) ]
```

## 2. Layering & Invocation Rules

1. **Transport Layer (`src/server.rs`)**:
   - Handles route mounting, middleware attachment (CORS, Trace, Auth), and HTTP serialization.
   - Strictly prohibited from containing database queries and core business logic.
2. **Domain Service Layer**:
   - Encapsulates business entities, state transitions, and scheduling algorithms.
3. **Storage Layer**:
   - Uses `sqlx` for type-safe asynchronous SQL interactions.
   - Database schema changes are strictly driven by `migrations/NNN_*.sql`.

## 3. Plugin-First Principle

All custom business adaptations (such as dynamic headers, auth decoration, vendor protocol adaptors, and data masking) must be implemented as modular middleware or plugins, never hardcoded into the core data plane.
