use std::fmt::Display;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::{sync::mpsc::Receiver, time::sleep};
use tracing::{info, warn};

pub struct ResourceRegistry {
    channel: Receiver<String>,
}
impl ResourceRegistry {
    pub fn new(channel: Receiver<String>) -> Self {
        ResourceRegistry { channel }
    }
    pub fn startup(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            struct Registry {
                tcp_sock: Vec<SocketAddr>,
                udp_sock: Vec<SocketAddr>,
            }
            let mut registry = Registry {
                tcp_sock: vec![],
                udp_sock: vec![],
            };
            info!("Resource Registry has been started");
            loop {
                match self.channel.recv().await {
                    Some(msg) => {
                        info!("Resource regsitration request received: {}", &msg);
                        let req = msg.split_whitespace().collect::<Vec<_>>();
                        if req[0] == "ResReg" {
                            match req[1] {
                                "tcp_sock" => registry
                                    .tcp_sock
                                    .push(SocketAddr::from_str(req[1]).unwrap()),
                                "udp_sock" => registry
                                    .udp_sock
                                    .push(SocketAddr::from_str(req[1]).unwrap()),
                                _ => {}
                            }
                        }
                    }
                    None => {
                        warn!("All resource registration handles have been dropped");
                        sleep(Duration::from_secs(5)).await
                    }
                }
            }
        })
    }
}
