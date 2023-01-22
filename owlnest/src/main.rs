use owlnest::{*, net::p2p::identity::IdentityUnion};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    setup_peer();
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(not(feature="relay-client"))]
fn setup_peer(){

    let local_ident = IdentityUnion::generate();
    let swarm_config = net::p2p::SwarmConfig::default();
    let transport = libp2p::tokio_development_transport(local_ident.get_keypair()).unwrap();
    let (mgr,out_bundle) = net::p2p::swarm::Builder::new(swarm_config).build(transport,8);
    let stdin = utils::stdin_event_bus::setup_bus(local_ident.get_peer_id());
    utils::stdin_event_bus::setup_distributor(stdin, mgr);
}
#[cfg(feature="relay-client")]
fn setup_peer(){
    use owlnest::net::p2p::relayed_swarm::OutEventBundle;
    

    let local_ident = IdentityUnion::generate();
    let swarm_config = net::p2p::SwarmConfig::default();
    let (mgr,out_bundle) = net::p2p::relayed_swarm::Builder::new(swarm_config).build_relayed(8);
    let stdin = utils::stdin_event_bus::setup_bus(local_ident.get_peer_id());
    utils::stdin_event_bus::setup_distributor_relayed(stdin, mgr);
    tokio::spawn(async move{
        let OutEventBundle{
            mut message_rx,..
        } = out_bundle;
        loop{
            if let Some(ev) = message_rx.recv().await{
                println!("{:#?}",ev)
            }
        }
    });
}