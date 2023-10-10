use libp2p::{PeerId, Multiaddr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Peer{
    id:PeerId,
    related_address:Vec<RelatedAddress>,
    public_key:Option<PublicKey>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RelatedAddress{
    address:Multiaddr,
    last_seen:chrono::DateTime<chrono::Utc>
}

/// Placeholder
#[derive(Debug, Serialize, Deserialize)]
pub struct PublicKey;