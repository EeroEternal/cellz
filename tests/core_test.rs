use std::sync::Arc;

use cellz::cell::CellManager;
use cellz::storage::LocalBlobStore;
use serde_json::json;

#[tokio::test]
async fn core_cell_event_and_kv_without_server_feature() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Arc::new(LocalBlobStore::new(temp.path().join("storage")));
    let manager = CellManager::new(temp.path().join("cells"), storage, 60);

    let handle = manager
        .create_cell(Some("core-1".into()), Some("core".into()), None)
        .await
        .unwrap();

    let event = handle
        .append_event(
            Some("turn-1".into()),
            "user_message",
            json!({ "content": "hello" }),
        )
        .await
        .unwrap();
    assert_eq!(event.sequence, 1);

    handle.set_kv("todo", json!(["a"])).await.unwrap();
    let value = handle.get_kv("todo").await.unwrap();
    assert_eq!(value, Some(json!(["a"])));

    let messages = handle.get_messages().await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, "user");
    assert_eq!(messages[0].content, "hello");
}
