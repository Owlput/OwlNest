mod net;
mod utils;
use std::io::Read;

use hyper::body::Buf;
use tokio::sync::mpsc;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Program started");
    let (resource_tx, _resource_rx) = mpsc::channel(16);

    net::http::server::startup("127.0.0.1:20000".into(), resource_tx.clone()).await;
    net::grpc::server::startup("127.0.0.1:20001".into(), resource_tx.clone()).await;
    net::grpc::client::startup("".into());
    let _resource_handle_remain = resource_tx.clone(); //Make the register happy in case of None
    // tokio::spawn(async {
    //     let response = hyper::Client::builder()
    //         .build_http()
    //         .request(
    //             hyper::Request::builder()
    //                 .uri("http://localhost:5001/api/v0/bitswap/stat")
    //                 .method("POST")
    //                 .body(hyper::Body::empty())
    //                 .unwrap(),
    //         )
    //         .await
    //         .unwrap();
    //         let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    //         let json = String::from_utf8(body.to_vec()).unwrap();
    //         println!("{}",json)
    // });
    tokio::signal::ctrl_c().await.unwrap();
}
