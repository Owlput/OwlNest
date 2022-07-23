use tonic::transport::Server;

use tokio::{sync::mpsc::Sender, task::JoinHandle};
use tracing::{error, info};

use super::protos::helloworld::{hello_world::greeter_server::GreeterServer, MyGreeter};

pub async fn startup(addr: String, register: Sender<String>) -> JoinHandle<()> {
    tokio::spawn(async move {
        register.send(format!("tcp_sock {}", &addr)).await.unwrap();
        drop(register);//Drop it because it's no longer needed
        let addr = addr.parse().unwrap();
        let greeter = MyGreeter::default();
        info!("gRPC server started on {:?}",addr);
        if let Err(e) = Server::builder()
            .add_service(GreeterServer::new(greeter))
            .serve(addr)
            .await
        {
            error!("{}", e);
        };
    })
}
