use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::cell::CellManager;
use crate::model::event::AppendEventRequest;
use crate::model::state::{CreateCellRequest, CreateCheckpointRequest, RestoreCheckpointRequest, SetKVRequest};

#[derive(Clone)]
pub struct AppState {
    pub manager: Arc<CellManager>,
}

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub since: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_cells(State(state): State<AppState>) -> impl IntoResponse {
    match state.manager.list_cells().await {
        Ok(cells) => (StatusCode::OK, Json(json!({ "cells": cells }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn create_cell(
    State(state): State<AppState>,
    Json(payload): Json<CreateCellRequest>,
) -> impl IntoResponse {
    match state
        .manager
        .create_cell(payload.id, payload.name, payload.metadata)
        .await
    {
        Ok(handle) => match handle.get_meta().await {
            Ok(meta) => (StatusCode::CREATED, Json(json!({ "cell": meta }))).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_cell(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
) -> impl IntoResponse {
    match state.manager.get_or_activate(&cell_id).await {
        Ok(handle) => match handle.get_meta().await {
            Ok(meta) => (StatusCode::OK, Json(json!({ "cell": meta }))).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn append_event(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
    Json(req): Json<AppendEventRequest>,
) -> impl IntoResponse {
    match state.manager.get_or_activate(&cell_id).await {
        Ok(handle) => match handle.append_event(req.turn_id, req.event_type, req.payload).await {
            Ok(event) => (StatusCode::CREATED, Json(json!({ "event": event }))).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_events(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> impl IntoResponse {
    match state.manager.get_or_activate(&cell_id).await {
        Ok(handle) => match handle.get_events(query.since, query.limit).await {
            Ok(events) => (StatusCode::OK, Json(json!({ "events": events }))).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_messages(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
) -> impl IntoResponse {
    match state.manager.get_or_activate(&cell_id).await {
        Ok(handle) => match handle.get_messages().await {
            Ok(messages) => (StatusCode::OK, Json(json!({ "messages": messages }))).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_kv(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
) -> impl IntoResponse {
    match state.manager.get_or_activate(&cell_id).await {
        Ok(handle) => match handle.list_kv().await {
            Ok(kv) => (StatusCode::OK, Json(json!({ "kv": kv }))).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn set_kv(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
    Json(req): Json<SetKVRequest>,
) -> impl IntoResponse {
    match state.manager.get_or_activate(&cell_id).await {
        Ok(handle) => match handle.set_kv(req.key, req.value).await {
            Ok(()) => (StatusCode::OK, Json(json!({ "status": "ok" }))).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn create_checkpoint(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
    Json(req): Json<CreateCheckpointRequest>,
) -> impl IntoResponse {
    let label = req.label.unwrap_or_else(|| "manual".to_string());
    match state.manager.get_or_activate(&cell_id).await {
        Ok(handle) => match handle.create_checkpoint(label).await {
            Ok(checkpoint) => {
                (StatusCode::CREATED, Json(json!({ "checkpoint": checkpoint }))).into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn restore_checkpoint(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
    Json(req): Json<RestoreCheckpointRequest>,
) -> impl IntoResponse {
    match state.manager.get_or_activate(&cell_id).await {
        Ok(handle) => match handle.restore_checkpoint(req.checkpoint_id).await {
            Ok(sequence) => (
                StatusCode::OK,
                Json(json!({ "status": "restored", "target_sequence": sequence })),
            )
                .into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn backup_cell(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
) -> impl IntoResponse {
    match state.manager.backup_cell(&cell_id).await {
        Ok(()) => (StatusCode::OK, Json(json!({ "status": "backed_up" }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn evict_cell(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
) -> impl IntoResponse {
    match state.manager.evict_cell(&cell_id).await {
        Ok(()) => (StatusCode::OK, Json(json!({ "status": "evicted" }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
