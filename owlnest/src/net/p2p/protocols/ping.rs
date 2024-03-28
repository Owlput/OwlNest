pub use libp2p::ping::Behaviour;
pub use libp2p::ping::Event as OutEvent;
use tracing::trace;

pub(crate) fn ev_dispatch(ev: &OutEvent) {
    let result = match &ev.result {
        Ok(rtt) => {
            format!("success with RTT {}ms", rtt.as_millis())
        }
        Err(e) => match e {
            libp2p::ping::Failure::Timeout => "timed out".into(),
            libp2p::ping::Failure::Unsupported => "unsupported".into(),
            libp2p::ping::Failure::Other { error } => format!("error {:?}", error),
        },
    };
    trace!("Ping to {} returned {}", ev.peer, result);
}
