use crate::frame::{Frame, Operation};
use serde::de::DeserializeOwned;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, watch, RwLock};

#[derive(Debug, Clone)]
pub struct StoreUpdate {
    pub view: String,
    pub key: String,
    pub operation: Operation,
    pub data: Option<serde_json::Value>,
    pub previous: Option<serde_json::Value>,
}

pub struct SharedStore {
    entities: Arc<RwLock<HashMap<String, HashMap<String, serde_json::Value>>>>,
    updates_tx: broadcast::Sender<StoreUpdate>,
    ready_views: Arc<RwLock<HashSet<String>>>,
    ready_tx: watch::Sender<HashSet<String>>,
    ready_rx: watch::Receiver<HashSet<String>>,
}

impl SharedStore {
    pub fn new() -> Self {
        let (updates_tx, _) = broadcast::channel(1000);
        let (ready_tx, ready_rx) = watch::channel(HashSet::new());
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            updates_tx,
            ready_views: Arc::new(RwLock::new(HashSet::new())),
            ready_tx,
            ready_rx,
        }
    }

    pub async fn apply_frame(&self, frame: Frame) {
        let entity_name = extract_entity_name(&frame.entity);
        let data = unwrap_list_item(&frame.data);
        tracing::debug!(
            "apply_frame: entity={}, key={}, op={}, has_item={}",
            entity_name,
            frame.key,
            frame.op,
            frame.data.get("item").is_some()
        );

        let mut entities = self.entities.write().await;
        let view_map = entities
            .entry(entity_name.to_string())
            .or_insert_with(HashMap::new);

        let operation = frame.operation();

        let previous = view_map.get(&frame.key).cloned();

        match operation {
            Operation::Upsert | Operation::Create => {
                view_map.insert(frame.key.clone(), data.clone());
            }
            Operation::Patch => {
                let entry = view_map
                    .entry(frame.key.clone())
                    .or_insert_with(|| serde_json::json!({}));
                if let (Some(obj), Some(patch)) = (entry.as_object_mut(), data.as_object()) {
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
            view: entity_name.to_string(),
            key: frame.key,
            operation,
            data: Some(data),
            previous,
        });

        self.mark_view_ready(entity_name).await;
    }

    pub async fn mark_view_ready(&self, view: &str) {
        let mut ready = self.ready_views.write().await;
        if ready.insert(view.to_string()) {
            let _ = self.ready_tx.send(ready.clone());
        }
    }

    pub async fn wait_for_view_ready(&self, view: &str, timeout: std::time::Duration) -> bool {
        let entity_name = extract_entity_name(view);

        if self.ready_views.read().await.contains(entity_name) {
            return true;
        }

        let mut rx = self.ready_rx.clone();
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let timeout_remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if timeout_remaining.is_zero() {
                return false;
            }

            tokio::select! {
                result = rx.changed() => {
                    if result.is_err() {
                        return false;
                    }
                    if rx.borrow().contains(entity_name) {
                        return true;
                    }
                }
                _ = tokio::time::sleep(timeout_remaining) => {
                    return false;
                }
            }
        }
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

fn extract_entity_name(view_path: &str) -> &str {
    view_path.split('/').next().unwrap_or(view_path)
}

fn unwrap_list_item(data: &serde_json::Value) -> serde_json::Value {
    if let Some(item) = data.get("item") {
        item.clone()
    } else {
        data.clone()
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
            ready_views: self.ready_views.clone(),
            ready_tx: self.ready_tx.clone(),
            ready_rx: self.ready_rx.clone(),
        }
    }
}
