use serde::{Deserialize, Serialize};

/// Streaming mode for different data access patterns
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Latest value only (watch semantics)
    State,
    /// Append-only stream
    Append,
    /// Collection/list view (also used for key-value lookups)
    List,
}

/// Data frame sent over WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub mode: Mode,
    #[serde(rename = "entity")]
    pub export: String,
    pub op: &'static str,
    pub key: String,
    pub data: serde_json::Value,
}

/// A single entity within a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntity {
    pub key: String,
    pub data: serde_json::Value,
}

/// Batch snapshot frame for initial data load
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFrame {
    pub mode: Mode,
    #[serde(rename = "entity")]
    pub export: String,
    pub op: &'static str,
    pub data: Vec<SnapshotEntity>,
}

impl Frame {
    pub fn entity(&self) -> &str {
        &self.export
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_entity_key_accessors() {
        let frame = Frame {
            mode: Mode::List,
            export: "SettlementGame/list".to_string(),
            op: "upsert",
            key: "123".to_string(),
            data: serde_json::json!({}),
        };

        assert_eq!(frame.entity(), "SettlementGame/list");
        assert_eq!(frame.key(), "123");
    }

    #[test]
    fn test_frame_serialization() {
        let frame = Frame {
            mode: Mode::List,
            export: "SettlementGame/list".to_string(),
            op: "upsert",
            key: "123".to_string(),
            data: serde_json::json!({"gameId": "123"}),
        };

        let json = serde_json::to_value(&frame).unwrap();
        assert_eq!(json["op"], "upsert");
        assert_eq!(json["mode"], "list");
        assert_eq!(json["entity"], "SettlementGame/list");
        assert_eq!(json["key"], "123");
    }
}
