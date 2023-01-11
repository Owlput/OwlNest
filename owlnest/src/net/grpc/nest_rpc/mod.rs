use owlcom::libp2p::swarm::{Libp2pSwarmOp, Multiaddr};
use tokio::sync::{mpsc, oneshot};
use tonic::{Code, Request, Response, Status};
use tracing::info;

pub mod generated {
    tonic::include_proto!("nest_rpc");
}

#[derive(Debug)]
pub struct RequestHandler {
    swarm_tx: mpsc::Sender<Libp2pSwarmOp>,
}

impl RequestHandler {
    pub fn new(swarm_tx: mpsc::Sender<Libp2pSwarmOp>) -> Self {
        Self { swarm_tx }
    }
}

#[tonic::async_trait]
impl generated::nest_rpc_server::NestRpc for RequestHandler {
    async fn heartbeat(
        &self,
        request: Request<generated::HeartbeatRequest>,
    ) -> Result<Response<generated::HeartbeatReply>, Status> {
        info!("Got Hb request: {:?}", request);
        let reply = generated::HeartbeatReply {
            rand: rand::random::<i64>(),
        };
        Ok(Response::new(reply))
    }
    async fn dial(
        &self,
        request: Request<generated::Multiaddr>,
    ) -> Result<Response<generated::Result>, Status> {
        let (callback_tx, callback_rx) = oneshot::channel();
        let op = match request.get_ref().multiaddr.parse::<Multiaddr>() {
            Ok(addr) => Libp2pSwarmOp::Dial { addr, callback_tx },
            Err(e) => {
                return Err(Status::new(
                    Code::InvalidArgument,
                    format!("Invalid Multiaddr {}", e),
                ))
            }
        };

        match self.swarm_tx.send(op).await {
            Ok(_) => match callback_rx.await {
                Ok(_) => Ok(Response::new(generated::Result {
                    is_ok: true,
                    reason: None,
                })),
                Err(e) => Err(Status::new(
                    Code::Internal,
                    format!("Callback failed with {}", e),
                )),
            },
            Err(e) => Err(Status::new(
                Code::Internal,
                format!("Dispatch dial operation failed with {}", e),
            )),
        }
    }
    async fn listen(
        &self,
        request: Request<generated::Multiaddr>,
    ) -> Result<Response<generated::Result>, Status> {
        let (callback_tx, callback_rx) = oneshot::channel();
        let op = match request.get_ref().multiaddr.parse::<Multiaddr>() {
            Ok(addr) => Libp2pSwarmOp::Listen { addr, callback_tx },
            Err(e) => {
                return Err(Status::new(
                    Code::InvalidArgument,
                    format!("Invalid Multiaddr {}", e),
                ))
            }
        };
        match self.swarm_tx.send(op).await {
            Ok(_) => match callback_rx.await {
                Ok(_) => Ok(Response::new(generated::Result {
                    is_ok: true,
                    reason: None,
                })),
                Err(e) => Err(Status::new(
                    Code::Internal,
                    format!("Callback failed with {}", e),
                )),
            },
            Err(e) => Err(Status::new(
                Code::Internal,
                format!("Dispatch listen operation failed with {}", e),
            )),
        }
    }
    async fn lookup_peer(
        &self,
        _request: Request<generated::PeerId>,
    ) -> Result<Response<generated::Result>, Status> {
        info!("Request for looking up peers received.");
        Err(Status::new(Code::Unimplemented, "Not implemented"))
    }
    async fn list_connected_peers(
        &self,
        _request: Request<generated::ListRequest>,
    ) -> Result<Response<generated::PeersList>, Status> {
        info!("Request for listing connected peers received.");
        Err(Status::new(Code::Unimplemented, "Not implemented"))
    }
    async fn list_trusted_peers(
        &self,
        _request: Request<generated::ListRequest>,
    ) -> Result<Response<generated::PeersList>, Status> {
        info!("Request for listing trusted peers received");
        Err(Status::new(Code::Unimplemented, "Not implemented"))
    }
}
