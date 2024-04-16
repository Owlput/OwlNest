pub use libp2p::upnp::tokio::Behaviour;
pub use libp2p::upnp::Event as OutEvent;
use tracing::info;

/// Log events to tracing
pub(crate) fn ev_dispatch(ev: &OutEvent) {
    match ev{
        OutEvent::NewExternalAddr(addr) => info!("New external address: {addr}"),
        OutEvent::ExpiredExternalAddr(addr) => info!("Renewal for external address {addr} failed"),
        OutEvent::GatewayNotFound => info!("Gateway not found or does not support UPnP"),
        OutEvent::NonRoutableGateway => info!("Gateway is not exposed directly to the public Internet, i.e. it itself has a private IP address."),
    }
}
