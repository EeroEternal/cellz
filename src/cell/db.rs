use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::model::event::{EventRecord, Message};
use crate::model::state::{CellMeta, CellStatus, CheckpointRecord};

#[derive(Clone)]
pub struct CellDb {
    pool: SqlitePool,
    cell_id: String,
    db_path: PathBuf,
}

impl CellDb {
    /// Open or create an isolated SQLite database for a specific Cell.
    pub async fn open(cell_id: &str, db_path: impl AsRef<Path>, initial_name: Option<&str>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let db_url = format!("sqlite://{}", db_path.to_string_lossy());
        let options = SqliteConnectOptions::from_str(&db_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .with_context(|| format!("Failed to connect to SQLite at {:?}", db_path))?;

        let cell_db = Self {
            pool,
            cell_id: cell_id.to_string(),
            db_path,
        };

        cell_db.init_schema(initial_name).await?;
        Ok(cell_db)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn cell_id(&self) -> &str {
        &self.cell_id
    }

    async fn init_schema(&self, initial_name: Option<&str>) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS cell_meta (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                event_sequence INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS events (
                sequence INTEGER PRIMARY KEY,
                id TEXT NOT NULL UNIQUE,
                turn_id TEXT,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                name TEXT,
                tool_call_id TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS kv_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS checkpoints (
                id TEXT PRIMARY KEY,
                sequence INTEGER NOT NULL,
                label TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Initialize cell schema")?;

        // Initialize cell_meta record if not exists
        let now = Utc::now().to_rfc3339();
        let default_name = initial_name.unwrap_or(&self.cell_id);

        sqlx::query(
            r#"
            INSERT INTO cell_meta (id, name, status, event_sequence, created_at, updated_at, metadata)
            VALUES (?1, ?2, 'active', 0, ?3, ?3, '{}')
            ON CONFLICT(id) DO UPDATE SET updated_at = ?3;
            "#,
        )
        .bind(&self.cell_id)
        .bind(default_name)
        .bind(&now)
        .execute(&self.pool)
        .await
        .context("Seed cell_meta")?;

        Ok(())
    }

    pub async fn get_meta(&self) -> Result<CellMeta> {
        let row = sqlx::query(
            "SELECT id, name, status, event_sequence, created_at, updated_at, metadata FROM cell_meta WHERE id = ?1",
        )
        .bind(&self.cell_id)
        .fetch_one(&self.pool)
        .await
        .context("Fetch cell_meta")?;

        let status_str: String = row.get("status");
        let status = CellStatus::from_str(&status_str)
            .map_err(|e| anyhow::anyhow!(e))?;

        let created_at_str: String = row.get("created_at");
        let updated_at_str: String = row.get("updated_at");
        let metadata_str: String = row.get("metadata");

        Ok(CellMeta {
            id: row.get("id"),
            name: row.get("name"),
            status,
            event_sequence: row.get("event_sequence"),
            created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc),
            metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
        })
    }

    pub async fn update_status(&self, status: CellStatus) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE cell_meta SET status = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(status.as_str())
            .bind(&now)
            .bind(&self.cell_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn append_event(
        &self,
        turn_id: Option<String>,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<EventRecord> {
        let mut tx = self.pool.begin().await?;

        // 1. Increment monotonic sequence
        let seq_row = sqlx::query(
            "UPDATE cell_meta SET event_sequence = event_sequence + 1, updated_at = ?1 WHERE id = ?2 RETURNING event_sequence",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(&self.cell_id)
        .fetch_one(&mut *tx)
        .await?;

        let next_seq: i64 = seq_row.get(0);
        let event_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let payload_str = serde_json::to_string(&payload)?;

        // 2. Insert event
        sqlx::query(
            "INSERT INTO events (sequence, id, turn_id, event_type, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(next_seq)
        .bind(&event_id)
        .bind(&turn_id)
        .bind(event_type)
        .bind(&payload_str)
        .bind(&now_str)
        .execute(&mut *tx)
        .await?;

        // 3. Materialize message projection if event represents message
        if event_type == "user_message" || event_type == "agent_message" || event_type == "system_message" {
            let role = match event_type {
                "user_message" => "user",
                "agent_message" => "assistant",
                _ => "system",
            };
            let content = payload.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let name = payload.get("name").and_then(|v| v.as_str());
            let tool_call_id = payload.get("tool_call_id").and_then(|v| v.as_str());

            sqlx::query(
                "INSERT INTO messages (id, role, content, name, tool_call_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET content = ?3;",
            )
            .bind(&event_id)
            .bind(role)
            .bind(content)
            .bind(name)
            .bind(tool_call_id)
            .bind(&now_str)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(EventRecord {
            sequence: next_seq,
            id: event_id,
            cell_id: self.cell_id.clone(),
            turn_id,
            event_type: event_type.to_string(),
            payload,
            created_at: now,
        })
    }

    pub async fn append_events_batch(
        &self,
        requests: Vec<crate::model::event::AppendEventRequest>,
    ) -> Result<Vec<EventRecord>> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.pool.begin().await?;
        let count = requests.len() as i64;

        let seq_row = sqlx::query(
            "UPDATE cell_meta SET event_sequence = event_sequence + ?1, updated_at = ?2 WHERE id = ?3 RETURNING event_sequence",
        )
        .bind(count)
        .bind(Utc::now().to_rfc3339())
        .bind(&self.cell_id)
        .fetch_one(&mut *tx)
        .await?;

        let final_seq: i64 = seq_row.get(0);
        let start_seq = final_seq - count + 1;

        let mut records = Vec::with_capacity(requests.len());
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        for (idx, req) in requests.into_iter().enumerate() {
            let seq = start_seq + idx as i64;
            let event_id = Uuid::new_v4().to_string();
            let payload_str = serde_json::to_string(&req.payload)?;

            sqlx::query(
                "INSERT INTO events (sequence, id, turn_id, event_type, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .bind(seq)
            .bind(&event_id)
            .bind(&req.turn_id)
            .bind(&req.event_type)
            .bind(&payload_str)
            .bind(&now_str)
            .execute(&mut *tx)
            .await?;

            if req.event_type == "user_message" || req.event_type == "agent_message" || req.event_type == "system_message" {
                let role = match req.event_type.as_str() {
                    "user_message" => "user",
                    "agent_message" => "assistant",
                    _ => "system",
                };
                let content = req.payload.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let name = req.payload.get("name").and_then(|v| v.as_str());
                let tool_call_id = req.payload.get("tool_call_id").and_then(|v| v.as_str());

                sqlx::query(
                    "INSERT INTO messages (id, role, content, name, tool_call_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(id) DO UPDATE SET content = ?3;",
                )
                .bind(&event_id)
                .bind(role)
                .bind(content)
                .bind(name)
                .bind(tool_call_id)
                .bind(&now_str)
                .execute(&mut *tx)
                .await?;
            }

            records.push(EventRecord {
                sequence: seq,
                id: event_id,
                cell_id: self.cell_id.clone(),
                turn_id: req.turn_id,
                event_type: req.event_type,
                payload: req.payload,
                created_at: now,
            });
        }

        tx.commit().await?;
        Ok(records)
    }

    pub async fn export_cell(&self) -> Result<crate::model::state::CellExport> {
        let meta = self.get_meta().await?;
        let events = self.get_events(None, Some(100_000)).await?;
        let messages = self.get_messages().await?;
        let kv = self.list_kv().await?;
        let checkpoints = self.list_checkpoints().await?;

        Ok(crate::model::state::CellExport {
            meta,
            events,
            messages,
            kv,
            checkpoints,
        })
    }

    pub async fn get_events(&self, since_seq: Option<i64>, limit: Option<i64>) -> Result<Vec<EventRecord>> {
        let since = since_seq.unwrap_or(0);
        let lim = limit.unwrap_or(500);

        let rows = sqlx::query(
            "SELECT sequence, id, turn_id, event_type, payload, created_at FROM events WHERE sequence > ?1 ORDER BY sequence ASC LIMIT ?2",
        )
        .bind(since)
        .bind(lim)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let created_at_str: String = row.get("created_at");
            let payload_str: String = row.get("payload");
            events.push(EventRecord {
                sequence: row.get("sequence"),
                id: row.get("id"),
                cell_id: self.cell_id.clone(),
                turn_id: row.get("turn_id"),
                event_type: row.get("event_type"),
                payload: serde_json::from_str(&payload_str).unwrap_or(Value::Null),
                created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
            });
        }
        Ok(events)
    }

    pub async fn get_messages(&self) -> Result<Vec<Message>> {
        let rows = sqlx::query(
            "SELECT id, role, content, name, tool_call_id, created_at FROM messages ORDER BY rowid ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut msgs = Vec::with_capacity(rows.len());
        for row in rows {
            let created_at_str: String = row.get("created_at");
            msgs.push(Message {
                id: row.get("id"),
                role: row.get("role"),
                content: row.get("content"),
                name: row.get("name"),
                tool_call_id: row.get("tool_call_id"),
                created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
            });
        }
        Ok(msgs)
    }

    pub async fn set_kv(&self, key: &str, value: &Value) -> Result<()> {
        let val_str = serde_json::to_string(value)?;
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO kv_state (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3;",
        )
        .bind(key)
        .bind(&val_str)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_kv(&self, key: &str) -> Result<Option<Value>> {
        let row = sqlx::query("SELECT value FROM kv_state WHERE key = ?1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(r) = row {
            let val_str: String = r.get("value");
            Ok(Some(serde_json::from_str(&val_str)?))
        } else {
            Ok(None)
        }
    }

    pub async fn list_kv(&self) -> Result<HashMap<String, Value>> {
        let rows = sqlx::query("SELECT key, value FROM kv_state")
            .fetch_all(&self.pool)
            .await?;

        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            let key: String = row.get("key");
            let val_str: String = row.get("value");
            map.insert(key, serde_json::from_str(&val_str).unwrap_or(Value::Null));
        }
        Ok(map)
    }

    pub async fn create_checkpoint(&self, label: &str) -> Result<CheckpointRecord> {
        let meta = self.get_meta().await?;
        let cp_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        sqlx::query("INSERT INTO checkpoints (id, sequence, label, created_at) VALUES (?1, ?2, ?3, ?4)")
            .bind(&cp_id)
            .bind(meta.event_sequence)
            .bind(label)
            .bind(&now_str)
            .execute(&self.pool)
            .await?;

        Ok(CheckpointRecord {
            id: cp_id,
            cell_id: self.cell_id.clone(),
            sequence: meta.event_sequence,
            label: label.to_string(),
            created_at: now,
        })
    }

    pub async fn list_checkpoints(&self) -> Result<Vec<CheckpointRecord>> {
        let rows = sqlx::query("SELECT id, sequence, label, created_at FROM checkpoints ORDER BY sequence ASC")
            .fetch_all(&self.pool)
            .await?;

        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            let created_at_str: String = row.get("created_at");
            list.push(CheckpointRecord {
                id: row.get("id"),
                cell_id: self.cell_id.clone(),
                sequence: row.get("sequence"),
                label: row.get("label"),
                created_at: DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc),
            });
        }
        Ok(list)
    }

    pub async fn restore_checkpoint(&self, checkpoint_id: &str) -> Result<i64> {
        let row = sqlx::query("SELECT sequence FROM checkpoints WHERE id = ?1")
            .bind(checkpoint_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Checkpoint {} not found", checkpoint_id))?;

        let target_seq: i64 = row.get("sequence");
        let now_str = Utc::now().to_rfc3339();

        let mut tx = self.pool.begin().await?;

        // Delete events after target_seq
        sqlx::query("DELETE FROM events WHERE sequence > ?1")
            .bind(target_seq)
            .execute(&mut *tx)
            .await?;

        // Update cell_meta sequence
        sqlx::query("UPDATE cell_meta SET event_sequence = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(target_seq)
            .bind(&now_str)
            .bind(&self.cell_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        // Log a rewind event
        self.append_event(
            None,
            "rewound",
            serde_json::json!({
                "checkpoint_id": checkpoint_id,
                "target_sequence": target_seq,
            }),
        )
        .await?;

        Ok(target_seq)
    }

    /// Explicit SQLite WAL checkpoint to flush changes to disk.
    pub async fn checkpoint_wal(&self) -> Result<()> {
        sqlx::query("PRAGMA wal_checkpoint(PASSIVE);")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}
