use crate::frame::{parse_snapshot_entities, Frame, Operation};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, watch, RwLock};

/// Default maximum number of entries per view before LRU eviction kicks in.
/// Set to 10,000 to provide a reasonable balance between memory usage and data retention.
pub const DEFAULT_MAX_ENTRIES_PER_VIEW: usize = 10_000;

/// Configuration for the SharedStore.
#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// Maximum number of entries to keep per view. When exceeded, oldest entries
    /// are evicted using LRU (Least Recently Used) strategy.
    /// Set to `None` to disable size limiting (not recommended for long-running clients).
    pub max_entries_per_view: Option<usize>,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            max_entries_per_view: Some(DEFAULT_MAX_ENTRIES_PER_VIEW),
        }
    }
}

/// Tracks access order for LRU eviction within a view.
struct ViewData {
    /// The actual entity data keyed by entity key.
    entities: HashMap<String, serde_json::Value>,
    /// Access order queue - front is oldest, back is most recent.
    /// Used for LRU eviction when max_entries is exceeded.
    access_order: VecDeque<String>,
}

fn deep_merge_with_append(
    target: &mut Value,
    patch: &Value,
    append_paths: &[String],
    current_path: &str,
) {
    match (target, patch) {
        (Value::Object(target_map), Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                let field_path = if current_path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", current_path, key)
                };
                match target_map.get_mut(key) {
                    Some(target_value) => {
                        deep_merge_with_append(target_value, patch_value, append_paths, &field_path)
                    }
                    None => {
                        target_map.insert(key.clone(), patch_value.clone());
                    }
                }
            }
        }
        (Value::Array(target_arr), Value::Array(patch_arr))
            if append_paths.contains(&current_path.to_string()) =>
        {
            target_arr.extend(patch_arr.iter().cloned());
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
    views: Arc<RwLock<HashMap<String, ViewData>>>,
    updates_tx: broadcast::Sender<StoreUpdate>,
    ready_views: Arc<RwLock<HashSet<String>>>,
    ready_tx: watch::Sender<HashSet<String>>,
    ready_rx: watch::Receiver<HashSet<String>>,
    config: StoreConfig,
}

impl ViewData {
    fn new() -> Self {
        Self {
            entities: HashMap::new(),
            access_order: VecDeque::new(),
        }
    }

    fn touch(&mut self, key: &str) {
        self.access_order.retain(|k| k != key);
        self.access_order.push_back(key.to_string());
    }

    fn insert(&mut self, key: String, value: serde_json::Value) {
        if !self.entities.contains_key(&key) {
            self.access_order.push_back(key.clone());
        } else {
            self.touch(&key);
        }
        self.entities.insert(key, value);
    }

    fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.access_order.retain(|k| k != key);
        self.entities.remove(key)
    }

    fn evict_oldest(&mut self) -> Option<String> {
        if let Some(oldest_key) = self.access_order.pop_front() {
            self.entities.remove(&oldest_key);
            Some(oldest_key)
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.entities.len()
    }
}

impl SharedStore {
    pub fn new() -> Self {
        Self::with_config(StoreConfig::default())
    }

    pub fn with_config(config: StoreConfig) -> Self {
        let (updates_tx, _) = broadcast::channel(1000);
        let (ready_tx, ready_rx) = watch::channel(HashSet::new());
        Self {
            views: Arc::new(RwLock::new(HashMap::new())),
            updates_tx,
            ready_views: Arc::new(RwLock::new(HashSet::new())),
            ready_tx,
            ready_rx,
            config,
        }
    }

    fn enforce_max_entries(&self, view_data: &mut ViewData) {
        if let Some(max) = self.config.max_entries_per_view {
            while view_data.len() > max {
                if let Some(evicted_key) = view_data.evict_oldest() {
                    tracing::debug!("evicted oldest entry: {}", evicted_key);
                }
            }
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

        let mut views = self.views.write().await;
        let view_data = views
            .entry(entity_name.to_string())
            .or_insert_with(ViewData::new);

        let previous = view_data.entities.get(&frame.key).cloned();

        let (current, patch) = match operation {
            Operation::Upsert | Operation::Create => {
                view_data.insert(frame.key.clone(), frame.data.clone());
                self.enforce_max_entries(view_data);
                (Some(frame.data), None)
            }
            Operation::Patch => {
                let raw_patch = frame.data.clone();
                let entry = view_data
                    .entities
                    .entry(frame.key.clone())
                    .or_insert_with(|| serde_json::json!({}));
                deep_merge_with_append(entry, &frame.data, &frame.append, "");
                let merged = entry.clone();
                view_data.touch(&frame.key);
                self.enforce_max_entries(view_data);
                (Some(merged), Some(raw_patch))
            }
            Operation::Delete => {
                view_data.remove(&frame.key);
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

        let mut views = self.views.write().await;
        let view_data = views
            .entry(entity_name.to_string())
            .or_insert_with(ViewData::new);

        for entity in snapshot_entities {
            let previous = view_data.entities.get(&entity.key).cloned();
            view_data.insert(entity.key.clone(), entity.data.clone());

            let _ = self.updates_tx.send(StoreUpdate {
                view: entity_name.to_string(),
                key: entity.key,
                operation: Operation::Upsert,
                data: Some(entity.data),
                previous,
                patch: None,
            });
        }

        self.enforce_max_entries(view_data);
        drop(views);
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
        let views = self.views.read().await;
        views
            .get(view)?
            .entities
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub async fn list<T: DeserializeOwned>(&self, view: &str) -> Vec<T> {
        let views = self.views.read().await;
        views
            .get(view)
            .map(|view_data| {
                view_data
                    .entities
                    .values()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub async fn all_raw(&self, view: &str) -> HashMap<String, serde_json::Value> {
        let views = self.views.read().await;
        views
            .get(view)
            .map(|view_data| view_data.entities.clone())
            .unwrap_or_default()
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
            views: self.views.clone(),
            updates_tx: self.updates_tx.clone(),
            ready_views: self.ready_views.clone(),
            ready_tx: self.ready_tx.clone(),
            ready_rx: self.ready_rx.clone(),
            config: self.config.clone(),
        }
    }
}
