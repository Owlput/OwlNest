use crate::behaviour_select;
use crate::net::p2p::protocols::*;
behaviour_select! {
    tethering=>Tethering:tethering::Behaviour,
    messaging=>Messaging:messaging::Behaviour,
}