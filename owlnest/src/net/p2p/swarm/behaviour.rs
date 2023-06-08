use crate::{behaviour_select, event_bus::ListenedEvent, net::p2p::{SwarmConfig, identity::IdentityUnion}};
use libp2p::core::{transport::{Boxed, OrTransport, Transport}, muxing::StreamMuxerBox};
use super::SwarmTransport;
use crate::net::p2p::protocols::*;

behaviour_select! {
    messaging=>Messaging:messaging::Behaviour,
    tethering=>Tethering:tethering::Behaviour,
    relay_server=>RelayServer:relay_server::Behaviour,
    relay_client=> RelayClient:relay_client::Behaviour,
     kad=>Kad:kad::Behaviour,
    identify=>Identify:identify::Behaviour,
    mdns=> Mdns:mdns::Behaviour,

}
impl Behaviour {
    pub fn new(config: SwarmConfig) -> (Self, SwarmTransport) {
        use libp2p::kad::store::MemoryStore;
        let ident = config.ident().clone();
        let kad_store = MemoryStore::new(ident.get_peer_id());
        let (relayed_transport, relay_client) = libp2p::relay::client::new(ident.get_peer_id());
        let behav = Self {
            kad: kad::Behaviour::new(ident.get_peer_id(), kad_store),
            mdns: mdns::Behaviour::new(config.mdns, ident.get_peer_id()).unwrap(),
            identify: identify::Behaviour::new(config.identify),
            messaging: messaging::Behaviour::new(config.messaging),
            tethering: tethering::Behaviour::new(config.tethering),
            relay_server: libp2p::relay::Behaviour::new(
                config.local_ident.get_peer_id(),
                config.relay_server,
            ),
            relay_client,
        };
        let transport = upgrade_transport(
            OrTransport::new(libp2p::tcp::tokio::Transport::default(), relayed_transport).boxed(),
            &ident,
        );
        (behav, transport)
    }
}
impl Into<ListenedEvent> for ToSwarmEvent{
    fn into(self)->ListenedEvent{
        ListenedEvent::new(format!("swarmEvent:{:?}",self),self)
    }
}

use futures::{AsyncRead, AsyncWrite};
fn upgrade_transport<StreamSink>(
    transport: Boxed<StreamSink>,
    ident: &IdentityUnion,
) -> Boxed<(PeerId, StreamMuxerBox)>
where
    StreamSink: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    transport
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(libp2p::noise::Config::new(&ident.get_keypair()).unwrap())
        .multiplex(libp2p::yamux::Config::default())
        .boxed()
}
