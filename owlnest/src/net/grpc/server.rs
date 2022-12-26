use libp2p::swarm::NetworkBehaviour;
use tonic::transport::{Server, Identity, ServerTlsConfig};

use tokio::{sync::mpsc::Sender, task::JoinHandle};
use tracing::{error, info};

use crate::net::grpc::protos::nest_rpc::RequestHandler;
use super::protos::nest_rpc::generated::nest_rpc_server::NestRpcServer;

pub async fn startup(addr: String, register: Sender<String>,ident:Identity) -> JoinHandle<()> {
    tokio::spawn(async move {
        register.send(format!("tcp_sock {}", &addr)).await.unwrap();
        drop(register);//Drop it because it's no longer needed
        let addr = addr.parse().unwrap();
        let greeter = RequestHandler::default();
        info!("gRPC server started on {:?}",addr);
        if let Err(e) = Server::builder()
            .tls_config(ServerTlsConfig::new().identity(ident)).unwrap()
            .add_service(NestRpcServer::new(greeter))
            .serve(addr)
            .await
        {
            error!("{}", e);
        };
    })
}
