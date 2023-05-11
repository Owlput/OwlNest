use tokio::{select, sync::mpsc};

use super::{listener_event::EventListenerOp, EventBusHandle};
use crate::net::p2p::protocols::*;
pub type OpOutBundle = (
    mpsc::Sender<messaging::event_listener::Op>,
    mpsc::Sender<kad::event_listener::Op>,
);
pub type EvInBundle = (
    mpsc::Sender<messaging::OutEvent>,
    mpsc::Sender<kad::OutEvent>
);

/// Spawn the task used for delegating operations to other tasks
/// that handle the actual event listening, returns a handle for
/// communicating with the task using a channel.
pub fn setup_ev_bus(op_out_bundle: OpOutBundle) -> EventBusHandle {
    let (handle_tx, mut handle_rx) = mpsc::channel(8);
    let handle = EventBusHandle { sender: handle_tx };
    tokio::spawn(async move {
        select! {
            Some(op) = handle_rx.recv()=>{
                match op{
                    EventListenerOp::Add(kind, ev_tx, callback_tx) => {
                        match kind{
                            super::listener_event::EventListenerKind::Behaviours(kind) => match kind{
                                super::listener_event::BehaviourListenerKind::Messaging(kind) => {
                                    let op = messaging::event_listener::Op::Add(kind,ev_tx,callback_tx);
                                    op_out_bundle.0.send(op).await.unwrap()
                                },
                                super::listener_event::BehaviourListenerKind::Kad(kind) => {
                                    let op = kad::event_listener::Op::Add(kind,ev_tx,callback_tx);
                                    op_out_bundle.1.send(op).await.unwrap()
                                },
                            },
                            super::listener_event::EventListenerKind::Swarm(_) => todo!(),
                        }
                    },
                    EventListenerOp::Remove(_, _, _) => todo!(),
                }
            }
        }
    });
    handle
}
