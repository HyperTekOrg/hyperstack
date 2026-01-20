use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::io::Read;

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

#[derive(Deserialize)]
struct CompressedFrame {
    compressed: String,
    data: String,
}

fn decompress_gzip(base64_data: &str) -> Result<String, Box<dyn std::error::Error>> {
    let compressed = BASE64.decode(base64_data)?;
    let mut decoder = GzDecoder::new(&compressed[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed)?;
    Ok(decompressed)
}

fn parse_and_decompress(text: &str) -> Result<Frame, serde_json::Error> {
    if let Ok(compressed) = serde_json::from_str::<CompressedFrame>(text) {
        if compressed.compressed == "gzip" {
            if let Ok(decompressed) = decompress_gzip(&compressed.data) {
                return serde_json::from_str(&decompressed);
            }
        }
    }
    serde_json::from_str(text)
}

pub fn parse_frame(bytes: &[u8]) -> Result<Frame, serde_json::Error> {
    let text = String::from_utf8_lossy(bytes);
    parse_and_decompress(&text)
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

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;

    #[test]
    fn test_parse_uncompressed_frame() {
        let frame_json = r#"{"mode":"list","entity":"test/list","op":"snapshot","key":"","data":[{"key":"1","data":{"id":1}}]}"#;
        let frame = parse_frame(frame_json.as_bytes()).unwrap();
        assert_eq!(frame.op, "snapshot");
        assert_eq!(frame.entity, "test/list");
    }

    #[test]
    fn test_parse_compressed_frame() {
        let original = r#"{"mode":"list","entity":"test/list","op":"snapshot","key":"","data":[{"key":"1","data":{"id":1}}]}"#;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(original.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();
        let base64_data = BASE64.encode(&compressed);

        let compressed_frame = format!(r#"{{"compressed":"gzip","data":"{}"}}"#, base64_data);

        let frame = parse_frame(compressed_frame.as_bytes()).unwrap();
        assert_eq!(frame.op, "snapshot");
        assert_eq!(frame.entity, "test/list");
    }
}
