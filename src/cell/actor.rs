use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{info, warn};

use crate::cell::db::CellDb;
use crate::model::event::{EventRecord, Message};
use crate::model::state::{CellMeta, CellStatus, CheckpointRecord};

pub enum ActorMessage {
    AppendEvent {
        turn_id: Option<String>,
        event_type: String,
        payload: Value,
        reply: oneshot::Sender<Result<EventRecord>>,
    },
    AppendEventsBatch {
        requests: Vec<crate::model::event::AppendEventRequest>,
        reply: oneshot::Sender<Result<Vec<EventRecord>>>,
    },
    GetEvents {
        since_seq: Option<i64>,
        limit: Option<i64>,
        reply: oneshot::Sender<Result<Vec<EventRecord>>>,
    },
    Export {
        reply: oneshot::Sender<Result<crate::model::state::CellExport>>,
    },
    GetMessages {
        reply: oneshot::Sender<Result<Vec<Message>>>,
    },
    GetMeta {
        reply: oneshot::Sender<Result<CellMeta>>,
    },
    SetKV {
        key: String,
        value: Value,
        reply: oneshot::Sender<Result<()>>,
    },
    GetKV {
        key: String,
        reply: oneshot::Sender<Result<Option<Value>>>,
    },
    ListKV {
        reply: oneshot::Sender<Result<HashMap<String, Value>>>,
    },
    CreateCheckpoint {
        label: String,
        reply: oneshot::Sender<Result<CheckpointRecord>>,
    },
    RestoreCheckpoint {
        checkpoint_id: String,
        reply: oneshot::Sender<Result<i64>>,
    },
    CheckpointWal {
        reply: oneshot::Sender<Result<()>>,
    },
    Backup {
        reply: oneshot::Sender<Result<Vec<u8>>>,
    },
    Fence,
    GetIdleDuration {
        reply: oneshot::Sender<std::time::Duration>,
    },
    Shutdown {
        reply: oneshot::Sender<()>,
    },
}

#[derive(Clone)]
pub struct CellHandle {
    pub cell_id: String,
    tx: mpsc::Sender<ActorMessage>,
    event_bus: broadcast::Sender<EventRecord>,
}

impl CellHandle {
    pub async fn append_event(
        &self,
        turn_id: Option<String>,
        event_type: impl Into<String>,
        payload: Value,
    ) -> Result<EventRecord> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::AppendEvent {
                turn_id,
                event_type: event_type.into(),
                payload,
                reply,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn append_events_batch(
        &self,
        requests: Vec<crate::model::event::AppendEventRequest>,
    ) -> Result<Vec<EventRecord>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::AppendEventsBatch { requests, reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn export(&self) -> Result<crate::model::state::CellExport> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::Export { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn get_events(
        &self,
        since_seq: Option<i64>,
        limit: Option<i64>,
    ) -> Result<Vec<EventRecord>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::GetEvents {
                since_seq,
                limit,
                reply,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn get_messages(&self) -> Result<Vec<Message>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::GetMessages { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn get_meta(&self) -> Result<CellMeta> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::GetMeta { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn set_kv(&self, key: impl Into<String>, value: Value) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::SetKV {
                key: key.into(),
                value,
                reply,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn get_kv(&self, key: impl Into<String>) -> Result<Option<Value>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::GetKV {
                key: key.into(),
                reply,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn list_kv(&self) -> Result<HashMap<String, Value>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::ListKV { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn create_checkpoint(&self, label: impl Into<String>) -> Result<CheckpointRecord> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::CreateCheckpoint {
                label: label.into(),
                reply,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn restore_checkpoint(&self, checkpoint_id: impl Into<String>) -> Result<i64> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::RestoreCheckpoint {
                checkpoint_id: checkpoint_id.into(),
                reply,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn checkpoint_wal(&self) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::CheckpointWal { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn backup(&self) -> Result<Vec<u8>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::Backup { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn fence(&self) {
        let _ = self.tx.send(ActorMessage::Fence).await;
    }

    pub async fn idle_duration(&self) -> Result<std::time::Duration> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::GetIdleDuration { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))
    }

    pub async fn shutdown(&self) {
        let (reply, rx) = oneshot::channel();
        if self.tx.send(ActorMessage::Shutdown { reply }).await.is_ok() {
            let _ = rx.await;
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EventRecord> {
        self.event_bus.subscribe()
    }
}

pub struct CellActor {
    cell_id: String,
    db: CellDb,
    event_bus: broadcast::Sender<EventRecord>,
    rx: mpsc::Receiver<ActorMessage>,
    last_active: Instant,
}

impl CellActor {
    /// Spawn a dedicated OS thread that owns the cell's rusqlite connection.
    pub fn spawn(
        cell_id: impl Into<String>,
        db_path: impl AsRef<Path>,
        initial_name: Option<String>,
    ) -> Result<CellHandle> {
        let cell_id = cell_id.into();
        let db_path = db_path.as_ref().to_path_buf();
        let (tx, rx) = mpsc::channel(128);
        let (event_bus, _) = broadcast::channel(256);
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);

        let thread_cell_id = cell_id.clone();
        let event_bus_for_actor = event_bus.clone();
        let short: String = cell_id.chars().take(8).collect();
        let thread_name = format!("cell-{short}");

        std::thread::Builder::new()
            .name(thread_name)
            .spawn(
                move || match CellDb::open(&thread_cell_id, &db_path, initial_name.as_deref()) {
                    Ok(db) => {
                        let actor = CellActor {
                            cell_id: thread_cell_id,
                            db,
                            event_bus: event_bus_for_actor,
                            rx,
                            last_active: Instant::now(),
                        };
                        let _ = ready_tx.send(Ok(()));
                        actor.run();
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                    }
                },
            )
            .context("failed to spawn cell actor thread")?;

        ready_rx
            .recv()
            .context("cell actor thread dropped before ready")??;

        Ok(CellHandle {
            cell_id,
            tx,
            event_bus,
        })
    }

    fn run(mut self) {
        info!("CellActor [{}] spawned and running", self.cell_id);

        while let Some(msg) = self.rx.blocking_recv() {
            self.last_active = Instant::now();

            match msg {
                ActorMessage::AppendEvent {
                    turn_id,
                    event_type,
                    payload,
                    reply,
                } => {
                    let res = self.db.append_event(turn_id, &event_type, payload);
                    if let Ok(ref record) = res {
                        let _ = self.event_bus.send(record.clone());
                    }
                    let _ = reply.send(res);
                }
                ActorMessage::AppendEventsBatch { requests, reply } => {
                    let res = self.db.append_events_batch(requests);
                    if let Ok(ref records) = res {
                        for rec in records {
                            let _ = self.event_bus.send(rec.clone());
                        }
                    }
                    let _ = reply.send(res);
                }
                ActorMessage::Export { reply } => {
                    let res = self.db.export_cell();
                    let _ = reply.send(res);
                }
                ActorMessage::GetEvents {
                    since_seq,
                    limit,
                    reply,
                } => {
                    let res = self.db.get_events(since_seq, limit);
                    let _ = reply.send(res);
                }
                ActorMessage::GetMessages { reply } => {
                    let res = self.db.get_messages();
                    let _ = reply.send(res);
                }
                ActorMessage::GetMeta { reply } => {
                    let res = self.db.get_meta();
                    let _ = reply.send(res);
                }
                ActorMessage::SetKV { key, value, reply } => {
                    let res = self.db.set_kv(&key, &value);
                    let _ = reply.send(res);
                }
                ActorMessage::GetKV { key, reply } => {
                    let res = self.db.get_kv(&key);
                    let _ = reply.send(res);
                }
                ActorMessage::ListKV { reply } => {
                    let res = self.db.list_kv();
                    let _ = reply.send(res);
                }
                ActorMessage::CreateCheckpoint { label, reply } => {
                    let res = self.db.create_checkpoint(&label);
                    let _ = reply.send(res);
                }
                ActorMessage::RestoreCheckpoint {
                    checkpoint_id,
                    reply,
                } => {
                    let res = self.db.restore_checkpoint(&checkpoint_id);
                    let _ = reply.send(res);
                }
                ActorMessage::CheckpointWal { reply } => {
                    let res = self.db.checkpoint_wal();
                    let _ = reply.send(res);
                }
                ActorMessage::Backup { reply } => {
                    let res = (|| {
                        self.db.checkpoint_wal()?;
                        std::fs::read(self.db.db_path()).map_err(Into::into)
                    })();
                    let _ = reply.send(res);
                }
                ActorMessage::Fence => {
                    warn!(
                        "CellActor [{}] fenced due to lease expiration or takeover",
                        self.cell_id
                    );
                    let _ = self.db.update_status(CellStatus::Suspended);
                    break;
                }
                ActorMessage::GetIdleDuration { reply } => {
                    let _ = reply.send(self.last_active.elapsed());
                }
                ActorMessage::Shutdown { reply } => {
                    info!("CellActor [{}] received shutdown signal", self.cell_id);
                    let _ = self.db.update_status(CellStatus::Suspended);
                    let _ = self.db.checkpoint_wal();
                    let _ = reply.send(());
                    break;
                }
            }
        }

        info!("CellActor [{}] terminated", self.cell_id);
    }
}
