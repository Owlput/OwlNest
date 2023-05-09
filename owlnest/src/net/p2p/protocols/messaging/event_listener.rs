use std::collections::HashMap;
use super::OutEvent;
use crate::event_bus::{listener_event::*, Error};
use tokio::{
    select,
    sync::{mpsc, oneshot},
};
type ListenerStore = Box<[HashMap<u64, mpsc::Sender<ListenedEvent>>]>;

#[derive(Debug)]
pub enum Op {
    Add(
        Kind,
        mpsc::Sender<ListenedEvent>,
        oneshot::Sender<Result<u64, Error>>,
    ),
    Remove(u64, oneshot::Sender<Result<(), Error>>),
}

#[derive(Debug)]
pub enum Kind {
    OnIncomingMessage,
}
impl Into<EventListenerKind> for Kind {
    fn into(self) -> EventListenerKind {
        EventListenerKind::Behaviours(BehaviourListenerKind::Messaging(self))
    }
}

pub(crate) fn setup_event_listener() -> (mpsc::Sender<OutEvent>, mpsc::Sender<Op>) {
    let (ev_tx, mut ev_rx) = mpsc::channel(8);
    let (op_tx, mut op_rx) = mpsc::channel(8);
    let listener_store:ListenerStore = Box::new([HashMap::new(); 1]);
    tokio::spawn(async move {
        loop {
            select! {
                Some(ev) = ev_rx.recv()=>{
                    match ev{
                        OutEvent::IncomingMessage { .. } => {
                            for (_,listener) in &listener_store[0]{
                                listener.send(ev.clone().into()).await.unwrap()
                            }
                        },
                        OutEvent::Error(_) => todo!(),
                        OutEvent::Unsupported(_) => todo!(),
                        OutEvent::InboundNegotiated(_) => todo!(),
                        OutEvent::OutboundNegotiated(_) => todo!(),
                    }
                }
                Some(op) = op_rx.recv()=>{}
            };
        }
    });
    (ev_tx, op_tx)
}
