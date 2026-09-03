use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellStatus {
    Active,
    Idle,
    Suspended,
    Terminated,
}

impl CellStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Idle => "idle",
            Self::Suspended => "suspended",
            Self::Terminated => "terminated",
        }
    }
}

impl std::fmt::Display for CellStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for CellStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "idle" => Ok(Self::Idle),
            "suspended" => Ok(Self::Suspended),
            "terminated" => Ok(Self::Terminated),
            other => Err(format!("Unknown cell status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellMeta {
    pub id: String,
    pub name: String,
    pub status: CellStatus,
    pub event_sequence: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCellRequest {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub id: String,
    pub cell_id: String,
    pub sequence: i64,
    pub label: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCheckpointRequest {
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreCheckpointRequest {
    pub checkpoint_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetKVRequest {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchAppendRequest {
    pub events: Vec<crate::model::event::AppendEventRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellExport {
    pub meta: CellMeta,
    pub events: Vec<crate::model::event::EventRecord>,
    pub messages: Vec<crate::model::event::Message>,
    pub kv: std::collections::HashMap<String, serde_json::Value>,
    pub checkpoints: Vec<CheckpointRecord>,
}
