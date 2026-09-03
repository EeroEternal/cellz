use std::convert::Infallible;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{debug, info};

use crate::api::handlers::AppState;

pub async fn sse_events_stream(
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
) -> impl IntoResponse {
    let handle = match state.manager.get_or_activate(&cell_id).await {
        Ok(h) => h,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let rx = handle.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| async move {
        match res {
            Ok(event) => match serde_json::to_string(&event) {
                Ok(data) => Some(Ok::<Event, Infallible>(Event::default().data(data))),
                Err(_) => None,
            },
            Err(_) => None,
        }
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keep-alive"))
        .into_response()
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum WsClientMessage {
    Ping,
    AppendEvent {
        turn_id: Option<String>,
        event_type: String,
        payload: serde_json::Value,
    },
    GetMeta,
}

pub async fn ws_cell_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(cell_id): Path<String>,
) -> impl IntoResponse {
    match state.manager.get_or_activate(&cell_id).await {
        Ok(handle) => ws.on_upgrade(move |socket| handle_socket(socket, handle)),
        Err(e) => (
            StatusCode::NOT_FOUND,
            axum::Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn handle_socket(socket: WebSocket, handle: crate::cell::CellHandle) {
    let (mut sender, mut receiver) = socket.split();
    let mut event_rx = handle.subscribe();

    info!("WebSocket connected for cell '{}'", handle.cell_id);

    // Task 1: Forward broadcasted events to WebSocket client
    let cell_id_clone = handle.cell_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            if let Ok(msg_str) = serde_json::to_string(&json!({ "type": "event", "data": event })) {
                if sender.send(Message::Text(msg_str.into())).await.is_err() {
                    break;
                }
            }
        }
        debug!("WebSocket outgoing task ended for cell '{}'", cell_id_clone);
    });

    // Task 2: Receive commands from WebSocket client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&text) {
                        match client_msg {
                            WsClientMessage::Ping => {}
                            WsClientMessage::AppendEvent {
                                turn_id,
                                event_type,
                                payload,
                            } => {
                                let _ = handle.append_event(turn_id, event_type, payload).await;
                            }
                            WsClientMessage::GetMeta => {}
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // If any task ends, abort the other
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
