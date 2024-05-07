use owlnest::{
    net::p2p::{identity::IdentityUnion, swarm::manager::Manager, SwarmConfig},
    *,
};
use std::{
    io::{Read, Write},
    path::Path,
    sync::Mutex,
};
use tokio::sync::Notify;
use tracing::Level;
use tracing_log::LogTracer;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, Layer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match read_config("./owlnest_config.toml") {
        Ok(v) => v,
        Err(e) => {
            println!("Cannot read config file, exiting");
            return Err(Box::new(e));
        }
    };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    if let Err(e) = setup_logging() {
        println!("Cannot log to file: {}", e);
    };
    let ident = if !config.swarm.identity_path.is_empty() {
        get_ident(&config.swarm.identity_path)?
    } else {
        IdentityUnion::generate()
    };
    let mgr = setup_peer(ident.clone(), config, rt.handle().clone());
    let shutdown_notifier = std::sync::Arc::new(Notify::const_new());
    cli::setup_interactive_shell(ident.clone(), mgr, shutdown_notifier.clone());
    rt.block_on(shutdown_notifier.notified());
    Ok(())
}

pub fn setup_peer(
    ident: IdentityUnion,
    config: SwarmConfig,
    executor: tokio::runtime::Handle,
) -> Manager {
    let _guard = executor.enter();
    net::p2p::swarm::Builder::new(config).build(ident, executor)
}

pub(crate) fn setup_logging() -> Result<(), std::io::Error> {
    let time = chrono::Local::now().timestamp_micros();
    let log_file_handle = match std::fs::create_dir("./logs") {
        Ok(_) => std::fs::File::create(format!("./logs/{}.log", time))?,
        Err(e) => {
            let error = format!("{:?}", e);
            if error.contains("AlreadyExists") {
                std::fs::File::create(format!("./logs/{}.log", time))?
            } else {
                std::fs::File::create(format!("{}.log", time))?
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
    if let Err(e) = LogTracer::init() {
        println!("Cannot read logs from `log` source: {}", e)
    };
    Ok(())
}

fn get_ident(path: &String) -> Result<IdentityUnion, std::io::Error> {
    use tracing::warn;
    match IdentityUnion::from_file_protobuf_encoding(path) {
        Ok(ident) => Ok(ident),
        Err(e) => {
            warn!("Failed to read keypair: {:?}", e);
            let ident = IdentityUnion::generate();
            ident.export_keypair(path)?;
            Ok(ident)
        }
    }
}
fn read_config(path: impl AsRef<Path>) -> Result<SwarmConfig, std::io::Error> {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .read(true)
        .create(false)
        .open(path.as_ref())
    {
        let mut config_text = String::new();
        if let Err(e) = f.read_to_string(&mut config_text) {
            println!("Cannot read config file: {}", e)
        };
        match toml::from_str(&config_text) {
            Ok(v) => return Ok(v),
            Err(e) => println!("Cannot parse config file: {}", e),
        }
    }
    println!("Cannot load config, trying to generate example config file...");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("./owlnest_config.toml.example")
    {
        let default_config = SwarmConfig::default();
        if let Err(e) = f.write(
            toml::to_string_pretty(&default_config)
                .expect("Serialization to succeed")
                .as_bytes(),
        ) {
            println!("Cannot write example config: {}", e);
        }
        let _ = f.flush();
        drop(f);
        println!("Example config file has been generated.")
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "Cannot read configuration file",
    ))
}
