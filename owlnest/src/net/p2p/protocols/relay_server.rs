use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info};

pub type Behaviour = libp2p::relay::Behaviour;
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
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub max_reservations: usize,
    pub max_reservations_per_peer: usize,
    pub reservation_duration: u64,

    pub max_circuits: usize,
    pub max_circuits_per_peer: usize,
    pub max_circuit_duration: u64,
    pub max_circuit_bytes: u64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            max_reservations: 128,
            max_reservations_per_peer: 4,
            reservation_duration: 60 * 60,

            max_circuits: 16,
            max_circuits_per_peer: 4,
            max_circuit_duration: 12 * 60 * 60, // 12 Hours
            max_circuit_bytes: 0,               // Unlimited
        }
    }
}
impl Into<libp2p::relay::Config> for Config {
    fn into(self) -> libp2p::relay::Config {
        let default = libp2p::relay::Config::default();
        let Config {
            max_reservations,
            max_reservations_per_peer,
            reservation_duration,
            max_circuits,
            max_circuits_per_peer,
            max_circuit_duration,
            max_circuit_bytes,
        } = self;
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
