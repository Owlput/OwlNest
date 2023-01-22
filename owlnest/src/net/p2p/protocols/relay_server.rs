use libp2p::relay::v2::relay;


pub fn ev_dispatch(ev: relay::Event) {
    match ev {
        relay::Event::ReservationReqAccepted {
            src_peer_id,
            renewed,
        } => println!(
            "Reservation from {} accepted, IsRenew:{}",
            src_peer_id, renewed
        ),
        relay::Event::ReservationReqAcceptFailed { src_peer_id, error } => println!(
            "Failed to accept reservation from {}, error:{}",
            src_peer_id, error
        ),
        relay::Event::ReservationReqDenied { src_peer_id } => {
            println!("Denied reservation from {}", src_peer_id)
        }
        relay::Event::ReservationReqDenyFailed { src_peer_id, error } => println!(
            "Failed to deny reservation from {}, error:{}",
            src_peer_id, error
        ),
        relay::Event::ReservationTimedOut { src_peer_id } => {
            println!("Reservation expired for source peer {}", src_peer_id)
        }
        relay::Event::CircuitReqReceiveFailed { src_peer_id, error } => println!(
            "Broken circuit request from {}, error:{}",
            src_peer_id, error
        ),
        relay::Event::CircuitReqDenied {
            src_peer_id,
            dst_peer_id,
        } => println!(
            "Circuit request from {} to peer {} denied",
            src_peer_id, dst_peer_id
        ),
        relay::Event::CircuitReqDenyFailed {
            src_peer_id,
            dst_peer_id,
            error,
        } => println!(
            "Failed to deny circuit request from {} to peer {}, error: {}",
            src_peer_id, dst_peer_id, error
        ),
        relay::Event::CircuitReqAccepted {
            src_peer_id,
            dst_peer_id,
        } => println!(
            "Circuit request from {} to peer {} accepted",
            src_peer_id, dst_peer_id
        ),
        relay::Event::CircuitReqOutboundConnectFailed {
            src_peer_id,
            dst_peer_id,
            error,
        } => println!(
            "Failed to connect the outbound from {} to {}, error: {}",
            src_peer_id, dst_peer_id, error
        ),
        relay::Event::CircuitReqAcceptFailed {
            src_peer_id,
            dst_peer_id,
            error,
        } => println!(
            "Failed to accept circuit request from {} to {}, error: {}",
            src_peer_id, dst_peer_id, error
        ),
        relay::Event::CircuitClosed {
            src_peer_id,
            dst_peer_id,
            error,
        } => println!(
            "Circuit from {} to {} closed, error?: {:?}",
            src_peer_id, dst_peer_id, error
        ),
    }
}
