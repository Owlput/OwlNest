use libp2p::{
    identity::{self, Keypair},
    PeerId,
};
use std::path::Path;
use std::{fs, io::Write};

#[derive(Debug, Clone)]
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
        self.peer_id
    }
    pub fn from_file_rsa_pkcs8_private<P>(path: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        P: AsRef<Path>,
    {
        let mut key = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => return Err(Box::new(e)),
        };
        let keypair = match Keypair::rsa_from_pkcs8(&mut key) {
            Ok(keypair) => keypair,
            Err(e) => return Err(Box::new(e)),
        };
        let peer_id = PeerId::from(keypair.public());
        Ok(IdentityUnion { keypair, peer_id })
    }
    pub fn from_file_protobuf_encoding<P>(path: P) -> Result<Self, Box<dyn std::error::Error>>
    where
        P: AsRef<Path>,
    {
        let buf = match fs::read(path) {
            Ok(buf) => buf,
            Err(e) => return Err(Box::new(e)),
        };
        let keypair = match Keypair::from_protobuf_encoding(&buf) {
            Ok(keypair) => keypair,
            Err(e) => return Err(Box::new(e)),
        };
        Ok(Self {
            peer_id: PeerId::from_public_key(&keypair.public()),
            keypair,
        })
    }

    pub fn export_public_key(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let buf = self.get_pubkey().encode_protobuf();
        Self::export_to_file(path, &buf)
    }
    /// Export the keypair to the given file.
    pub fn export_keypair(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        Self::export_to_file(path, &self.keypair.to_protobuf_encoding().unwrap())
    }
    fn export_to_file<P>(path: P, buf: &[u8]) -> Result<(), std::io::Error>
    where
        P: AsRef<Path>,
    {
        let mut handle = std::fs::File::create(path)?;
        handle.write_all(buf)
    }
}

impl Default for IdentityUnion {
    fn default() -> Self {
        Self::generate()
    }
}

impl From<Keypair> for IdentityUnion {
    fn from(value: Keypair) -> Self {
        let peer_id = PeerId::from(value.public());
        Self {
            keypair: value,
            peer_id,
        }
    }
}
