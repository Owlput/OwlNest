use std::fs;
use std::path::Path;
use libp2p::{
    identity::{self, Keypair},
    PeerId,
};

#[derive(Debug,Clone)]
pub struct IdentityUnion {
    keypair: identity::Keypair,
    peer_id: PeerId,
}

impl IdentityUnion {
    pub fn generate() -> Self {
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        Self { keypair, peer_id }
    }
    pub fn get_pubkey(&self) -> identity::PublicKey {
        self.keypair.public()
    }
    pub fn get_keypair(&self) -> identity::Keypair {
        self.keypair.clone()
    }
    /// Return a clone of the `peer_id` field.
    pub fn get_peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }
    pub fn from_rsa_pkcs8_private(path: &Path) -> Result<Self,Box<dyn std::error::Error>> {
        let mut key = match fs::read(path){
            Ok(bytes) => bytes,
            Err(e) => return Err(Box::new(e)),
        };
        let keypair = match Keypair::rsa_from_pkcs8(&mut key){
            Ok(keypair) => keypair,
            Err(e) => return Err(Box::new(e)),
        };
        let peer_id = PeerId::from(keypair.public());
        Ok(IdentityUnion { keypair, peer_id })
    }
}

impl From<Keypair> for IdentityUnion{
    fn from(value: Keypair) -> Self {
        let peer_id = PeerId::from(value.public());
        Self {
            keypair: value,
            peer_id,
        }
    }
} 