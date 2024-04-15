use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub timeout: Duration,
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
        }
    }
}
