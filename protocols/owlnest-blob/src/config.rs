use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Timeout for a transmission, in milliseconds. 0 for no timeout(wait forever).
    pub timeout_ms: u64,
    pub max_pending_recv: usize,
    /// Timeout in seconds. 0 for no timeout(wait forever).
    pub pending_recv_timeout_sec: u64,
    pub ongoing_recv_timeout_sec: u64,
    /// Timeout in seconds. 0 for no timeout(wait forever).
    pub pending_send_timeout_sec: u64,
    pub ongoing_send_timeout_sec: u64,
}
impl Config {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            timeout_ms: 60 * 1000,
            max_pending_recv: 16,
            pending_recv_timeout_sec: 60 * 60,
            ongoing_recv_timeout_sec: 60,
            pending_send_timeout_sec: 0,
            ongoing_send_timeout_sec: 3 * 60,
        }
    }
}
