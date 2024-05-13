use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub timeout: Duration,
    pub max_advertise_capacity: usize,
}
impl Config {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_advertise_capacity: 32,
        }
    }
}
