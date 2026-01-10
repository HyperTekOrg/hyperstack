use crate::frame::{Frame, Operation};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone)]
pub struct StoreUpdate {
    pub view: String,
    pub key: String,
    pub operation: Operation,
    pub data: Option<serde_json::Value>,
}

pub struct SharedStore {
    entities: Arc<RwLock<HashMap<String, HashMap<String, serde_json::Value>>>>,
    updates_tx: broadcast::Sender<StoreUpdate>,
}

impl SharedStore {
    pub fn new() -> Self {
        let (updates_tx, _) = broadcast::channel(1000);
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            updates_tx,
        }
    }

    pub async fn apply_frame(&self, frame: Frame) {
        let mut entities = self.entities.write().await;
        let view_map = entities
            .entry(frame.entity.clone())
            .or_insert_with(HashMap::new);

        let operation = frame.operation();

        match operation {
            Operation::Upsert | Operation::Create => {
                view_map.insert(frame.key.clone(), frame.data.clone());
            }
            Operation::Patch => {
                let entry = view_map
                    .entry(frame.key.clone())
                    .or_insert_with(|| serde_json::json!({}));
                if let (Some(obj), Some(patch)) = (entry.as_object_mut(), frame.data.as_object()) {
                    for (k, v) in patch {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
            Operation::Delete => {
                view_map.remove(&frame.key);
            }
        }

        let _ = self.updates_tx.send(StoreUpdate {
            view: frame.entity,
            key: frame.key,
            operation,
            data: Some(frame.data),
        });
    }

    pub async fn get<T: DeserializeOwned>(&self, view: &str, key: &str) -> Option<T> {
        let entities = self.entities.read().await;
        entities
            .get(view)?
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub async fn list<T: DeserializeOwned>(&self, view: &str) -> Vec<T> {
        let entities = self.entities.read().await;
        entities
            .get(view)
            .map(|map| {
                map.values()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub async fn all_raw(&self, view: &str) -> HashMap<String, serde_json::Value> {
        let entities = self.entities.read().await;
        entities.get(view).cloned().unwrap_or_default()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StoreUpdate> {
        self.updates_tx.subscribe()
    }
}

impl Default for SharedStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SharedStore {
    fn clone(&self) -> Self {
        Self {
            entities: self.entities.clone(),
            updates_tx: self.updates_tx.clone(),
        }
    }
}
