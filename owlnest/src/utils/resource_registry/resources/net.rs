use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug,Hash,PartialEq,Eq,Clone)]
pub enum ConnType{
    TcpV4{
        addr:Ipv4Addr,
        port:u16
    },
    TcpV6{
        addr:Ipv6Addr,
        port:u16
    },
    UdpV4{
        addr:Ipv4Addr,
        port:u16
    },
    UdpV6{
        addr:Ipv6Addr,
        port:u16
    }
}

