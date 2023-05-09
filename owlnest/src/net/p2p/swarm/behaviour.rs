use super::*;

use libp2p::core::transport::{Boxed, OrTransport, Transport};
use libp2p::core::upgrade;
use libp2p::swarm::NetworkBehaviour;

/// Combined behaviour for libp2p swarm.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
pub struct Behaviour {
    pub messaging: messaging::Behaviour,
    pub tethering: tethering::Behaviour,
    pub relay_server: relay_server::Behaviour,
    pub relay_client: relay_client::Behaviour,
    pub kad: kad::Behaviour,
    pub identify: identify::Behaviour,
    pub mdns: mdns::Behaviour,
}
impl Behaviour {
    pub fn new(config: SwarmConfig) -> (Self, SwarmTransport) {
        use libp2p::kad::store::MemoryStore;

        let ident = config.ident();
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

use futures::{AsyncRead, AsyncWrite};
fn upgrade_transport<StreamSink>(
    transport: Boxed<StreamSink>,
    ident: &IdentityUnion,
) -> Boxed<(PeerId, StreamMuxerBox)>
where
    StreamSink: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    transport
        .upgrade(upgrade::Version::V1)
        .authenticate(libp2p::noise::Config::new(&ident.get_keypair()).unwrap())
        .multiplex(libp2p::yamux::Config::default())
        .boxed()
}
