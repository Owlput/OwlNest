use libp2p::swarm::NetworkBehaviour;

pub mod messaging;
pub mod tethering;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = OutEvent)]
pub struct Behaviour{
    pub messaging:messaging::Behaviour,
    pub tethering:tethering::Behaviour
}
impl Behaviour{
    pub fn send_message(&mut self,msg:messaging::Message){
        self.messaging.push_event(messaging::InEvent::PostMessage(msg))
    }
    pub fn tether_op_exec(&mut self,op:tethering::InEvent){
        self.tethering.push_op(op)
    }
}


#[derive(Debug)]
pub enum OutEvent{
    Messaging(messaging::OutEvent),
    Tethering(tethering::OutEvent)
}

impl From<messaging::OutEvent> for OutEvent{
    fn from(value: messaging::OutEvent) -> Self {
            OutEvent::Messaging(value)
    }
}
impl From<tethering::OutEvent> for OutEvent{
    fn from(value: tethering::OutEvent) -> Self {
            OutEvent::Tethering(value)
    }
}