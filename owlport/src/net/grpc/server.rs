use tonic::{transport::Server, Request, Response, Status};

use hello_world::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

pub struct MyGreeter{}

#[tonic::async_trait]
impl Greeter for MyGreeter{

}