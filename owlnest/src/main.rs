use owlnest::{*, net::p2p::{identity::IdentityUnion, swarm::OutEventBundle}};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    #[cfg(feature="full")]
    setup_peer();
    #[cfg(not(feature="full"))]
    println!("Won't start if not with full feature");
    let _ = tokio::signal::ctrl_c().await;
}

fn setup_peer(){
    let local_ident = IdentityUnion::generate();
    let swarm_config = net::p2p::SwarmConfig::default();
    let (mgr,out_bundle) = net::p2p::swarm::Builder::new(swarm_config).build(8);
    let stdin = utils::stdin_event_bus::setup_bus(local_ident.get_peer_id());
    utils::stdin_event_bus::setup_distributor(stdin, mgr);
    tokio::spawn(async move{
        let OutEventBundle{
            mut messaging_rx,..
        } = out_bundle;
        loop{
            if let Some(ev) = messaging_rx.recv().await{
                println!("{:#?}",ev)
            }
        }
    });
}