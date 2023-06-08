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

    #[cfg(feature = "disabled")]
    pub fn export_public_key(
        &self,
        folder_path: &str,
        file_name: &str,
    ) -> Result<(), std::io::Error> {
        let folder_path = if folder_path.ends_with("/") {
            folder_path.rsplit_once("/").unwrap().0
        } else {
            folder_path
        };
        let (buf, extension) = match self.get_pubkey() {
            identity::PublicKey::Ed25519(key) => (key.encode().to_vec(), "ed25519"),
            identity::PublicKey::Rsa(key) => (key.encode_pkcs1(), "rsa"),
            identity::PublicKey::Secp256k1(key) => {
                (key.encode_uncompressed().to_vec(), "secp256k1")
            }
            identity::PublicKey::Ecdsa(key) => (key.encode_der().to_vec(), "der"),
        };

        let path = format!("{}/{}.{}", folder_path, file_name, extension);
        Self::export_to_file(path, &buf)
    }
    pub fn export_keypair(&self, folder_path: &str, file_name: &str) -> Result<(), std::io::Error> {
        let folder_path = if folder_path.ends_with('/') {
            folder_path.rsplit_once('/').unwrap().0
        } else {
            folder_path
        };
        let path = format!("{}/{}.keypair", folder_path, file_name);
        Self::export_to_file(path, &self.keypair.to_protobuf_encoding().unwrap())
    }
    fn export_to_file<P>(path: P, buf: &[u8]) -> Result<(), std::io::Error>
    where
        P: AsRef<Path>,
    {
        let mut handle = match std::fs::File::create(path) {
            Ok(handle) => handle,
            Err(e) => return Err(e),
        };
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
