use owlcom::libp2p::swarm::Libp2pSwarmOp;
//use libp2p::swarm::NetworkBehaviour;
use tonic::transport::{Identity, Server};

use tokio::{sync::mpsc::{Sender, self}, task::JoinHandle};
use tracing::{error, info};

use super::nest_rpc::generated::nest_rpc_server::NestRpcServer;
use crate::net::grpc::nest_rpc::RequestHandler;

pub async fn startup(addr: String, register: Sender<String>, _ident: Identity) -> (JoinHandle<()>,mpsc::Receiver<Libp2pSwarmOp>) {
    let (swarm_tx,swarm_rx) = mpsc::channel(8);
    let join_handle = tokio::spawn(async move {
        info!("gRPC server starting on {:?}", addr);
        register.send(format!("tcp_sock {}", &addr)).await.unwrap();
        drop(register); //Drop it because it's no longer needed
        let addr = addr.parse().unwrap();
        let greeter = RequestHandler::new(swarm_tx);
        let handle = Server::builder()
            /* .tls_config(ServerTlsConfig::new().identity(ident))
            .unwrap() */
            .add_service(NestRpcServer::new(greeter))
            .serve(addr);
        info!("gRPC server started on {:?}", addr);
        if let Err(e) = handle.await{
            error!("Error from gRPC server: {:?}",e);
        }
    });
    (join_handle,swarm_rx)
}
