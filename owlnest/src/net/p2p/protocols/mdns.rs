use libp2p::{PeerId,Multiaddr};

pub use libp2p::mdns::tokio::Behaviour;
pub type OutEvent = libp2p::mdns::Event;
pub use libp2p::mdns::Config;
use tracing::info;

pub fn ev_dispatch(ev:OutEvent){
    match ev{
        libp2p::mdns::Event::Discovered(list) => info!("discovered peer {:?}",list.collect::<Vec<(PeerId,Multiaddr)>>()),
        libp2p::mdns::Event::Expired(list) => info!("expired peer {:?}",list.collect::<Vec<(PeerId,Multiaddr)>>()),
    }
}