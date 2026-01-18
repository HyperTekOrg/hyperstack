use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    State,
    Append,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Upsert,
    Patch,
    Delete,
    Create,
    Snapshot,
}

impl std::str::FromStr for Operation {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "upsert" => Operation::Upsert,
            "patch" => Operation::Patch,
            "delete" => Operation::Delete,
            "create" => Operation::Create,
            "snapshot" => Operation::Snapshot,
            _ => Operation::Upsert,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub mode: Mode,
    #[serde(rename = "entity")]
    pub entity: String,
    pub op: String,
    #[serde(default)]
    pub key: String,
    pub data: serde_json::Value,
    #[serde(default)]
    pub append: Vec<String>,
}

impl Frame {
    pub fn entity_name(&self) -> &str {
        &self.entity
    }

    pub fn operation(&self) -> Operation {
        self.op.parse().unwrap()
    }

    pub fn is_snapshot(&self) -> bool {
        self.op == "snapshot"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntity {
    pub key: String,
    pub data: serde_json::Value,
}

pub fn parse_frame(bytes: &[u8]) -> Result<Frame, serde_json::Error> {
    let text = String::from_utf8_lossy(bytes);
    serde_json::from_str(&text)
}

pub fn parse_snapshot_entities(data: &serde_json::Value) -> Vec<SnapshotEntity> {
    match data {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}
