use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub timeout: Duration,
    pub max_pending_recv: usize,
    /// Timeout in seconds. 0 for no timeout.
    pub pending_recv_timeout: u64,
    pub ongoing_recv_timeout: u64,
    /// Timeout in seconds. 0 for no timeout.
    pub pending_send_timeout: u64,
    pub ongoing_send_timeout: u64,
}
impl Config {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            max_pending_recv: 16,
            pending_recv_timeout: 60 * 60,
            ongoing_recv_timeout: 60,
            pending_send_timeout: 0,
            ongoing_send_timeout: 3 * 60,
        }
    }
}
