use std::sync::Mutex;

use owlnest::{
    net::p2p::{identity::IdentityUnion, protocols},
    *, event_bus::{EventBusHandle, bus::*},
};
use tracing::Level;

#[tokio::main]
async fn main() {
    setup_logging();
    let ident = get_ident();
    let (op_bundle,ev_bundle) = setup_event_listener();
    let ev_bus_handle = setup_ev_bus(op_bundle);
    setup_peer(ident, &ev_bus_handle,ev_bundle);
    let _ = tokio::signal::ctrl_c().await;
}

fn setup_peer(ident: IdentityUnion, event_bus_handle:&EventBusHandle, bus_ev_in: EvInBundle) {
    let swarm_config = net::p2p::SwarmConfig {
        local_ident: ident.clone(),
        kad: protocols::kad::Config::default(),
        identify: protocols::identify::Config::new("/owlnest/0.0.1".into(), ident.get_pubkey()),
        mdns: protocols::mdns::Config::default(),
        messaging: protocols::messaging::Config::default(),
        tethering: protocols::tethering::Config::default(),
        relay_server: protocols::relay_server::Config::default(),
    };
    let mgr = net::p2p::swarm::Builder::new(swarm_config).build(8,event_bus_handle.clone(),bus_ev_in);
    cli::setup_interactive_shell(ident.clone(), mgr.clone());
}

fn setup_logging() {
    let time = chrono::Local::now().to_rfc3339();
    let log_file_handle = match std::fs::create_dir("./logs") {
        Ok(_) => std::fs::File::create(format!("./logs/{}.log", time)).unwrap(),
        Err(e) => {
            let error = format!("{:?}", e);
            if error.contains("AlreadyExists") {
                std::fs::File::create(format!("./logs/{}.log", time)).unwrap()
            } else {
                std::fs::File::create(format!("{}.log", time)).unwrap()
            }
        }
    };
    tracing_subscriber::fmt::fmt()
        .with_max_level(Level::DEBUG)
        .with_ansi(false)
        .with_writer(Mutex::new(log_file_handle))
        .init();
}

fn get_ident() -> IdentityUnion {
    // match IdentityUnion::from_file_protobuf_encoding("./id.keypair"){
    //     Ok(ident) => ident,
    //     Err(e) => {
    //         warn!("Failed to read keypair: {:?}",e);
    //         let ident = IdentityUnion::generate();
    //         ident.export_keypair(".", "id").unwrap();
    //         ident
    //     },
    // }
    IdentityUnion::generate()
}

fn setup_event_listener()->(OpOutBundle,EvInBundle){
    let (messaging_ev_in,messaging_op_in) = protocols::messaging::event_listener::setup_event_listener();
    let (kad_ev_in,kad_op_in) = protocols::kad::event_listener::setup_event_listener(8);
    ((messaging_op_in,kad_op_in),(messaging_ev_in,kad_ev_in))
}