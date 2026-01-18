use crate::frame::{parse_snapshot_entities, Frame, Operation};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, watch, RwLock};

fn deep_merge(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(target_map), Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                match target_map.get_mut(key) {
                    Some(target_value) => deep_merge(target_value, patch_value),
                    None => {
                        target_map.insert(key.clone(), patch_value.clone());
                    }
                }
            }
        }
        (target, patch) => {
            *target = patch.clone();
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoreUpdate {
    pub view: String,
    pub key: String,
    pub operation: Operation,
    pub data: Option<serde_json::Value>,
    pub previous: Option<serde_json::Value>,
    /// The raw patch data for Patch operations (before merging into full state).
    /// This allows consumers to see exactly what fields changed without diffing.
    pub patch: Option<serde_json::Value>,
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
        tracing::debug!(
            "apply_frame: entity={}, key={}, op={}",
            entity_name,
            frame.key,
            frame.op,
        );

        let operation = frame.operation();

        if operation == Operation::Snapshot {
            self.apply_snapshot(&frame).await;
            return;
        }

        let mut entities = self.entities.write().await;
        let view_map = entities
            .entry(entity_name.to_string())
            .or_insert_with(HashMap::new);

        let previous = view_map.get(&frame.key).cloned();

        let (current, patch) = match operation {
            Operation::Upsert | Operation::Create => {
                view_map.insert(frame.key.clone(), frame.data.clone());
                (Some(frame.data), None)
            }
            Operation::Patch => {
                let raw_patch = frame.data.clone();
                let entry = view_map
                    .entry(frame.key.clone())
                    .or_insert_with(|| serde_json::json!({}));
                deep_merge(entry, &frame.data);
                (Some(entry.clone()), Some(raw_patch))
            }
            Operation::Delete => {
                view_map.remove(&frame.key);
                (None, None)
            }
            Operation::Snapshot => unreachable!(),
        };

        let _ = self.updates_tx.send(StoreUpdate {
            view: entity_name.to_string(),
            key: frame.key,
            operation,
            data: current,
            previous,
            patch,
        });

        self.mark_view_ready(entity_name).await;
    }

    async fn apply_snapshot(&self, frame: &Frame) {
        let entity_name = extract_entity_name(&frame.entity);
        let snapshot_entities = parse_snapshot_entities(&frame.data);

        tracing::debug!(
            "apply_snapshot: entity={}, count={}",
            entity_name,
            snapshot_entities.len()
        );

        let mut entities = self.entities.write().await;
        let view_map = entities
            .entry(entity_name.to_string())
            .or_insert_with(HashMap::new);

        for entity in snapshot_entities {
            let previous = view_map.get(&entity.key).cloned();
            view_map.insert(entity.key.clone(), entity.data.clone());

            let _ = self.updates_tx.send(StoreUpdate {
                view: entity_name.to_string(),
                key: entity.key,
                operation: Operation::Upsert,
                data: Some(entity.data),
                previous,
                patch: None,
            });
        }

        drop(entities);
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
