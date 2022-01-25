use hello_world::greeter_client::GreeterClient;
use hello_world::HelloRequest;
use tokio::task::JoinHandle;
use tracing::info;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

pub fn startup(_addr:String)->JoinHandle<()>{
    tokio::spawn(async move{
        let mut client = GreeterClient::connect("http://127.0.0.1:20000").await.unwrap();
        let request = tonic::Request::new(HelloRequest{
            name:"Tonic".into(),
        });
        let response =client.say_hello(request).await;
        info!("GOT: {:?}",response);
        info!("Client exited")
    })
}