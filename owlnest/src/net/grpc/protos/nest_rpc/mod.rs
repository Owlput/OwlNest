use tonic::{Request, Response, Status};
use tracing::info;
pub mod nest_status;

pub mod generated {
    tonic::include_proto!("nest_rpc");
}

#[derive(Debug, Default)]
pub struct RequestHandler;

#[tonic::async_trait]
impl generated::nest_rpc_server::NestRpc for RequestHandler {
    async fn hb(
        &self,
        request: Request<generated::HbRequest>,
    ) -> Result<Response<generated::HbReply>, Status> {
        info!("Got Hb request: {:?}", request);
        let reply = generated::HbReply {
            rand: rand::random::<i64>(),
        };
        Ok(Response::new(reply))
    }
}
