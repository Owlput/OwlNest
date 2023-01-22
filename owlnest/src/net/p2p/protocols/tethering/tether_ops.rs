use super::*;
use owlnest_proc::impl_stamp;

#[derive(Debug, Serialize, Deserialize)]
#[impl_stamp]
pub enum TetherOps {
    Dial(u128, Multiaddr),
    Listen(u128, Multiaddr),
    Trust(u128, PeerId),
}
impl TetherOps {
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}
