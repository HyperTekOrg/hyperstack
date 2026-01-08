use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    State,
    Kv,
    Append,
    List,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub mode: Mode,
    #[serde(rename = "entity")]
    pub export: String,
    pub op: String,
    pub key: String,
    pub data: serde_json::Value,
}

impl Frame {
    pub fn entity(&self) -> &str {
        &self.export
    }
}
