mod net;
mod utils;

use tokio::sync::mpsc;
use tonic::transport::Identity;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Program started");
    let ident = read_ident().await.unwrap();
    let (resource_tx, _resource_rx) = mpsc::channel(16);
    net::http::server::startup("127.0.0.1:20002".into(), resource_tx.clone()).await;
    net::grpc::server::startup("127.0.0.1:20003".into(), resource_tx.clone(),ident).await;
    let _swarm = net::p2p::setup_swarm();
    let _resource_handle_remain = resource_tx.clone(); //Make the register happy in case of None
    tokio::signal::ctrl_c().await.unwrap();
}

async fn read_ident()->Result<Identity,Box<dyn std::error::Error>>{
    let cert = tokio::fs::read("certs/owlnest_test_cert.pem").await?;
    let key = tokio::fs::read("certs/owlnest_test_key_decrypted.pem").await?;
    Ok(Identity::from_pem(cert, key))

}