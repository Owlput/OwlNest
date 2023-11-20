use owlnest_macro::{behaviour_select, generate_event_select};

behaviour_select! {
    messaging: Messaging,
    tethering: Tethering,
    relay_server: RelayServer,
    relay_client: RelayClient,
    relay_ext:RelayExt,
    kad: Kad,
    identify: Identify,
    mdns: Mdns,
    dcutr: Dcutr,
}

// use libp2p_swarm::NetworkBehaviour;
// use super::super::protocols::*;
// #[derive(NetworkBehaviour)]
// pub struct Behaviour{
//     pub messaging: messaging::Behaviour,
//     pub tethering: tethering::Behaviour,
//     pub relay_server: relay_server::Behaviour,
//     pub relay_client: relay_client::Behaviour,
//     pub relay_ext:relay_ext::Behaviour,
//     pub kad: kad::Behaviour,
//     pub identify: identify::Behaviour,
//     pub mdns: mdns::Behaviour,
// }
