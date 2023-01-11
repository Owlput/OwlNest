use owlcom::libp2p::swarm::Libp2pSwarmManager;

pub fn setup_swarm()->Libp2pSwarmManager{
    let (mgr,_event) = owlcom::libp2p::swarm::setup_ping_peer();
    mgr
}