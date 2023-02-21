use owlnest::{
    net::p2p::{identity::IdentityUnion, protocols, swarm::OutEventBundle},
    *,
};
use tracing::Level;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::DEBUG)
        .init();
    setup_peer();
    let _ = tokio::signal::ctrl_c().await;
}

fn setup_peer() {
    let local_ident = IdentityUnion::generate();
    let swarm_config = net::p2p::SwarmConfig {
        local_ident: local_ident.clone(),
        kad: protocols::kad::Config::default(),
        identify:protocols::identify::Config::new("/owlnest/0.0.1".into(), local_ident.get_pubkey()),
        mdns:protocols::mdns::Config::default(),
        messaging: protocols::messaging::Config::default(),
        tethering: protocols::tethering::Config::default(),
        relay_server: protocols::relay_server::Config::default(),
    };
    let (mgr, out_bundle) = net::p2p::swarm::Builder::new(swarm_config).build(8);
    utils::stdin_event_bus::setup_bus(local_ident.get_peer_id(), mgr.clone());
    #[cfg(feature = "messaging")]
    tokio::spawn(async move {
        let OutEventBundle {
            mut messaging_rx, ..
        } = out_bundle;
        loop {
            if let Some(ev) = messaging_rx.recv().await {
                println!("{:#?}", ev)
            }
        }
    });
}
