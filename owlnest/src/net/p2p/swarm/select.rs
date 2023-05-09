use libp2p::swarm::ConnectionHandler;
use libp2p::swarm::NetworkBehaviour;

use crate::{generate_in_event_select, generate_out_event_select, generate_inbound_upgrade_select};
use crate::net::p2p::protocols::*;


generate_in_event_select!(Messaging:messaging::InEvent,Tethering:tethering::InEvent);
generate_out_event_select!(Messaging:messaging::OutEvent,Tethering:tethering::OutEvent);

pub struct ConnectionHandlerSelect{

}
generate_inbound_upgrade_select!(tethering=>Tethering:<tethering::Behaviour as NetworkBehaviour>::ConnectionHandler);