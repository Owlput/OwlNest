use super::*;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub time: u128,
    pub from: PeerId,
    pub to: PeerId,
    pub msg: String,
}
impl Message {
    pub fn new(from: PeerId, to: PeerId, msg: String) -> Self {
        Self {
            time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            from,
            to,
            msg,
        }
    }
    #[inline]
    pub fn as_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }
}
