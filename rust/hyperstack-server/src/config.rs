use std::net::SocketAddr;

pub use crate::health::HealthConfig;
pub use crate::http_health::HttpHealthConfig;

/// WebSocket server configuration
#[derive(Clone, Debug)]
pub struct WebSocketConfig {
    pub bind_address: SocketAddr,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::]:8877".parse().expect("valid socket address"),
        }
    }
}

impl WebSocketConfig {
    pub fn new(bind_address: impl Into<SocketAddr>) -> Self {
        Self {
            bind_address: bind_address.into(),
        }
    }
}

/// Yellowstone gRPC configuration
#[derive(Clone, Debug)]
pub struct YellowstoneConfig {
    pub endpoint: String,
    pub x_token: Option<String>,
}

impl YellowstoneConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            x_token: None,
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.x_token = Some(token.into());
        self
    }
}

/// Main server configuration
#[derive(Clone, Debug, Default)]
pub struct ServerConfig {
    pub websocket: Option<WebSocketConfig>,
    pub yellowstone: Option<YellowstoneConfig>,
    pub health: Option<HealthConfig>,
    pub http_health: Option<HttpHealthConfig>,
}

impl ServerConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_websocket(mut self, config: WebSocketConfig) -> Self {
        self.websocket = Some(config);
        self
    }

    pub fn with_yellowstone(mut self, config: YellowstoneConfig) -> Self {
        self.yellowstone = Some(config);
        self
    }

    pub fn with_health(mut self, config: HealthConfig) -> Self {
        self.health = Some(config);
        self
    }

    pub fn with_http_health(mut self, config: HttpHealthConfig) -> Self {
        self.http_health = Some(config);
        self
    }
}
