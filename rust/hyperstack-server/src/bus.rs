use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, watch, RwLock};

/// Message sent through the event bus
#[derive(Debug, Clone)]
pub struct BusMessage {
    pub key: String,
    pub entity: String,
    pub payload: Arc<Bytes>,
}

/// Manager for all event buses in the system
/// Supports multiple bus types for different streaming semantics
#[derive(Clone)]
pub struct BusManager {
    state_buses: Arc<RwLock<HashMap<(String, String), watch::Sender<Arc<Bytes>>>>>,
    kv_buses: Arc<RwLock<HashMap<String, broadcast::Sender<Arc<BusMessage>>>>>,
    list_buses: Arc<RwLock<HashMap<String, broadcast::Sender<Arc<BusMessage>>>>>,
    broadcast_capacity: usize,
}

impl BusManager {
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            state_buses: Arc::new(RwLock::new(HashMap::new())),
            kv_buses: Arc::new(RwLock::new(HashMap::new())),
            list_buses: Arc::new(RwLock::new(HashMap::new())),
            broadcast_capacity: capacity,
        }
    }

    /// Get or create a state bus (latest-value semantics)
    /// Each (view_id, key) pair gets its own watch channel
    pub async fn get_or_create_state_bus(
        &self,
        view_id: &str,
        key: &str,
    ) -> watch::Receiver<Arc<Bytes>> {
        let mut buses = self.state_buses.write().await;
        let entry = (view_id.to_string(), key.to_string());

        let tx = buses
            .entry(entry)
            .or_insert_with(|| {
                let empty = Arc::new(Bytes::new());
                watch::channel(empty).0
            })
            .clone();

        tx.subscribe()
    }

    /// Get or create a KV bus (key-value semantics with broadcast)
    pub async fn get_or_create_kv_bus(
        &self,
        view_id: &str,
    ) -> broadcast::Receiver<Arc<BusMessage>> {
        let mut buses = self.kv_buses.write().await;

        let tx = buses
            .entry(view_id.to_string())
            .or_insert_with(|| broadcast::channel(self.broadcast_capacity).0)
            .clone();

        tx.subscribe()
    }

    /// Get or create a list bus (append-only semantics)
    pub async fn get_or_create_list_bus(
        &self,
        view_id: &str,
    ) -> broadcast::Receiver<Arc<BusMessage>> {
        let mut buses = self.list_buses.write().await;

        let tx = buses
            .entry(view_id.to_string())
            .or_insert_with(|| broadcast::channel(self.broadcast_capacity).0)
            .clone();

        tx.subscribe()
    }

    /// Publish to a state bus (latest-value)
    pub async fn publish_state(&self, view_id: &str, key: &str, frame: Arc<Bytes>) {
        let buses = self.state_buses.read().await;
        if let Some(tx) = buses.get(&(view_id.to_string(), key.to_string())) {
            let _ = tx.send(frame);
        }
    }

    /// Publish to a KV bus
    pub async fn publish_kv(&self, view_id: &str, message: Arc<BusMessage>) {
        let buses = self.kv_buses.read().await;
        if let Some(tx) = buses.get(view_id) {
            let _ = tx.send(message);
        }
    }

    /// Publish to a list bus
    pub async fn publish_list(&self, view_id: &str, message: Arc<BusMessage>) {
        let buses = self.list_buses.read().await;
        if let Some(tx) = buses.get(view_id) {
            let _ = tx.send(message);
        }
    }
}

impl Default for BusManager {
    fn default() -> Self {
        Self::new()
    }
}
