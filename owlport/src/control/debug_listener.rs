pub mod adapters;
use tokio::{
    sync::mpsc::Receiver,
    time::{sleep, Duration},
};

pub use self::adapters::DebugAdapter;

pub struct DebugListener {
    adapters: Vec<DebugAdapter>,
    collector: Receiver<String>,
}
impl DebugListener {
    pub fn new(adapters: Vec<DebugAdapter>, collector: Receiver<String>) -> Self {
        DebugListener {
            adapters,
            collector,
        }
    }
    pub async fn startup(mut self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match self.collector.recv().await {
                    Some(msg) => {
                        for adapter in &self.adapters {
                            match adapter.send(&msg) {
                                Ok(_) => {}
                                Err(e) => println!("{}", e),
                            };
                        }
                    }
                    None => {
                        for adapter in &self.adapters {
                            match adapter.send(&String::from(
                                "WARN[DebugListener]:All senders have been dropped. Retry in 1s",
                            )) {
                                Ok(_) => {}
                                Err(e) => println!("{}", e),
                            };
                        }
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        })
    }
}
