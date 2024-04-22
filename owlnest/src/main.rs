use owlnest::{
    net::p2p::{identity::IdentityUnion, protocols, swarm::manager::Manager},
    *,
};
use std::sync::Mutex;
use tokio::sync::Notify;
use tracing::Level;
use tracing_log::LogTracer;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, Layer};

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    setup_logging();
    let ident = get_ident();
    let mgr = setup_peer(ident.clone(), rt.handle().clone());
    let shutdown_notifier = std::sync::Arc::new(Notify::const_new());
    cli::setup_interactive_shell(ident.clone(), mgr, shutdown_notifier.clone());
    rt.block_on(shutdown_notifier.notified());
}

pub fn setup_peer(ident: IdentityUnion, executor: tokio::runtime::Handle) -> Manager {
    let _guard = executor.enter();
    let swarm_config = net::p2p::SwarmConfig {
        local_ident: ident.clone(),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-kad"))]
        kad: protocols::kad::Config::default(),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-identify"))]
        identify: protocols::identify::Config::new("/owlnest/0.0.1".into(), ident.get_pubkey()),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-mdns"))]
        mdns: protocols::mdns::Config::default(),
        #[cfg(any(feature = "owlnest-protocols", feature = "owlnest-messaging"))]
        messaging: protocols::messaging::Config::default(),
        #[cfg(any(feature = "libp2p-protocols", feature = "libp2p-relay-server"))]
        relay_server: protocols::relay_server::Config::default(),
    };
    net::p2p::swarm::Builder::new(swarm_config).build(8, executor)
}

pub(crate) fn setup_logging() {
    let time = chrono::Local::now().timestamp_micros();
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
    let filter = tracing_subscriber::filter::Targets::new().with_target("", Level::WARN);
    let layer = tracing_subscriber::fmt::Layer::default()
        .with_ansi(false)
        .with_writer(Mutex::new(log_file_handle))
        .with_filter(filter);
    let reg = tracing_subscriber::registry().with(layer);
    tracing::subscriber::set_global_default(reg).expect("you can only set global default once");
    LogTracer::init().unwrap()
}

fn get_ident() -> IdentityUnion {
    // use tracing::warn;
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
