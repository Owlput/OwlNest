pub mod swarm {
    use crate::net::p2p::swarm::op::swarm;
    use tokio::sync::oneshot;
    #[derive(Debug)]
    pub struct InEvent {
        op: swarm::Op,
        callback: oneshot::Sender<swarm::OpResult>,
    }
    impl InEvent {
        pub fn new(op: swarm::Op, callback: oneshot::Sender<swarm::OpResult>) -> Self {
            InEvent { op, callback }
        }
        pub fn into_inner(self) -> (swarm::Op, oneshot::Sender<swarm::OpResult>) {
            (self.op, self.callback)
        }
    }
}

pub mod behaviour {

    use crate::net::p2p::protocols::*;
    #[derive(Debug)]
    pub(crate) enum InEvent {
        Messaging(messaging::InEvent),
        Tethering(tethering::InEvent),
        Kad(kad::InEvent),
    }

}
