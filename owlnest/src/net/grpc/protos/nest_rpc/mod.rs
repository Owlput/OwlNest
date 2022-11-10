use tonic::{Request, Response, Status};
use tracing::info;

pub mod nest_rpc {
    tonic::include_proto!("nest_rpc");
}

#[derive(Debug, Default)]
pub struct NestRpcServer;

#[tonic::async_trait]
impl nest_rpc::nest_rpc_server::NestRpc for NestRpcServer {
    async fn hb(
        &self,
        request: Request<nest_rpc::HbRequest>,
    ) -> Result<Response<nest_rpc::HbReply>, Status> {
        info!("Got Hb request: {:?}", request);
        let reply = nest_rpc::HbReply {
            rand: rand::random::<i64>(),
        };
        Ok(Response::new(reply))
    }
}
