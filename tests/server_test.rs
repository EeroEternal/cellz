use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use cellz::cell::CellManager;
use cellz::server::create_router;
use cellz::storage::LocalBlobStore;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn setup_test_app() -> (axum::Router, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let cells_dir = temp_dir.path().join("cells");
    let storage_dir = temp_dir.path().join("storage");

    let storage = Arc::new(LocalBlobStore::new(&storage_dir));
    let manager = Arc::new(CellManager::new(&cells_dir, storage, 60));
    let app = create_router(manager);
    (app, temp_dir)
}

#[tokio::test]
async fn test_health_check() {
    let (app, _dir) = setup_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "cellz");
}

#[tokio::test]
async fn test_cell_lifecycle_and_state_machine() {
    let (app, _dir) = setup_test_app().await;

    // 1. Create a new Cell
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cells")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "id": "agent-session-001",
                        "name": "Test Coding Agent",
                        "metadata": { "repo": "paratensor/zene" }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    // 2. Append a user message event
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cells/agent-session-001/events")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "turn_id": "turn-1",
                        "event_type": "user_message",
                        "payload": {
                            "content": "Please implement feature X"
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    // 3. Append tool call and result
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cells/agent-session-001/events")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "turn_id": "turn-1",
                        "event_type": "tool_call",
                        "payload": {
                            "tool": "view_file",
                            "args": { "path": "src/lib.rs" }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // 4. Append assistant message
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cells/agent-session-001/events")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "turn_id": "turn-1",
                        "event_type": "agent_message",
                        "payload": {
                            "content": "I have inspected the file and completed the change."
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // 5. Query projected messages
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/cells/agent-session-001/messages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let messages = json["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[1]["role"], "assistant");

    // 6. Test KV state
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cells/agent-session-001/kv")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "key": "current_goal",
                        "value": { "goal": "clean architecture", "done": false }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/cells/agent-session-001/kv")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["kv"]["current_goal"]["done"], false);

    // 7. Checkpoint creation
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cells/agent-session-001/checkpoints")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "label": "v1.0" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let cp_id = json["checkpoint"]["id"].as_str().unwrap().to_string();

    // 8. Test backup to BlobStore
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cells/agent-session-001/backup")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 9. Test restore checkpoint
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cells/agent-session-001/restore")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "checkpoint_id": cp_id }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 10. Evict cell from memory
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cells/agent-session-001/evict")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 11. Re-query cell after eviction (should auto-activate from disk/storage)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/cells/agent-session-001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 12. Test batch event append
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cells/agent-session-001/events/batch")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "events": [
                            { "event_type": "turn_start", "payload": { "turn": 2 } },
                            { "event_type": "user_message", "payload": { "content": "How are you?" } },
                            { "event_type": "agent_message", "payload": { "content": "I am ready!" } }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["events"].as_array().unwrap().len(), 3);

    // 13. Test export cell
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/cells/agent-session-001/export")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["export"]["events"].as_array().unwrap().len() >= 4);
    assert_eq!(json["export"]["meta"]["id"], "agent-session-001");
}
