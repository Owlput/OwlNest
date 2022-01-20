use std::fmt::Display;
use std::net::SocketAddr;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;

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
            loop {
                match self.channel.recv().await {
                    Some(msg) => {
                        println!("Raw:Resource Registration received");
                        let req = msg.split_whitespace().collect::<Vec<_>>();
                        if req[0] == "ResReg" {
                            match req[1] {
                                "tcp_sock" => registry.tcp_sock.push(SocketAddr::from_str(req[1]).unwrap()),
                                "udp_sock" => registry.udp_sock.push(SocketAddr::from_str(req[1]).unwrap()),
                                _ =>{}
                            }
                        }
                    }
                    None => {}
                }
            }
        })
    }
}
