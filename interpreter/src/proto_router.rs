use prost_types::Any;
use serde_json::Value;
use std::collections::HashMap;

pub type ProtoDecoder = fn(&[u8]) -> Result<(Value, String), Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct ProtoRouter {
    decoders: HashMap<String, ProtoDecoder>,
}

impl ProtoRouter {
    pub fn new() -> Self {
        ProtoRouter {
            decoders: HashMap::new(),
        }
    }

    pub fn register(&mut self, type_url: String, decoder: ProtoDecoder) {
        self.decoders.insert(type_url, decoder);
    }

    pub fn decode(&self, any: Any) -> Result<(Value, String), Box<dyn std::error::Error>> {
        let decoder = self
            .decoders
            .get(&any.type_url)
            .ok_or_else(|| format!("No decoder found for type_url: {}", any.type_url))?;

        decoder(&any.value)
    }
}

impl Default for ProtoRouter {
    fn default() -> Self {
        Self::new()
    }
}
