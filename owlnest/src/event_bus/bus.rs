use super::{prelude::*, Handle};
use crate::event_bus::listened_event::EventListenerOp;
use std::{collections::HashMap, time::Duration};
use tokio::{
    select,
    sync::{broadcast, mpsc},
    time::MissedTickBehavior,
};

#[derive(Debug, Clone)]
pub struct EventTap(mpsc::Sender<ListenedEvent>);
impl EventTap {
    pub(crate) fn new(sender: mpsc::Sender<ListenedEvent>) -> Self {
        Self(sender)
    }
    pub fn blocking_send(
        &self,
        value: ListenedEvent,
    ) -> Result<(), mpsc::error::SendError<ListenedEvent>> {
        self.0.blocking_send(value)
    }
    pub async fn send(
        &self,
        value: ListenedEvent,
    ) -> Result<(), mpsc::error::SendError<ListenedEvent>> {
        self.0.send(value.into()).await
    }
}

/// Spawn the task used for delegating operations to other tasks
/// that handle the actual event listening, returns a handle for
/// communicating with the task using a channel.
pub fn setup_ev_bus() -> (Handle, EventTap) {
    let (ev_tx, mut ev_rx) = mpsc::channel::<ListenedEvent>(8);
    let (op_tx, mut op_rx) = mpsc::channel(8);
    let mut listener_store: HashMap<String, broadcast::Sender<ListenedEvent>> = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    tokio::spawn(async move {
        select! {
            Some(op) = op_rx.recv()=>{
                match op{
                    EventListenerOp::Add(kind, callback_tx) => {
                        if let Some((_,listener)) = listener_store.get_key_value(&kind){
                            callback_tx.send(Ok(listener.subscribe())).unwrap()
                        } else {
                            let (tx,rx) = broadcast::channel(8);
                            listener_store.insert(kind, tx);
                            callback_tx.send(Ok(rx)).unwrap()
                        }
                    },
                };
            }
            Some(ev) = ev_rx.recv()=>{
                if let Some(listener) = listener_store.get(&ev.kind()){
                    let _ = listener.send(ev);
                }
            }
            _ = interval.tick()=>{
                listener_store.drain_filter(|_,v|v.receiver_count() == 0).count();
            }
        }
    });
    (Handle::new(op_tx), EventTap::new(ev_tx))
}
