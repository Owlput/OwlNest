use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tracing::info;

use crate::net::http::routes::main_entry;

pub async fn startup(addr: String, register: Sender<String>) -> JoinHandle<()> {
    info!("An REST server has been registered at socket {}", &addr);
    register.send(format!("tcp_sock {}", &addr)).await.unwrap();
    drop(register); //Dropping the register because it's no longer needed
                    //Spawn a tokio task to handle the server
    tokio::spawn(async move {
        let main_entry_route = main_entry::new();
        axum::Server::bind(&addr.parse().unwrap())
            .serve(main_entry_route.into_make_service())
            .await
            .unwrap();
    })
}
