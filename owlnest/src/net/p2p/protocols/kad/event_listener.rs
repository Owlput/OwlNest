use super::*;
use crate::event_bus::{
    listener_event::{BehaviourListenerKind, EventListenerKind},
    Error,
};
use std::collections::HashMap;
use tokio::{
    select,
    sync::{mpsc, oneshot},
};

#[repr(i8)]
#[derive(Debug)]
pub enum Kind {
    OnOutboundQueryProgressed = 0,
}
impl Into<EventListenerKind> for Kind {
    fn into(self) -> EventListenerKind {
        EventListenerKind::Behaviours(BehaviourListenerKind::Kad(self))
    }
}

#[derive(Debug)]
pub enum Op {
    Add(
        Kind,
        mpsc::Sender<ListenedEvent>,
        oneshot::Sender<Result<u64, Error>>,
    ),
    Remove(u64, oneshot::Sender<Result<(), Error>>),
}

/// Spawn a task that delegates all events to their listeners.
pub(crate) fn setup_event_listener(
    buffer_size: usize,
) -> (mpsc::Sender<OutEvent>, mpsc::Sender<Op>) {
    let (ev_tx, mut ev_rx) = mpsc::channel(buffer_size);
    let (op_tx, mut op_rx) = mpsc::channel(8);
    tokio::spawn(async move {
        let mut listener_store: Box<[HashMap<u64, mpsc::Sender<ListenedEvent>>]> =
            Box::new([HashMap::new(); 1]);
        loop {
            select! {
                Some(ev) = ev_rx.recv() =>{
                    match ev {
                        OutEvent::OutboundQueryProgressed {
                            ..
                        } => {
                            for (_,sender) in &listener_store[0] {
                                sender.send(ev.clone().into()).await.unwrap();
                            }
                        }
                        _ => {}
                    }
                }
                Some(ev) = op_rx.recv()=>{
                    match ev{
                        Op::Add(kind,sender,callback) => {
                            let id:u64 = rand::random();
                            let result = match kind{
                                Kind::OnOutboundQueryProgressed => listener_store[0].try_insert(id, sender),
                            }.map_or(Err(Error::QueueFull), |_|Ok(id));
                            callback.send(result).unwrap();
                        },
                        Op::Remove(id,callback) => {
                            if let Some(_) = listener_store[0].remove(&id){
                                callback.send(Ok(())).unwrap()
                            } else {
                                callback.send(Err(Error::NotFound)).unwrap()
                            };
                        },
                    }
                }
            }
        }
    });
    (ev_tx, op_tx)
}
