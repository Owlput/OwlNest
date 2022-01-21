#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
mod utils;
mod net;
use utils::*;
use net::*;
use tokio::{join, sync::mpsc, time::*};

#[tokio::main]
async fn main() {
    let (resource_tx, resource_rx) = mpsc::channel(16);
    let resource_registry = resource_registry::ResourceRegistry::new(resource_rx);
    let api_server =
        net::api::server::APIServer::new("127.0.0.1:10000".into(), "".into(), resource_tx.clone())
            .await;
    resource_registry.startup();
    api_server.startup();
    tokio::signal::ctrl_c().await.unwrap();
}
