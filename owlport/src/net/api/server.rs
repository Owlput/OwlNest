use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use tokio::sync::mpsc::Sender;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

pub struct APIServer {
    sock_addr: SocketAddr,
    path: String,
}

impl APIServer {
    pub async fn new(addr:String,path:String,register:Sender<String>) ->Self{
        register.send(format!("tcp_sock {}",&addr)).await.unwrap();
        APIServer{
            sock_addr:addr.parse().unwrap(),
            path,
        }
        }
    pub fn startup(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            async fn hello_world(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
                Ok(Response::new("Hello, World".into()))
            }
            let make_svc =
                make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(hello_world)) });
            Server::bind(&self.sock_addr).serve(make_svc).await.unwrap();
        })
    }
}
