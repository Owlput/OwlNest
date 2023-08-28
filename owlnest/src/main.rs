use std::sync::Mutex;

use owlnest::{
    net::p2p::{identity::IdentityUnion, protocols},
    *, event_bus::{Handle, bus::*},
};
use tracing::{Level, info};

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
    setup_logging(&rt);
    let ident = get_ident();
    let (ev_bus_handle, ev_tap) = setup_ev_bus(rt.handle());
    rt.block_on(setup_peer(ident, &ev_bus_handle,ev_tap));
    let _ = rt.block_on(tokio::signal::ctrl_c());
    std::thread::sleep(std::time::Duration::from_secs(10));
}

async fn setup_peer(ident: IdentityUnion, event_bus_handle:&Handle, ev_tap:EventTap) {
    let swarm_config = net::p2p::SwarmConfig {
        local_ident: ident.clone(),
        kad: protocols::kad::Config::default(),
        identify: protocols::identify::Config::new("/owlnest/0.0.1".into(), ident.get_pubkey()),
        mdns: protocols::mdns::Config::default(),
        messaging: protocols::messaging::Config::default(),
        tethering: protocols::tethering::Config,
        relay_server: protocols::relay_server::Config::default(),
    };
    let mgr = net::p2p::swarm::Builder::new(swarm_config).build(8,event_bus_handle.clone(),ev_tap);
    cli::setup_interactive_shell(ident, mgr);
}

fn setup_logging(rt:&tokio::runtime::Runtime) {
    
    let time = chrono::Local::now().timestamp_micros();
    #[cfg(target_os = "linux")]
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
    #[cfg(target_os = "windows")]
    let log_file_handle = match std::fs::create_dir(".\\logs") {
        Ok(_) => std::fs::File::create(format!(".\\logs\\{}.log", time)).unwrap(),
        Err(e) => {
            let error = format!("{:?}", e);
            if error.contains("AlreadyExists") {
                std::fs::File::create(format!(".\\logs\\{}.log", time)).unwrap()
            } else {
                panic!("{}",e)
            }
        }
    };
    let _ = rt.enter();
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