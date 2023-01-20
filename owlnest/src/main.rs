use owlnest::*;

use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let (resource_tx, _resource_rx) = mpsc::channel(16);
    net::http::server::startup("127.0.0.1:20002".into(), resource_tx.clone()).await;
    let _swarm = net::p2p::setup_swarm();
    let _resource_handle_remain = resource_tx.clone(); //Make the register happy in case of None
    tokio::signal::ctrl_c().await.unwrap();
}
