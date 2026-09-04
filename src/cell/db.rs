use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use uuid::Uuid;

use crate::model::event::{EventRecord, Message};
use crate::model::state::{CellMeta, CellStatus, CheckpointRecord};

pub struct CellDb {
    conn: Connection,
    cell_id: String,
    db_path: PathBuf,
}

impl CellDb {
    /// Open or create an isolated SQLite database for a specific Cell.
    pub fn open(
        cell_id: &str,
        db_path: impl AsRef<Path>,
        initial_name: Option<&str>,
    ) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        let conn = open_connection(&db_path)?;
        let cell_db = Self {
            conn,
            cell_id: cell_id.to_string(),
            db_path,
        };
        cell_db.init_schema(initial_name)?;
        Ok(cell_db)
    }

    /// Read cell_meta from an existing file without starting an actor.
    pub fn peek_meta(db_path: impl AsRef<Path>) -> Result<CellMeta> {
        let conn = Connection::open_with_flags(
            db_path.as_ref(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .with_context(|| format!("Failed to open SQLite at {:?}", db_path.as_ref()))?;
        conn.busy_timeout(Duration::from_secs(5))?;
        load_meta(&conn, None)
    }

    /// Checkpoint WAL on an inactive file and read the snapshot bytes.
    pub fn snapshot_file(cell_id: &str, db_path: impl AsRef<Path>) -> Result<(Vec<u8>, CellMeta)> {
        let db = Self::open(cell_id, db_path.as_ref(), None)?;
        db.checkpoint_wal()?;
        let meta = db.get_meta()?;
        let path = db.db_path.clone();
        drop(db);
        let bytes = std::fs::read(&path)
            .with_context(|| format!("Failed to read snapshot at {:?}", path))?;
        Ok((bytes, meta))
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn cell_id(&self) -> &str {
        &self.cell_id
    }

    fn init_schema(&self, initial_name: Option<&str>) -> Result<()> {
        self.conn
            .execute_batch(
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
            .context("Initialize cell schema")?;

        let now = Utc::now().to_rfc3339();
        let default_name = initial_name.unwrap_or(&self.cell_id);

        self.conn
            .execute(
                r#"
            INSERT INTO cell_meta (id, name, status, event_sequence, created_at, updated_at, metadata)
            VALUES (?1, ?2, 'active', 0, ?3, ?3, '{}')
            ON CONFLICT(id) DO UPDATE SET updated_at = ?3;
            "#,
                params![&self.cell_id, default_name, &now],
            )
            .context("Seed cell_meta")?;

        Ok(())
    }

    pub fn get_meta(&self) -> Result<CellMeta> {
        load_meta(&self.conn, Some(&self.cell_id))
    }

    pub fn update_status(&self, status: CellStatus) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE cell_meta SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), &now, &self.cell_id],
        )?;
        Ok(())
    }

    pub fn append_event(
        &mut self,
        turn_id: Option<String>,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<EventRecord> {
        let tx = self.conn.transaction()?;

        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let next_seq: i64 = tx.query_row(
            "UPDATE cell_meta SET event_sequence = event_sequence + 1, updated_at = ?1 WHERE id = ?2 RETURNING event_sequence",
            params![&now_str, &self.cell_id],
            |row| row.get(0),
        )?;

        let event_id = Uuid::new_v4().to_string();
        let payload_str = serde_json::to_string(&payload)?;

        tx.execute(
            "INSERT INTO events (sequence, id, turn_id, event_type, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![next_seq, &event_id, &turn_id, event_type, &payload_str, &now_str],
        )?;

        if event_type == "user_message"
            || event_type == "agent_message"
            || event_type == "system_message"
        {
            materialize_message(&tx, event_type, &event_id, &payload, &now_str)?;
        }

        tx.commit()?;

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

    pub fn append_events_batch(
        &mut self,
        requests: Vec<crate::model::event::AppendEventRequest>,
    ) -> Result<Vec<EventRecord>> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        let tx = self.conn.transaction()?;
        let count = requests.len() as i64;
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        let final_seq: i64 = tx.query_row(
            "UPDATE cell_meta SET event_sequence = event_sequence + ?1, updated_at = ?2 WHERE id = ?3 RETURNING event_sequence",
            params![count, &now_str, &self.cell_id],
            |row| row.get(0),
        )?;
        let start_seq = final_seq - count + 1;

        let mut records = Vec::with_capacity(requests.len());
        for (idx, req) in requests.into_iter().enumerate() {
            let seq = start_seq + idx as i64;
            let event_id = Uuid::new_v4().to_string();
            let payload_str = serde_json::to_string(&req.payload)?;

            tx.execute(
                "INSERT INTO events (sequence, id, turn_id, event_type, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![seq, &event_id, &req.turn_id, &req.event_type, &payload_str, &now_str],
            )?;

            if req.event_type == "user_message"
                || req.event_type == "agent_message"
                || req.event_type == "system_message"
            {
                materialize_message(&tx, &req.event_type, &event_id, &req.payload, &now_str)?;
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

        tx.commit()?;
        Ok(records)
    }

    pub fn export_cell(&self) -> Result<crate::model::state::CellExport> {
        let meta = self.get_meta()?;
        let events = self.get_events(None, Some(100_000))?;
        let messages = self.get_messages()?;
        let kv = self.list_kv()?;
        let checkpoints = self.list_checkpoints()?;

        Ok(crate::model::state::CellExport {
            meta,
            events,
            messages,
            kv,
            checkpoints,
        })
    }

    pub fn get_events(
        &self,
        since_seq: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<EventRecord>> {
        let since = since_seq.unwrap_or(0);
        let lim = limit.unwrap_or(500);
        let mut stmt = self.conn.prepare(
            "SELECT sequence, id, turn_id, event_type, payload, created_at FROM events WHERE sequence > ?1 ORDER BY sequence ASC LIMIT ?2",
        )?;
        let mut rows = stmt.query(params![since, lim])?;
        let mut events = Vec::new();
        while let Some(row) = rows.next()? {
            let created_at_str: String = row.get("created_at")?;
            let payload_str: String = row.get("payload")?;
            events.push(EventRecord {
                sequence: row.get("sequence")?,
                id: row.get("id")?,
                cell_id: self.cell_id.clone(),
                turn_id: row.get("turn_id")?,
                event_type: row.get("event_type")?,
                payload: serde_json::from_str(&payload_str).unwrap_or(Value::Null),
                created_at: parse_rfc3339(&created_at_str)?,
            });
        }
        Ok(events)
    }

    pub fn get_messages(&self) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, name, tool_call_id, created_at FROM messages ORDER BY rowid ASC",
        )?;
        let mut rows = stmt.query([])?;
        let mut msgs = Vec::new();
        while let Some(row) = rows.next()? {
            let created_at_str: String = row.get("created_at")?;
            msgs.push(Message {
                id: row.get("id")?,
                role: row.get("role")?,
                content: row.get("content")?,
                name: row.get("name")?,
                tool_call_id: row.get("tool_call_id")?,
                created_at: parse_rfc3339(&created_at_str)?,
            });
        }
        Ok(msgs)
    }

    pub fn set_kv(&self, key: &str, value: &Value) -> Result<()> {
        let val_str = serde_json::to_string(value)?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO kv_state (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = ?3;",
            params![key, &val_str, &now],
        )?;
        Ok(())
    }

    pub fn get_kv(&self, key: &str) -> Result<Option<Value>> {
        let val_str: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM kv_state WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?;
        match val_str {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    pub fn list_kv(&self) -> Result<HashMap<String, Value>> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM kv_state")?;
        let mut rows = stmt.query([])?;
        let mut map = HashMap::new();
        while let Some(row) = rows.next()? {
            let key: String = row.get("key")?;
            let val_str: String = row.get("value")?;
            map.insert(key, serde_json::from_str(&val_str).unwrap_or(Value::Null));
        }
        Ok(map)
    }

    pub fn create_checkpoint(&self, label: &str) -> Result<CheckpointRecord> {
        let meta = self.get_meta()?;
        let cp_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        self.conn.execute(
            "INSERT INTO checkpoints (id, sequence, label, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![&cp_id, meta.event_sequence, label, &now_str],
        )?;

        Ok(CheckpointRecord {
            id: cp_id,
            cell_id: self.cell_id.clone(),
            sequence: meta.event_sequence,
            label: label.to_string(),
            created_at: now,
        })
    }

    pub fn list_checkpoints(&self) -> Result<Vec<CheckpointRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sequence, label, created_at FROM checkpoints ORDER BY sequence ASC",
        )?;
        let mut rows = stmt.query([])?;
        let mut list = Vec::new();
        while let Some(row) = rows.next()? {
            let created_at_str: String = row.get("created_at")?;
            list.push(CheckpointRecord {
                id: row.get("id")?,
                cell_id: self.cell_id.clone(),
                sequence: row.get("sequence")?,
                label: row.get("label")?,
                created_at: parse_rfc3339(&created_at_str)?,
            });
        }
        Ok(list)
    }

    pub fn restore_checkpoint(&mut self, checkpoint_id: &str) -> Result<i64> {
        let target_seq: i64 = self
            .conn
            .query_row(
                "SELECT sequence FROM checkpoints WHERE id = ?1",
                params![checkpoint_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("Checkpoint {} not found", checkpoint_id))?;
        let now_str = Utc::now().to_rfc3339();

        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM events WHERE sequence > ?1",
            params![target_seq],
        )?;
        tx.execute(
            "UPDATE cell_meta SET event_sequence = ?1, updated_at = ?2 WHERE id = ?3",
            params![target_seq, &now_str, &self.cell_id],
        )?;
        tx.commit()?;

        self.append_event(
            None,
            "rewound",
            serde_json::json!({
                "checkpoint_id": checkpoint_id,
                "target_sequence": target_seq,
            }),
        )?;

        Ok(target_seq)
    }

    /// Explicit SQLite WAL checkpoint to flush and truncate WAL changes to disk.
    pub fn checkpoint_wal(&self) -> Result<()> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }
}

fn open_connection(db_path: &Path) -> Result<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create cell dir {:?}", parent))?;
    }
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to connect to SQLite at {:?}", db_path))?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(conn)
}

fn load_meta(conn: &Connection, cell_id: Option<&str>) -> Result<CellMeta> {
    let row = if let Some(id) = cell_id {
        conn.query_row(
            "SELECT id, name, status, event_sequence, created_at, updated_at, metadata FROM cell_meta WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>("id")?,
                    row.get::<_, String>("name")?,
                    row.get::<_, String>("status")?,
                    row.get::<_, i64>("event_sequence")?,
                    row.get::<_, String>("created_at")?,
                    row.get::<_, String>("updated_at")?,
                    row.get::<_, String>("metadata")?,
                ))
            },
        )
        .context("Fetch cell_meta")?
    } else {
        conn.query_row(
            "SELECT id, name, status, event_sequence, created_at, updated_at, metadata FROM cell_meta LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>("id")?,
                    row.get::<_, String>("name")?,
                    row.get::<_, String>("status")?,
                    row.get::<_, i64>("event_sequence")?,
                    row.get::<_, String>("created_at")?,
                    row.get::<_, String>("updated_at")?,
                    row.get::<_, String>("metadata")?,
                ))
            },
        )
        .context("Fetch cell_meta")?
    };

    let (id, name, status_str, event_sequence, created_at_str, updated_at_str, metadata_str) = row;
    let status = CellStatus::from_str(&status_str).map_err(|e| anyhow::anyhow!(e))?;

    Ok(CellMeta {
        id,
        name,
        status,
        event_sequence,
        created_at: parse_rfc3339(&created_at_str)?,
        updated_at: parse_rfc3339(&updated_at_str)?,
        metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
    })
}

fn materialize_message(
    tx: &rusqlite::Transaction,
    event_type: &str,
    event_id: &str,
    payload: &Value,
    now_str: &str,
) -> Result<()> {
    let role = match event_type {
        "user_message" => "user",
        "agent_message" => "assistant",
        _ => "system",
    };
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let name = payload.get("name").and_then(|v| v.as_str());
    let tool_call_id = payload.get("tool_call_id").and_then(|v| v.as_str());

    tx.execute(
        "INSERT INTO messages (id, role, content, name, tool_call_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET content = ?3;",
        params![event_id, role, content, name, tool_call_id, now_str],
    )?;
    Ok(())
}

fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(s)?.with_timezone(&Utc))
}
