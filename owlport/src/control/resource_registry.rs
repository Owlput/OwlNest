use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ResourceRegistry {
    pub debug: bool,
    pub tcp_sock: Vec<std::net::SocketAddrV4>,
    pub udp_sock: Vec<std::net::SocketAddrV4>,
}
impl ResourceRegistry {
    pub fn new() -> Self {
        ResourceRegistry {
            debug: false,
            tcp_sock: vec![],
            udp_sock: vec![],
        }
    }
}

impl Display for ResourceRegistry{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f,"{}",serde_json::to_string(self).unwrap()) 
}
}