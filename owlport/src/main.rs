mod net;
mod utils;
use tokio::sync::mpsc;
use tracing::info;
use utils::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Program started");
    let (resource_tx, resource_rx) = mpsc::channel(16);
    resource_registry::startup(resource_rx);
    net::http::server::startup("127.0.0.1:10000".into(), resource_tx.clone()).await;
    net::grpc::server::startup("127.0.0.1:20000".into(), resource_tx.clone()).await;
    net::grpc::client::startup("".into());
    let _resource_handle_remain = resource_tx.clone(); //Make the register happy in case of None
    tokio::signal::ctrl_c().await.unwrap();
}
