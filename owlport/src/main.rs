mod control;

use control::debug_listener::adapters::*;

#[tokio::main]
async fn main() {
    let debug_adapters_enabled = vec![DebugAdapter::Console,DebugAdapter::FileLog(filelog::FilelogAdapter::new("".into()))];
    let mut debug = control::debug_listener::DebugListener::new(debug_adapters_enabled);
    let log_test = tokio::spawn(async move{
        debug.push("Hi!");
    });
    log_test.await.unwrap()
}
