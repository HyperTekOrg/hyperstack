use thiserror::Error;

#[derive(Error, Debug)]
pub enum HyperStackError {
    #[error("Missing WebSocket URL")]
    MissingUrl,

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Max reconnection attempts reached ({0})")]
    MaxReconnectAttempts(u32),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),

    #[error("Channel error: {0}")]
    ChannelError(String),
}
