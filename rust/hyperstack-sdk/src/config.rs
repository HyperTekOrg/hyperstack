use std::time::Duration;

#[derive(Debug, Clone)]
pub struct HyperStackConfig {
    pub auto_reconnect: bool,
    pub reconnect_intervals: Vec<Duration>,
    pub max_reconnect_attempts: u32,
    pub ping_interval: Duration,
}

impl Default for HyperStackConfig {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            reconnect_intervals: vec![
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
                Duration::from_secs(8),
                Duration::from_secs(16),
            ],
            max_reconnect_attempts: 5,
            ping_interval: Duration::from_secs(15),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub auto_reconnect: bool,
    pub reconnect_intervals: Vec<Duration>,
    pub max_reconnect_attempts: u32,
    pub ping_interval: Duration,
}

impl From<HyperStackConfig> for ConnectionConfig {
    fn from(config: HyperStackConfig) -> Self {
        Self {
            auto_reconnect: config.auto_reconnect,
            reconnect_intervals: config.reconnect_intervals,
            max_reconnect_attempts: config.max_reconnect_attempts,
            ping_interval: config.ping_interval,
        }
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        HyperStackConfig::default().into()
    }
}
