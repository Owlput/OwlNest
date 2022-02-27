mod net;
mod utils;
mod standalone;
use tokio::sync::mpsc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Program started");
    let (resource_tx, _resource_rx) = mpsc::channel(16);

    net::http::server::startup("127.0.0.1:10000".into(), resource_tx.clone()).await;
    net::grpc::server::startup("127.0.0.1:20000".into(), resource_tx.clone()).await;
    net::grpc::client::startup("".into());
    let _resource_handle_remain = resource_tx.clone(); //Make the register happy in case of None
    tokio::signal::ctrl_c().await.unwrap();
}
