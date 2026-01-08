use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub enum StreamStatus {
    Connected,
    Disconnected,
    Reconnecting,
    Error(String),
}

/// Configuration for health monitoring
#[derive(Debug, Clone)]
pub struct HealthConfig {
    pub heartbeat_interval: Duration,
    pub health_check_timeout: Duration,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(10),
        }
    }
}

impl HealthConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    pub fn with_health_check_timeout(mut self, timeout: Duration) -> Self {
        self.health_check_timeout = timeout;
        self
    }
}

/// Health monitor for tracking stream status and connectivity
pub struct HealthMonitor {
    config: HealthConfig,
    stream_status: Arc<RwLock<StreamStatus>>,
    last_event_time: Arc<RwLock<Option<SystemTime>>>,
    error_count: Arc<RwLock<u32>>,
    connection_start_time: Arc<RwLock<Option<Instant>>>,
}

impl HealthMonitor {
    pub fn new(config: HealthConfig) -> Self {
        Self {
            config,
            stream_status: Arc::new(RwLock::new(StreamStatus::Disconnected)),
            last_event_time: Arc::new(RwLock::new(None)),
            error_count: Arc::new(RwLock::new(0)),
            connection_start_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the health monitoring background task
    pub async fn start(&self) -> tokio::task::JoinHandle<()> {
        let monitor = self.clone();

        tokio::spawn(async move {
            let mut interval = interval(monitor.config.heartbeat_interval);

            loop {
                interval.tick().await;
                monitor.check_health().await;
            }
        })
    }

    /// Record that an event was received from the stream
    pub async fn record_event(&self) {
        *self.last_event_time.write().await = Some(SystemTime::now());
    }

    /// Record that the stream connection was established
    pub async fn record_connection(&self) {
        *self.stream_status.write().await = StreamStatus::Connected;
        *self.connection_start_time.write().await = Some(Instant::now());
        info!("Stream connection established");
    }

    /// Record that the stream disconnected
    pub async fn record_disconnection(&self) {
        *self.stream_status.write().await = StreamStatus::Disconnected;
        *self.connection_start_time.write().await = None;
        warn!("Stream disconnected");
    }

    /// Record that the stream is attempting to reconnect
    pub async fn record_reconnecting(&self) {
        *self.stream_status.write().await = StreamStatus::Reconnecting;
        info!("Stream reconnecting");
    }

    /// Record an error from the stream
    pub async fn record_error(&self, error: String) {
        *self.stream_status.write().await = StreamStatus::Error(error.clone());
        *self.error_count.write().await += 1;
        error!("Stream error: {}", error);
    }

    /// Check if the stream is currently healthy
    pub async fn is_healthy(&self) -> bool {
        let status = self.stream_status.read().await;
        let last_event_time = *self.last_event_time.read().await;

        match *status {
            StreamStatus::Connected => {
                // Check if we've received events recently
                if let Some(last_event) = last_event_time {
                    let time_since_last_event = SystemTime::now()
                        .duration_since(last_event)
                        .unwrap_or(Duration::from_secs(u64::MAX));

                    // Consider unhealthy if no events for 2x heartbeat interval
                    time_since_last_event < (self.config.heartbeat_interval * 2)
                } else {
                    // No events yet, but connected - might be waiting for first event
                    let connection_time = self.connection_start_time.read().await;
                    if let Some(start_time) = *connection_time {
                        let time_since_connection = start_time.elapsed();
                        // Give it some time to receive first event
                        time_since_connection < Duration::from_secs(60)
                    } else {
                        false
                    }
                }
            }
            StreamStatus::Reconnecting => true, // Considered healthy if actively reconnecting
            _ => false,
        }
    }

    /// Get the current stream status
    pub async fn status(&self) -> StreamStatus {
        self.stream_status.read().await.clone()
    }

    /// Get the current error count
    pub async fn error_count(&self) -> u32 {
        *self.error_count.read().await
    }

    async fn check_health(&self) {
        let is_healthy = self.is_healthy().await;
        let status = self.stream_status.read().await.clone();

        if !is_healthy {
            match status {
                StreamStatus::Connected => {
                    warn!("Stream appears to be stale - no recent events");
                }
                StreamStatus::Disconnected => {
                    warn!("Stream is disconnected");
                }
                StreamStatus::Error(ref error) => {
                    error!("Stream in error state: {}", error);
                }
                StreamStatus::Reconnecting => {
                    info!("Stream is reconnecting");
                }
            }
        }
    }
}

impl Clone for HealthMonitor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            stream_status: Arc::clone(&self.stream_status),
            last_event_time: Arc::clone(&self.last_event_time),
            error_count: Arc::clone(&self.error_count),
            connection_start_time: Arc::clone(&self.connection_start_time),
        }
    }
}
