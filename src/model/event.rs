use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier and sequence for an event within a Cell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventRecord {
    pub sequence: i64,
    pub id: String,
    pub cell_id: String,
    pub turn_id: Option<String>,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl EventRecord {
    pub fn new(
        sequence: i64,
        cell_id: String,
        turn_id: Option<String>,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            sequence,
            id: Uuid::new_v4().to_string(),
            cell_id,
            turn_id,
            event_type: event_type.into(),
            payload,
            created_at: Utc::now(),
        }
    }
}

/// Request to append an event to a Cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEventRequest {
    pub turn_id: Option<String>,
    pub event_type: String,
    pub payload: serde_json::Value,
}

/// Standard Message representation projected from events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    pub created_at: DateTime<Utc>,
}
