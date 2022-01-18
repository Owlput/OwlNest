#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
pub mod control;
use control::*;
use tokio::{sync::mpsc, time::*};

#[tokio::main]
async fn main() {
    let debug_adapters_enabled = vec![
        DebugAdapter::Console,
        DebugAdapter::FileLog(filelog::FilelogAdapter::new("".into())),
    ];

    let mut resource_registry = resource_registry::ResourceRegistry::new();
    let (debug_tx, debug_rx) = mpsc::channel(16);
    let debug_listener = debug_listener::DebugListener::new(debug_adapters_enabled,debug_rx);
    resource_registry.debug = true;
    let task1 = tokio::spawn(async move {
        println!("Raw:sender created");
        for i in 1..10 {
            match debug_tx.send(format!("Hello number {}", i)).await {
                Ok(_) => println!("Raw:Successfully sent"),
                Err(e) => println!("Err:{}", e),
            }
            sleep(Duration::from_secs(1)).await;
        }
        println!("Raw:sender dropped");
    });
    debug_listener.startup().await;
    task1.await.unwrap();
}
