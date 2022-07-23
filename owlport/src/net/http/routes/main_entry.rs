use axum::{routing::get, Router};
use hyper::{Body, Request};

pub fn new() -> Router {
    let router = Router::new().route("/", get(hello_world));
    router
}
async fn hello_world(request: Request<Body>) -> String {
    format!("{:#?}", request.headers())
}
