use std::time::Duration;

pub struct Behaviour {
    config: Config,
}

pub struct Config {
    timeout: Duration,
}

impl Config {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(30),
        }
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}
impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
