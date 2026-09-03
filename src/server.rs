use axum::{Json, Router, routing::get};
use serde_json::{Value, json};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/ping", get(ping))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "console-kit"
    }))
}

async fn ping() -> Json<Value> {
    Json(json!({
        "message": "pong"
    }))
}
