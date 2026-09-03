use std::collections::HashMap;
use std::time::Instant;

use anyhow::Result;
use serde_json::Value;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::info;

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
    GetEvents {
        since_seq: Option<i64>,
        limit: Option<i64>,
        reply: oneshot::Sender<Result<Vec<EventRecord>>>,
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
        rx.await.map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn get_events(&self, since_seq: Option<i64>, limit: Option<i64>) -> Result<Vec<EventRecord>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::GetEvents {
                since_seq,
                limit,
                reply,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn get_messages(&self) -> Result<Vec<Message>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::GetMessages { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn get_meta(&self) -> Result<CellMeta> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::GetMeta { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
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
        rx.await.map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
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
        rx.await.map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn list_kv(&self) -> Result<HashMap<String, Value>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::ListKV { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
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
        rx.await.map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
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
        rx.await.map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
    }

    pub async fn checkpoint_wal(&self) -> Result<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(ActorMessage::CheckpointWal { reply })
            .await
            .map_err(|_| anyhow::anyhow!("Cell actor mailbox closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Cell actor dropped reply"))?
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
    pub fn spawn(cell_id: &str, db: CellDb) -> CellHandle {
        let (tx, rx) = mpsc::channel(128);
        let (event_bus, _) = broadcast::channel(256);

        let actor = Self {
            cell_id: cell_id.to_string(),
            db,
            event_bus: event_bus.clone(),
            rx,
            last_active: Instant::now(),
        };

        let cell_id_clone = cell_id.to_string();
        tokio::spawn(async move {
            actor.run().await;
        });

        CellHandle {
            cell_id: cell_id_clone,
            tx,
            event_bus,
        }
    }

    async fn run(mut self) {
        info!("CellActor [{}] spawned and running", self.cell_id);

        while let Some(msg) = self.rx.recv().await {
            self.last_active = Instant::now();

            match msg {
                ActorMessage::AppendEvent {
                    turn_id,
                    event_type,
                    payload,
                    reply,
                } => {
                    let res = self.db.append_event(turn_id, &event_type, payload).await;
                    if let Ok(ref record) = res {
                        // Broadcast event to active subscribers (SSE / WS)
                        let _ = self.event_bus.send(record.clone());
                    }
                    let _ = reply.send(res);
                }
                ActorMessage::GetEvents {
                    since_seq,
                    limit,
                    reply,
                } => {
                    let res = self.db.get_events(since_seq, limit).await;
                    let _ = reply.send(res);
                }
                ActorMessage::GetMessages { reply } => {
                    let res = self.db.get_messages().await;
                    let _ = reply.send(res);
                }
                ActorMessage::GetMeta { reply } => {
                    let res = self.db.get_meta().await;
                    let _ = reply.send(res);
                }
                ActorMessage::SetKV { key, value, reply } => {
                    let res = self.db.set_kv(&key, &value).await;
                    let _ = reply.send(res);
                }
                ActorMessage::GetKV { key, reply } => {
                    let res = self.db.get_kv(&key).await;
                    let _ = reply.send(res);
                }
                ActorMessage::ListKV { reply } => {
                    let res = self.db.list_kv().await;
                    let _ = reply.send(res);
                }
                ActorMessage::CreateCheckpoint { label, reply } => {
                    let res = self.db.create_checkpoint(&label).await;
                    let _ = reply.send(res);
                }
                ActorMessage::RestoreCheckpoint {
                    checkpoint_id,
                    reply,
                } => {
                    let res = self.db.restore_checkpoint(&checkpoint_id).await;
                    let _ = reply.send(res);
                }
                ActorMessage::CheckpointWal { reply } => {
                    let res = self.db.checkpoint_wal().await;
                    let _ = reply.send(res);
                }
                ActorMessage::Shutdown { reply } => {
                    info!("CellActor [{}] received shutdown signal", self.cell_id);
                    let _ = self.db.update_status(CellStatus::Suspended).await;
                    let _ = self.db.checkpoint_wal().await;
                    self.db.close().await;
                    let _ = reply.send(());
                    break;
                }
            }
        }

        info!("CellActor [{}] terminated", self.cell_id);
    }
}
