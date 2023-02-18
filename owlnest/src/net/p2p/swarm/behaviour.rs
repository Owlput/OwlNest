use super::*;
use futures::{AsyncRead, AsyncWrite};
use libp2p::core::transport::{OrTransport, Boxed,Transport};
use libp2p::core::upgrade;
use libp2p::swarm::NetworkBehaviour;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = OutEvent)]
pub struct Behaviour {
    #[cfg(feature = "messaging")]
    pub messaging: messaging::Behaviour,
    #[cfg(feature = "tethering")]
    pub tethering: tethering::Behaviour,
    #[cfg(feature = "relay-server")]
    pub relay_server: relay_server::Behaviour,
    #[cfg(feature = "relay-client")]
    pub relay_client: relay_client::Behaviour,
    pub keep_alive: libp2p::swarm::keep_alive::Behaviour,
}
impl Behaviour {
    #[cfg(not(feature = "relay-client"))]
    pub fn new(config: SwarmConfig) -> (Self, SwarmTransport) {
        let ident = config.ident();
        let behav = Self {
            #[cfg(feature = "messaging")]
            messaging: messaging::Behaviour::new(config.messaging),
            #[cfg(feature = "tethering")]
            tethering: tethering::Behaviour::new(config.tethering),
            #[cfg(feature = "relay-server")]
            relay_server: libp2p::relay::v2::relay::Relay::new(
                ident.get_peer_id(),
                config.relay_server,
            ),
            keep_alive: libp2p::swarm::keep_alive::Behaviour::default(),
        };
        let transport = libp2p::tokio_development_transport(ident.get_keypair()).unwrap();
        (behav, transport)
    }
    #[cfg(feature = "relay-client")]
    pub fn new(config: SwarmConfig) -> (Self, super::SwarmTransport) {
        let ident = config.ident();
        let (relayed_transport, relay_client) =
            libp2p::relay::v2::client::Client::new_transport_and_behaviour(ident.get_peer_id());
        let behav = Self {
            #[cfg(feature = "messaging")]
            messaging: messaging::Behaviour::new(config.messaging),
            #[cfg(feature = "tethering")]
            tethering: tethering::Behaviour::new(config.tethering),
            #[cfg(feature = "relay-server")]
            relay_server: libp2p::relay::v2::relay::Relay::new(
                config.local_ident.get_peer_id(),
                config.relay_server,
            ),
            #[cfg(feature = "relay-client")]
            relay_client,
            keep_alive: libp2p::swarm::keep_alive::Behaviour::default(),
        };
        let transport = upgrade_transport(
            OrTransport::new(libp2p::tcp::tokio::Transport::default(), relayed_transport).boxed(),
            &ident,
        );
        (behav, transport)
    }
}

impl Behaviour {
    #[cfg(feature = "messaging")]
    pub fn message_op_exec(&mut self, op: messaging::InEvent) {
        self.messaging.push_event(op)
    }
    #[cfg(feature = "tethering")]
    pub fn tether_op_exec(&mut self, op: tethering::InEvent) {
        self.tethering.push_op(op)
    }
}

#[cfg(feature="relay-client")]
fn upgrade_transport<StreamSink>(
    transport: Boxed<StreamSink>,
    ident: &IdentityUnion,
) -> Boxed<(PeerId, StreamMuxerBox)>
where
    StreamSink: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    transport
        .upgrade(upgrade::Version::V1)
        .authenticate(libp2p::noise::NoiseAuthenticated::xx(&ident.get_keypair()).unwrap())
        .multiplex(libp2p::yamux::YamuxConfig::default())
        .boxed()
}
