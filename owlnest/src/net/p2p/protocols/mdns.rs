pub use libp2p::mdns::tokio::Behaviour;
pub type OutEvent = libp2p::mdns::Event;
pub use libp2p::mdns::Config;

use crate::net::p2p::swarm::Swarm;

pub fn ev_dispatch(ev: OutEvent, swarm: &mut Swarm) {
    match ev {
        libp2p::mdns::Event::Discovered(list) => {
            for (peer,addr) in list{
                swarm.behaviour_mut().kad.add_address(&peer, addr);
            }
        },
        libp2p::mdns::Event::Expired(list) =>{
            for (peer,addr) in list{
                swarm.behaviour_mut().kad.remove_address(&peer, &addr);
            }
        },
    }
}
