pub use libp2p::dcutr::Behaviour;
use tracing::info;
pub type OutEvent = libp2p::dcutr::Event;

pub(crate) fn ev_dispatch(ev: &OutEvent) {
    info!("{:?}", ev)
}