use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientMessage {
    Subscribe(Subscription),
    Unsubscribe(Unsubscription),
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subscription {
    pub view: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Unsubscription {
    pub view: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

impl Unsubscription {
    pub fn new(view: impl Into<String>) -> Self {
        Self {
            view: view.into(),
            key: None,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn sub_key(&self) -> String {
        format!("{}:{}", self.view, self.key.as_deref().unwrap_or("*"),)
    }
}

impl From<&Subscription> for Unsubscription {
    fn from(sub: &Subscription) -> Self {
        Self {
            view: sub.view.clone(),
            key: sub.key.clone(),
        }
    }
}

impl Subscription {
    pub fn new(view: impl Into<String>) -> Self {
        Self {
            view: view.into(),
            key: None,
            partition: None,
            filters: None,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn with_filters(mut self, filters: HashMap<String, String>) -> Self {
        self.filters = Some(filters);
        self
    }

    pub fn sub_key(&self) -> String {
        let filters_str = self
            .filters
            .as_ref()
            .map(|f| serde_json::to_string(f).unwrap_or_default())
            .unwrap_or_default();
        format!(
            "{}:{}:{}:{}",
            self.view,
            self.key.as_deref().unwrap_or("*"),
            self.partition.as_deref().unwrap_or(""),
            filters_str
        )
    }
}

#[derive(Debug, Default)]
pub struct SubscriptionRegistry {
    subscriptions: HashMap<String, Subscription>,
}

impl SubscriptionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, sub: Subscription) {
        let key = sub.sub_key();
        self.subscriptions.insert(key, sub);
    }

    pub fn remove(&mut self, sub: &Subscription) {
        let key = sub.sub_key();
        self.subscriptions.remove(&key);
    }

    pub fn contains(&self, sub: &Subscription) -> bool {
        let key = sub.sub_key();
        self.subscriptions.contains_key(&key)
    }

    pub fn all(&self) -> Vec<Subscription> {
        self.subscriptions.values().cloned().collect()
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.subscriptions.clear();
    }
}
