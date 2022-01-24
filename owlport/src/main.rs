#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
mod net;
mod utils;
use net::*;
use tokio::{join, sync::mpsc, time::*};
use tracing::info;
use utils::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Program started");
    let (resource_tx, resource_rx) = mpsc::channel(16);
    let resource_registry = resource_registry::ResourceRegistry::new(resource_rx);
    net::rest::server::startup("127.0.0.1:10000".into(), resource_tx.clone()).await;
    resource_registry.startup();
    let _resource_handle_remain  = resource_tx.clone();
    tokio::signal::ctrl_c().await.unwrap();
}
