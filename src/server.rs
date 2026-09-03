use std::sync::Arc;

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::api::handlers::{
    append_event, backup_cell, create_cell, create_checkpoint, evict_cell, get_cell, get_events,
    get_messages, list_cells, list_kv, restore_checkpoint, set_kv, AppState,
};
use crate::api::ws::{sse_events_stream, ws_cell_handler};
use crate::cell::CellManager;

pub fn create_router(manager: Arc<CellManager>) -> Router {
    let state = AppState { manager };

    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/cells", get(list_cells).post(create_cell))
        .route("/api/v1/cells/{id}", get(get_cell))
        .route("/api/v1/cells/{id}/events", get(get_events).post(append_event))
        .route("/api/v1/cells/{id}/messages", get(get_messages))
        .route("/api/v1/cells/{id}/kv", get(list_kv).post(set_kv))
        .route("/api/v1/cells/{id}/checkpoints", post(create_checkpoint))
        .route("/api/v1/cells/{id}/restore", post(restore_checkpoint))
        .route("/api/v1/cells/{id}/backup", post(backup_cell))
        .route("/api/v1/cells/{id}/evict", post(evict_cell))
        .route("/api/v1/cells/{id}/stream", get(sse_events_stream))
        .route("/api/v1/cells/{id}/ws", get(ws_cell_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "cellz",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
