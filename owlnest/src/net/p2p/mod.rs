pub mod swarm;
pub mod protocols;
pub mod identity;

pub use swarm::*;
pub use protocols::{BehaviourEvent,OutEvent};
use tokio::sync::mpsc;

pub fn setup_swarm()->(Libp2pSwarmManager,mpsc::Receiver<BehaviourEvent>){
    let identity = identity::IdentityUnion::generate();
    Libp2pSwarmBuilder::default().build(libp2p::tokio_development_transport(identity.get_keypair()).unwrap(), identity, 8)
}