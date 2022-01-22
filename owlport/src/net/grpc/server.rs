// use hyper::{service::Service, Body};
// use std::{net::SocketAddr, error::Error};
// use tonic::{
//     body::BoxBody,
//     transport::{NamedService, Server},
//     Request, Response, Status,
// };

// use tokio::task::JoinHandle;

// pub struct GrpcServer<T> {
//     sock: SocketAddr,
//     services: Vec<T>,
// }

// impl<S> GrpcServer<S>
// where
//     S: Service<Request<Body>, Response = Response<BoxBody>> + NamedService + Clone + Send + 'static + hyper::service::Service<hyper::Request<hyper::Body>>,
//     S::Future: Send + 'static,
//     S::Error: Into<Box<dyn Error + Send + Sync>> + Send,
// {
//     pub fn new(sock: String, services: Vec<S>) -> Self {
//         GrpcServer {
//             sock: sock.parse().unwrap(),
//             services,
//         }
//     }

//     pub fn startup(self) -> JoinHandle<()> {
//         tokio::spawn(async move {
//             let server = Server::builder();
//             for svc in self.services {
//                 server.add_service(svc);
//             }
//         })
//     }
// }
