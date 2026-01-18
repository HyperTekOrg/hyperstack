use crate::config::{ConnectionConfig, HyperStackConfig};
use crate::connection::{ConnectionManager, ConnectionState};
use crate::entity::{Entity, EntityData};
use crate::error::HyperStackError;
use crate::frame::Frame;
use crate::store::{SharedStore, StoreConfig};
use crate::stream::{EntityStream, KeyFilter, RichEntityStream};
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
        self.store
            .wait_for_view_ready(E::NAME, self.config.initial_data_timeout)
            .await;
        self.store.get::<E::Data>(E::NAME, key).await
    }

    pub async fn list<E: Entity>(&self) -> Vec<E::Data> {
        self.connection
            .ensure_subscription(E::list_view(), None)
            .await;
        self.store
            .wait_for_view_ready(E::NAME, self.config.initial_data_timeout)
            .await;
        self.store.list::<E::Data>(E::NAME).await
    }

    pub fn watch<E: Entity>(&self) -> EntityStream<E::Data> {
        EntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            E::NAME.to_string(),
            E::list_view().to_string(),
            KeyFilter::None,
            None,
        )
    }

    pub fn watch_key<E: Entity>(&self, key: &str) -> EntityStream<E::Data> {
        EntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            E::NAME.to_string(),
            E::list_view().to_string(),
            KeyFilter::Single(key.to_string()),
            Some(key.to_string()),
        )
    }

    pub fn watch_keys<E: Entity>(&self, keys: &[&str]) -> EntityStream<E::Data> {
        EntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            E::NAME.to_string(),
            E::list_view().to_string(),
            KeyFilter::Multiple(keys.iter().map(|s| s.to_string()).collect()),
            None,
        )
    }

    pub fn watch_rich<E: Entity>(&self) -> RichEntityStream<E::Data> {
        RichEntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            E::NAME.to_string(),
            E::list_view().to_string(),
            KeyFilter::None,
            None,
        )
    }

    pub fn watch_key_rich<E: Entity>(&self, key: &str) -> RichEntityStream<E::Data> {
        RichEntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            E::NAME.to_string(),
            E::list_view().to_string(),
            KeyFilter::Single(key.to_string()),
            Some(key.to_string()),
        )
    }

    pub async fn get_data<D: EntityData>(&self, key: &str) -> Option<D> {
        self.get::<D::Entity>(key).await
    }

    pub async fn list_data<D: EntityData>(&self) -> Vec<D> {
        self.list::<D::Entity>().await
    }

    pub fn watch_data<D: EntityData>(&self) -> EntityStream<D> {
        self.watch::<D::Entity>()
    }

    pub fn watch_key_data<D: EntityData>(&self, key: &str) -> EntityStream<D> {
        self.watch_key::<D::Entity>(key)
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

    pub fn initial_data_timeout(mut self, timeout: Duration) -> Self {
        self.config.initial_data_timeout = timeout;
        self
    }

    pub fn max_entries_per_view(mut self, max: usize) -> Self {
        self.config.max_entries_per_view = Some(max);
        self
    }

    pub fn unlimited_entries(mut self) -> Self {
        self.config.max_entries_per_view = None;
        self
    }

    pub async fn connect(self) -> Result<HyperStack, HyperStackError> {
        let url = self.url.ok_or(HyperStackError::MissingUrl)?;
        let store_config = StoreConfig {
            max_entries_per_view: self.config.max_entries_per_view,
        };
        let store = SharedStore::with_config(store_config);
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
