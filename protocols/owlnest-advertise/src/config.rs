use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub timeout_ms: u64,
    pub max_advertise_capacity: usize,
}
impl Config {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            timeout_ms: 30 * 1000,
            max_advertise_capacity: 32,
        }
    }
}
