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
}

impl std::str::FromStr for Operation {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "upsert" => Operation::Upsert,
            "patch" => Operation::Patch,
            "delete" => Operation::Delete,
            "create" => Operation::Create,
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
    pub key: String,
    pub data: serde_json::Value,
}

impl Frame {
    pub fn entity_name(&self) -> &str {
        &self.entity
    }

    pub fn operation(&self) -> Operation {
        self.op.parse().unwrap()
    }
}

pub fn parse_frame(bytes: &[u8]) -> Result<Frame, serde_json::Error> {
    let text = String::from_utf8_lossy(bytes);
    serde_json::from_str(&text)
}
