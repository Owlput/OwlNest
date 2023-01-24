use super::*;
#[derive(Debug, Serialize, Deserialize)]
pub enum TetherOps {
    Dial(Multiaddr),
    Listen(Multiaddr),
    Trust(PeerId),
}
impl TetherOps {
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}
