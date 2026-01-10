use crate::config::{ConnectionConfig, HyperStackConfig};
use crate::connection::{ConnectionManager, ConnectionState};
use crate::entity::Entity;
use crate::error::HyperStackError;
use crate::frame::Frame;
use crate::store::SharedStore;
use crate::stream::EntityStream;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct HyperStack {
    connection: ConnectionManager,
    store: SharedStore,
    #[allow(dead_code)]
    config: HyperStackConfig,
}

impl HyperStack {
    pub fn builder() -> HyperStackBuilder {
        HyperStackBuilder::default()
    }

    pub async fn connect(url: &str) -> Result<Self, HyperStackError> {
        Self::builder().url(url).connect().await
    }

    pub async fn get<E: Entity>(&self, key: &str) -> Option<E::Data> {
        self.connection
            .ensure_subscription(E::state_view(), Some(key))
            .await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.store.get::<E::Data>(E::NAME, key).await
    }

    pub async fn list<E: Entity>(&self) -> Vec<E::Data> {
        self.connection
            .ensure_subscription(E::list_view(), None)
            .await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.store.list::<E::Data>(E::NAME).await
    }

    pub async fn watch<E: Entity>(&self) -> EntityStream<E::Data> {
        self.connection
            .ensure_subscription(E::list_view(), None)
            .await;
        EntityStream::new(self.store.subscribe(), E::NAME.to_string())
    }

    pub async fn watch_key<E: Entity>(&self, key: &str) -> EntityStream<E::Data> {
        self.connection
            .ensure_subscription(E::kv_view(), Some(key))
            .await;
        EntityStream::new_filtered(self.store.subscribe(), E::NAME.to_string(), key.to_string())
    }

    pub async fn connection_state(&self) -> ConnectionState {
        self.connection.state().await
    }

    pub async fn disconnect(&self) {
        self.connection.disconnect().await;
    }

    pub fn store(&self) -> &SharedStore {
        &self.store
    }
}

#[derive(Default)]
pub struct HyperStackBuilder {
    url: Option<String>,
    config: HyperStackConfig,
}

impl HyperStackBuilder {
    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    pub fn auto_reconnect(mut self, enabled: bool) -> Self {
        self.config.auto_reconnect = enabled;
        self
    }

    pub fn reconnect_intervals(mut self, intervals: Vec<Duration>) -> Self {
        self.config.reconnect_intervals = intervals;
        self
    }

    pub fn max_reconnect_attempts(mut self, max: u32) -> Self {
        self.config.max_reconnect_attempts = max;
        self
    }

    pub fn ping_interval(mut self, interval: Duration) -> Self {
        self.config.ping_interval = interval;
        self
    }

    pub async fn connect(self) -> Result<HyperStack, HyperStackError> {
        let url = self.url.ok_or(HyperStackError::MissingUrl)?;
        let store = SharedStore::new();
        let store_clone = store.clone();

        let (frame_tx, mut frame_rx) = mpsc::channel::<Frame>(1000);

        let connection_config: ConnectionConfig = self.config.clone().into();
        let connection = ConnectionManager::new(url, connection_config, frame_tx).await;

        tokio::spawn(async move {
            while let Some(frame) = frame_rx.recv().await {
                store_clone.apply_frame(frame).await;
            }
        });

        Ok(HyperStack {
            connection,
            store,
            config: self.config,
        })
    }
}
