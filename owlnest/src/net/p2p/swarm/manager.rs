use super::in_event::ProtocolInEvent;
use crate::net::p2p::protocols::*;
use crate::net::p2p::swarm;
use libp2p::PeerId;
use libp2p::{
    swarm::{derive_prelude::ListenerId, DialError},
    Multiaddr, TransportError,
};
use tokio::sync::{mpsc, oneshot};

/// Mailbox for the actual task that manages the swarm.
#[derive(Debug, Clone)]
pub struct Manager {
    pub swarm_sender: mpsc::Sender<swarm::InEvent>,
    pub protocol_sender: mpsc::Sender<swarm::ProtocolInEvent>,
}

impl Manager {
    pub async fn dial(&self, addr: &Multiaddr) -> Result<(), DialError> {
        let (tx, rx) = oneshot::channel();
        self.swarm_sender
            .send(swarm::InEvent::Dial(addr.clone(), tx))
            .await
            .unwrap();
        rx.await.unwrap()
    }
    pub fn blocking_dial(&self, addr: &Multiaddr) -> Result<(), DialError> {
        let (tx, rx) = oneshot::channel();
        self.swarm_sender
            .blocking_send(swarm::InEvent::Dial(addr.clone(), tx))
            .unwrap();
        rx.blocking_recv().unwrap()
    }
    pub async fn listen(
        &self,
        addr: &Multiaddr,
    ) -> Result<ListenerId, TransportError<std::io::Error>> {
        let (tx, rx) = oneshot::channel();
        self.swarm_sender
            .send(swarm::InEvent::Listen(addr.clone(), tx))
            .await
            .unwrap();
        rx.await.unwrap()
    }
    pub fn blocking_listen(
        &self,
        addr: &Multiaddr,
    ) -> Result<ListenerId, TransportError<std::io::Error>> {
        let (tx, rx) = oneshot::channel();
        self.swarm_sender
            .blocking_send(swarm::InEvent::Listen(addr.clone(), tx))
            .unwrap();
        rx.blocking_recv().unwrap()
    }
    pub async fn remote_exec(
        &self,
        peer: PeerId,
        op: tethering::subprotocols::exec::Op,
    ) -> Result<
        tethering::subprotocols::exec::result::OpResult,
        tethering::subprotocols::exec::result::HandleError,
    > {
        let (result_tx, result_rx) = oneshot::channel();
        let (handle_tx, handle_rx) = oneshot::channel();
        let ev = ProtocolInEvent::Tethering(tethering::InEvent::RemoteExec(
            peer, op, handle_tx, result_tx,
        ));
        self.protocol_sender.send(ev).await.unwrap();
        match handle_rx.await {
            Ok(tethering::subprotocols::exec::result::HandleResult::Error(e)) => Err(e),
            Err(_) => panic!("Unexpected sender dropped."),
            _ => Ok(result_rx.await.unwrap()),
        }
    }
    pub fn blocking_remote_exec(
        &self,
        peer: PeerId,
        op: tethering::subprotocols::exec::Op,
    ) -> Result<
        tethering::subprotocols::exec::result::OpResult,
        tethering::subprotocols::exec::result::HandleError,
    > {
        let (result_tx, result_rx) = oneshot::channel();
        let (handle_tx, handle_rx) = oneshot::channel();
        let ev = ProtocolInEvent::Tethering(tethering::InEvent::RemoteExec(
            peer, op, handle_tx, result_tx,
        ));
        self.protocol_sender.blocking_send(ev).unwrap();
        if let tethering::subprotocols::exec::result::HandleResult::Error(e) =
            handle_rx.blocking_recv().unwrap()
        {
            return Err(e);
        }
        Ok(result_rx.blocking_recv().unwrap())
    }
    pub async fn tethering_local_exec(
        &self,
        op: tethering::TetheringOp,
    ) -> Result<tethering::TetheringOpResult, tethering::subprotocols::exec::result::HandleError>
    {
        let (tx, rx) = oneshot::channel();
        let ev = tethering::InEvent::LocalExec(op, tx);
        self.protocol_sender
            .send(ProtocolInEvent::Tethering(ev))
            .await
            .unwrap();
        Ok(rx.await.unwrap())
    }
    pub fn blocking_tethering_local_exec(
        &self,
        op: tethering::TetheringOp,
    ) -> tethering::TetheringOpResult {
        let (tx, rx) = oneshot::channel();
        let ev = tethering::InEvent::LocalExec(op, tx);
        self.protocol_sender
            .blocking_send(ProtocolInEvent::Tethering(ev))
            .unwrap();
        rx.blocking_recv().unwrap()
    }
    pub async fn messaging_exec(&self, op: messaging::Op) -> messaging::OpResult {
        let (tx, rx) = oneshot::channel();
        let ev = ProtocolInEvent::Messaging(messaging::InEvent::new(op, tx));
        self.protocol_sender.send(ev).await.unwrap();
        rx.await.unwrap()
    }
    pub fn blocking_messaging_exec(&self, op: messaging::Op) -> messaging::OpResult {
        let (tx, rx) = oneshot::channel();
        let ev = ProtocolInEvent::Messaging(messaging::InEvent::new(op, tx));
        self.protocol_sender.blocking_send(ev).unwrap();
        rx.blocking_recv().unwrap()
    }
}
