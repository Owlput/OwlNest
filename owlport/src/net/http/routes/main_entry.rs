use axum::{routing::get, Router};

pub fn new() -> Router {
    let router = Router::new().route("/", get(hello_world));
    router
}
 async fn hello_world()->String{
    "Hello World".into()
 }