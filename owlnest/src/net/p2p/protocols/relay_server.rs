use std::time::Duration;
use tracing::{debug, info};
use super::*;

pub use libp2p::relay::Behaviour;
pub use libp2p::relay::{HOP_PROTOCOL_NAME, STOP_PROTOCOL_NAME};

/// An ailas to `libp2p::relay::Event` for unified naming.
pub type OutEvent = libp2p::relay::Event;

/// Log the events to tracing
pub(crate) fn ev_dispatch(ev: &OutEvent) {
    use libp2p::relay::Event::*;
    match ev {
        ReservationReqAccepted {
            src_peer_id,
            renewed,
        } => debug!(
            "Reservation from {} accepted, IsRenew:{}",
            src_peer_id, renewed
        ),
        ReservationReqDenied { src_peer_id } => {
            info!("Denied reservation from {}", src_peer_id)
        }
        ReservationTimedOut { src_peer_id } => {
            info!("Reservation expired for source peer {}", src_peer_id)
        }
        CircuitReqDenied {
            src_peer_id,
            dst_peer_id,
        } => info!(
            "Circuit request from {} to peer {} denied",
            src_peer_id, dst_peer_id
        ),
        CircuitReqAccepted {
            src_peer_id,
            dst_peer_id,
        } => debug!(
            "Circuit request from {} to peer {} accepted",
            src_peer_id, dst_peer_id
        ),
        CircuitClosed {
            src_peer_id,
            dst_peer_id,
            error,
        } => info!(
            "Circuit from {} to {} closed, error?: {:?}",
            src_peer_id, dst_peer_id, error
        ),
        ev => info!("{:?}", ev),
    }
}

/// Configuration for the relay [`Behaviour`].
///
/// # Panics
///
/// [`Config::max_circuit_duration`] may not exceed [`u32::MAX`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Maximum amount of reservations that can be made on this peer.  
    pub max_reservations: usize,
    /// Maximum amount of reservations that can be made by a single peer.
    pub max_reservations_per_peer: usize,
    /// Timeout for a sigle reservation in seconds, default to 1 hour.
    pub reservation_duration_sec: u64,

    /// Maximum amount of active circuits that can be established through
    /// this peer.
    pub max_circuits: usize,
    /// Maximum amount of active circuits that can be made for a single
    /// source peer.
    pub max_circuits_per_peer: usize,
    /// Timeout for a single circuit in seconds, default to 12 hours.
    pub max_circuit_duration_sec: u64,
    /// Maximum amount of data a single circuit can handle in total,
    /// including both directions of all time since the circuit is established.
    /// Once exceeded, the circuit will be closed by the relay server
    /// immediately without clean up.  
    /// Default to 0(unlimited).
    pub max_circuit_bytes: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            max_reservations: 128,
            max_reservations_per_peer: 4,
            reservation_duration_sec: 60 * 60,

            max_circuits: 16,
            max_circuits_per_peer: 4,
            max_circuit_duration_sec: 12 * 60 * 60, // 12 Hours
            max_circuit_bytes: 0,                   // Unlimited
        }
    }
}
impl From<Config> for libp2p::relay::Config {
    fn from(value: Config) -> Self {
        let default = libp2p::relay::Config::default();
        let Config {
            max_reservations,
            max_reservations_per_peer,
            reservation_duration_sec: reservation_duration,
            max_circuits,
            max_circuits_per_peer,
            max_circuit_duration_sec: max_circuit_duration,
            max_circuit_bytes,
        } = value;
        libp2p::relay::Config {
            max_reservations,
            max_reservations_per_peer,
            reservation_duration: Duration::from_secs(reservation_duration),
            reservation_rate_limiters: default.reservation_rate_limiters,
            max_circuits,
            max_circuits_per_peer,
            max_circuit_duration: Duration::from_secs(max_circuit_duration),
            max_circuit_bytes,
            circuit_src_rate_limiters: default.circuit_src_rate_limiters,
        }
    }
}
