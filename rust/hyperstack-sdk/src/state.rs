use crate::mutation::Mode;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

enum StoreData<T> {
    Kv(HashMap<String, T>),
    Append(Vec<T>),
    List(Vec<T>),
}

pub type Update<T> = (String, T);

pub struct EntityStore<T> {
    mode: Mode,
    data: Arc<RwLock<StoreData<T>>>,
    update_tx: broadcast::Sender<Update<T>>,
    _phantom: PhantomData<T>,
}

impl<T> EntityStore<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    pub fn new(mode: Mode) -> Self {
        let data = match mode {
            Mode::Kv | Mode::State => StoreData::Kv(HashMap::new()),
            Mode::Append => StoreData::Append(Vec::new()),
            Mode::List => StoreData::List(Vec::new()),
        };

        let (update_tx, _) = broadcast::channel(1000);

        Self {
            mode,
            data: Arc::new(RwLock::new(data)),
            update_tx,
            _phantom: PhantomData,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Update<T>> {
        self.update_tx.subscribe()
    }

    pub async fn get(&self, key: &str) -> Option<T> {
        let data = self.data.read().await;
        match &*data {
            StoreData::Kv(map) => map.get(key).cloned(),
            _ => None,
        }
    }

    pub async fn all(&self) -> HashMap<String, T> {
        let data = self.data.read().await;
        match &*data {
            StoreData::Kv(map) => map.clone(),
            _ => HashMap::new(),
        }
    }

    pub async fn all_vec(&self) -> Vec<T> {
        let data = self.data.read().await;
        match &*data {
            StoreData::Append(vec) | StoreData::List(vec) => vec.clone(),
            _ => Vec::new(),
        }
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub(crate) async fn apply_patch(&self, key: String, patch: Value) {
        let mut data = self.data.write().await;
        match &mut *data {
            StoreData::Kv(map) => {
                let current = map.get(&key).and_then(|v| serde_json::to_value(v).ok());
                let mut merged = current.unwrap_or_else(|| serde_json::json!({}));

                if let (Some(obj), Some(patch_obj)) = (merged.as_object_mut(), patch.as_object()) {
                    for (k, v) in patch_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }

                if let Ok(typed) = serde_json::from_value::<T>(merged) {
                    map.insert(key.clone(), typed.clone());
                    let _ = self.update_tx.send((key, typed));
                }
            }
            StoreData::List(vec) | StoreData::Append(vec) => {
                let item_data = patch.get("item").cloned().unwrap_or(patch);
                if let Ok(typed) = serde_json::from_value::<T>(item_data) {
                    vec.push(typed.clone());
                    let _ = self.update_tx.send((key, typed));
                }
            }
        }
    }

    pub(crate) async fn apply_upsert(&self, key: String, value: Value) {
        let mut data = self.data.write().await;
        
        if let Ok(typed) = serde_json::from_value::<T>(value) {
            match &mut *data {
                StoreData::Kv(map) => {
                    map.insert(key.clone(), typed.clone());
                    let _ = self.update_tx.send((key, typed));
                }
                StoreData::Append(vec) | StoreData::List(vec) => {
                    vec.push(typed.clone());
                    let _ = self.update_tx.send((key, typed));
                }
            }
        }
        
    }

    pub(crate) async fn apply_delete(&self, key: String) {
        let mut data = self.data.write().await;
        if let StoreData::Kv(map) = &mut *data {
            map.remove(&key);
        }
    }
}

impl<T> Default for EntityStore<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new(Mode::Kv)
    }
}

impl<T> Clone for EntityStore<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            mode: self.mode,
            data: self.data.clone(),
            update_tx: self.update_tx.clone(),
            _phantom: PhantomData,
        }
    }
}

