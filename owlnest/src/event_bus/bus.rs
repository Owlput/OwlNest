use std::{collections::HashMap, time::Duration};
use tokio::{
    select,
    sync::{broadcast, mpsc}, time::MissedTickBehavior,
};
use super::{prelude::*, Handle};
use crate::event_bus::listened_event::EventListenerOp;

pub struct EventTap {
    inner: mpsc::Sender<ListenedEvent>,
}
impl EventTap {
    pub fn new(sender: mpsc::Sender<ListenedEvent>) -> Self {
        Self { inner: sender }
    }
    pub fn blocking_send(
        &self,
        value: impl Into<ListenedEvent>,
    ) -> Result<(), mpsc::error::SendError<ListenedEvent>> {
        self.inner.blocking_send(value.into())
    }
    pub async fn send(
        &self,
        value: impl Into<ListenedEvent>,
    ) -> Result<(), mpsc::error::SendError<ListenedEvent>> {
        self.inner.send(value.into()).await
    }
}

/// Spawn the task used for delegating operations to other tasks
/// that handle the actual event listening, returns a handle for
/// communicating with the task using a channel.
pub fn setup_ev_bus() -> (Handle, EventTap) {
    let (ev_tx, mut ev_rx) = mpsc::channel(8);
    let (op_tx, mut op_rx) = mpsc::channel(8);
    let mut listener_store: HashMap<String, broadcast::Sender<ListenedEvent>> = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    tokio::spawn(async move {
        select! {
            Some(op) = op_rx.recv()=>{
                match op{
                    EventListenerOp::Add(kind, callback_tx) => {
                        let key = format!("{:?}",kind);
                        if let Some((_,listener)) = listener_store.get_key_value(&key){
                            callback_tx.send(Ok(listener.subscribe()))
                        } else {
                            let (tx,rx) = broadcast::channel(8);
                            listener_store.insert(key, tx);
                            callback_tx.send(Ok(rx))
                        }
                    },
                };
            }
            Some(ev) = ev_rx.recv()=>{
                if let Some(listener) = listener_store.get(k){

                }
            }
            _ = interval.tick()=>{
                for (k,v) in listener_store{
                    if v.receiver_count() == 0{
                        listener_store.remove(&k);
                    }
                }
            }
        }
    });
    (Handle::new(op_tx), EventTap::new(ev_tx))
}
