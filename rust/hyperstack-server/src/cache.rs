//! Entity cache for snapshot-on-subscribe functionality.
//!
//! This module provides an `EntityCache` that maintains full projected entities
//! in memory with LRU eviction. When a new client subscribes, they receive
//! cached snapshots immediately rather than waiting for the next live mutation.

use lru::LruCache;
use serde_json::Value;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Default maximum entities per view
const DEFAULT_MAX_ENTITIES_PER_VIEW: usize = 1000;

/// Default maximum array length before LRU eviction
const DEFAULT_MAX_ARRAY_LENGTH: usize = 100;

/// Configuration for the entity cache
#[derive(Debug, Clone)]
pub struct EntityCacheConfig {
    /// Maximum number of entities to cache per view
    pub max_entities_per_view: usize,
    /// Maximum array length before oldest elements are evicted
    pub max_array_length: usize,
}

impl Default for EntityCacheConfig {
    fn default() -> Self {
        Self {
            max_entities_per_view: DEFAULT_MAX_ENTITIES_PER_VIEW,
            max_array_length: DEFAULT_MAX_ARRAY_LENGTH,
        }
    }
}

/// Entity cache that maintains full projected entities with LRU eviction.
///
/// The cache is populated as mutations flow through the projector, regardless
/// of subscriber state. When a new subscriber connects, they receive snapshots
/// of all cached entities for their requested view.
#[derive(Clone)]
pub struct EntityCache {
    /// view_id -> LRU<entity_key, full_projected_entity>
    caches: Arc<RwLock<HashMap<String, LruCache<String, Value>>>>,
    config: EntityCacheConfig,
}

impl EntityCache {
    /// Create a new entity cache with default configuration
    pub fn new() -> Self {
        Self::with_config(EntityCacheConfig::default())
    }

    /// Create a new entity cache with custom configuration
    pub fn with_config(config: EntityCacheConfig) -> Self {
        Self {
            caches: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Upsert a patch into the cache, merging with existing entity data.
    ///
    /// This method:
    /// 1. Gets or creates the LRU cache for the view
    /// 2. Gets or creates an empty entity for the key
    /// 3. Deep merges the patch into the entity (appending arrays)
    /// 4. Updates the LRU cache (promoting the key to most recently used)
    pub async fn upsert(&self, view_id: &str, key: &str, patch: &Value) {
        let mut caches = self.caches.write().await;

        let cache = caches.entry(view_id.to_string()).or_insert_with(|| {
            LruCache::new(
                NonZeroUsize::new(self.config.max_entities_per_view)
                    .expect("max_entities_per_view must be > 0"),
            )
        });

        // Get existing entity or create empty object
        let entity = cache
            .get_mut(key)
            .map(|v| v.clone())
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

        // Deep merge patch into entity
        let merged = deep_merge_with_array_append(entity, patch, self.config.max_array_length);

        // Put back into cache (this also promotes to most recently used)
        cache.put(key.to_string(), merged);
    }

    /// Get all cached entities for a view.
    ///
    /// Returns a vector of (key, entity) pairs for sending as snapshots
    /// to new subscribers.
    pub async fn get_all(&self, view_id: &str) -> Vec<(String, Value)> {
        let caches = self.caches.read().await;

        caches
            .get(view_id)
            .map(|cache| cache.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    /// Get a specific entity from the cache
    pub async fn get(&self, view_id: &str, key: &str) -> Option<Value> {
        let caches = self.caches.read().await;
        caches
            .get(view_id)
            .and_then(|cache| cache.peek(key).cloned())
    }

    /// Get the number of cached entities for a view
    pub async fn len(&self, view_id: &str) -> usize {
        let caches = self.caches.read().await;
        caches.get(view_id).map(|c| c.len()).unwrap_or(0)
    }

    /// Check if the cache for a view is empty
    pub async fn is_empty(&self, view_id: &str) -> bool {
        self.len(view_id).await == 0
    }

    /// Clear all cached entities for a view
    pub async fn clear(&self, view_id: &str) {
        let mut caches = self.caches.write().await;
        if let Some(cache) = caches.get_mut(view_id) {
            cache.clear();
        }
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        let mut caches = self.caches.write().await;
        caches.clear();
    }
}

impl Default for EntityCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Deep merge two JSON values, appending arrays instead of replacing them.
///
/// For arrays, new elements are appended to the end. If the array exceeds
/// `max_array_length`, the oldest elements (from the beginning) are removed.
fn deep_merge_with_array_append(base: Value, patch: &Value, max_array_length: usize) -> Value {
    match (base, patch) {
        // Both are objects: recursively merge
        (Value::Object(mut base_map), Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                let merged = if let Some(base_value) = base_map.remove(key) {
                    deep_merge_with_array_append(base_value, patch_value, max_array_length)
                } else {
                    // Key doesn't exist in base, use patch value (with array truncation if needed)
                    truncate_arrays_if_needed(patch_value.clone(), max_array_length)
                };
                base_map.insert(key.clone(), merged);
            }
            Value::Object(base_map)
        }

        // Both are arrays: append and apply LRU eviction
        (Value::Array(mut base_arr), Value::Array(patch_arr)) => {
            // Append new elements
            base_arr.extend(patch_arr.iter().cloned());

            // LRU eviction: remove oldest elements from beginning if over limit
            if base_arr.len() > max_array_length {
                let excess = base_arr.len() - max_array_length;
                base_arr.drain(0..excess);
            }

            Value::Array(base_arr)
        }

        // Patch has array but base doesn't: use patch array (truncated if needed)
        (_, Value::Array(patch_arr)) => {
            let mut arr = patch_arr.clone();
            if arr.len() > max_array_length {
                let excess = arr.len() - max_array_length;
                arr.drain(0..excess);
            }
            Value::Array(arr)
        }

        // Default: patch value overwrites base
        (_, patch_value) => patch_value.clone(),
    }
}

/// Recursively truncate any arrays in a value to the max length
fn truncate_arrays_if_needed(value: Value, max_array_length: usize) -> Value {
    match value {
        Value::Array(mut arr) => {
            // Truncate this array if needed
            if arr.len() > max_array_length {
                let excess = arr.len() - max_array_length;
                arr.drain(0..excess);
            }
            // Recursively process elements
            Value::Array(
                arr.into_iter()
                    .map(|v| truncate_arrays_if_needed(v, max_array_length))
                    .collect(),
            )
        }
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, truncate_arrays_if_needed(v, max_array_length)))
                .collect(),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_basic_upsert_and_get() {
        let cache = EntityCache::new();

        cache
            .upsert("tokens/list", "abc123", &json!({"name": "Test Token"}))
            .await;

        let entity = cache.get("tokens/list", "abc123").await;
        assert!(entity.is_some());
        assert_eq!(entity.unwrap()["name"], "Test Token");
    }

    #[tokio::test]
    async fn test_deep_merge_objects() {
        let cache = EntityCache::new();

        // First patch: set initial data
        cache
            .upsert(
                "tokens/list",
                "abc123",
                &json!({
                    "id": "abc123",
                    "metrics": {"volume": 100}
                }),
            )
            .await;

        // Second patch: add more data
        cache
            .upsert(
                "tokens/list",
                "abc123",
                &json!({
                    "metrics": {"trades": 50}
                }),
            )
            .await;

        let entity = cache.get("tokens/list", "abc123").await.unwrap();
        assert_eq!(entity["id"], "abc123");
        assert_eq!(entity["metrics"]["volume"], 100);
        assert_eq!(entity["metrics"]["trades"], 50);
    }

    #[tokio::test]
    async fn test_array_append() {
        let cache = EntityCache::new();

        // First patch with initial events
        cache
            .upsert(
                "tokens/list",
                "abc123",
                &json!({
                    "events": [{"type": "buy", "amount": 100}]
                }),
            )
            .await;

        // Second patch appends to events
        cache
            .upsert(
                "tokens/list",
                "abc123",
                &json!({
                    "events": [{"type": "sell", "amount": 50}]
                }),
            )
            .await;

        let entity = cache.get("tokens/list", "abc123").await.unwrap();
        let events = entity["events"].as_array().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["type"], "buy");
        assert_eq!(events[1]["type"], "sell");
    }

    #[tokio::test]
    async fn test_array_lru_eviction() {
        let config = EntityCacheConfig {
            max_entities_per_view: 1000,
            max_array_length: 3,
        };
        let cache = EntityCache::with_config(config);

        // Add 5 events (exceeds max of 3)
        cache
            .upsert(
                "tokens/list",
                "abc123",
                &json!({
                    "events": [
                        {"id": 1}, {"id": 2}, {"id": 3}, {"id": 4}, {"id": 5}
                    ]
                }),
            )
            .await;

        let entity = cache.get("tokens/list", "abc123").await.unwrap();
        let events = entity["events"].as_array().unwrap();

        // Should only have last 3 (oldest evicted)
        assert_eq!(events.len(), 3);
        assert_eq!(events[0]["id"], 3);
        assert_eq!(events[1]["id"], 4);
        assert_eq!(events[2]["id"], 5);
    }

    #[tokio::test]
    async fn test_array_append_with_lru() {
        let config = EntityCacheConfig {
            max_entities_per_view: 1000,
            max_array_length: 3,
        };
        let cache = EntityCache::with_config(config);

        // Start with 2 events
        cache
            .upsert(
                "tokens/list",
                "abc123",
                &json!({
                    "events": [{"id": 1}, {"id": 2}]
                }),
            )
            .await;

        // Append 2 more (total 4, exceeds max 3)
        cache
            .upsert(
                "tokens/list",
                "abc123",
                &json!({
                    "events": [{"id": 3}, {"id": 4}]
                }),
            )
            .await;

        let entity = cache.get("tokens/list", "abc123").await.unwrap();
        let events = entity["events"].as_array().unwrap();

        // Should have last 3
        assert_eq!(events.len(), 3);
        assert_eq!(events[0]["id"], 2);
        assert_eq!(events[1]["id"], 3);
        assert_eq!(events[2]["id"], 4);
    }

    #[tokio::test]
    async fn test_entity_lru_eviction() {
        let config = EntityCacheConfig {
            max_entities_per_view: 2,
            max_array_length: 100,
        };
        let cache = EntityCache::with_config(config);

        // Add 3 entities (exceeds max of 2)
        cache.upsert("tokens/list", "key1", &json!({"id": 1})).await;
        cache.upsert("tokens/list", "key2", &json!({"id": 2})).await;
        cache.upsert("tokens/list", "key3", &json!({"id": 3})).await;

        // key1 should be evicted (LRU)
        assert!(cache.get("tokens/list", "key1").await.is_none());
        assert!(cache.get("tokens/list", "key2").await.is_some());
        assert!(cache.get("tokens/list", "key3").await.is_some());
    }

    #[tokio::test]
    async fn test_get_all() {
        let cache = EntityCache::new();

        cache.upsert("tokens/list", "key1", &json!({"id": 1})).await;
        cache.upsert("tokens/list", "key2", &json!({"id": 2})).await;

        let all = cache.get_all("tokens/list").await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_separate_views() {
        let cache = EntityCache::new();

        cache
            .upsert("tokens/list", "key1", &json!({"type": "token"}))
            .await;
        cache
            .upsert("games/list", "key1", &json!({"type": "game"}))
            .await;

        let token = cache.get("tokens/list", "key1").await.unwrap();
        let game = cache.get("games/list", "key1").await.unwrap();

        assert_eq!(token["type"], "token");
        assert_eq!(game["type"], "game");
    }

    #[test]
    fn test_deep_merge_function() {
        let base = json!({
            "a": 1,
            "b": {"c": 2},
            "arr": [1, 2]
        });

        let patch = json!({
            "b": {"d": 3},
            "arr": [3],
            "e": 4
        });

        let result = deep_merge_with_array_append(base, &patch, 100);

        assert_eq!(result["a"], 1);
        assert_eq!(result["b"]["c"], 2);
        assert_eq!(result["b"]["d"], 3);
        assert_eq!(result["arr"].as_array().unwrap().len(), 3);
        assert_eq!(result["e"], 4);
    }
}
