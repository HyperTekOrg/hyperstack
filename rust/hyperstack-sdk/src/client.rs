use crate::config::{ConnectionConfig, HyperStackConfig};
use crate::connection::{ConnectionManager, ConnectionState};
use crate::entity::Stack;
use crate::error::HyperStackError;
use crate::frame::Frame;
use crate::store::{SharedStore, StoreConfig};
use crate::view::Views;
use std::marker::PhantomData;
use std::time::Duration;
use tokio::sync::mpsc;

/// HyperStack client with typed views access.
///
/// ```ignore
/// use hyperstack_sdk::prelude::*;
/// use hyperstack_stacks::ore::OreStack;
///
/// let hs = HyperStack::<OreStack>::connect().await?;
/// let rounds = hs.views.latest().get().await;
/// ```
pub struct HyperStack<S: Stack> {
    connection: ConnectionManager,
    store: SharedStore,
    #[allow(dead_code)]
    config: HyperStackConfig,
    pub views: S::Views,
    _stack: PhantomData<S>,
}

impl<S: Stack> HyperStack<S> {
    /// Connect to the stack's default URL.
    pub async fn connect() -> Result<Self, HyperStackError> {
        Self::builder().connect().await
    }

    /// Connect with custom URL.
    pub async fn connect_url(url: &str) -> Result<Self, HyperStackError> {
        Self::builder().url(url).connect().await
    }

    /// Create a builder for custom configuration.
    pub fn builder() -> HyperStackBuilder<S> {
        HyperStackBuilder::new()
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

/// Builder for HyperStack with custom configuration.
pub struct HyperStackBuilder<S: Stack> {
    url: String,
    config: HyperStackConfig,
    _stack: PhantomData<S>,
}

impl<S: Stack> HyperStackBuilder<S> {
    fn new() -> Self {
        Self {
            url: S::url().to_string(),
            config: HyperStackConfig::default(),
            _stack: PhantomData,
        }
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = url.to_string();
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

    pub async fn connect(self) -> Result<HyperStack<S>, HyperStackError> {
        let store_config = StoreConfig {
            max_entries_per_view: self.config.max_entries_per_view,
        };
        let store = SharedStore::with_config(store_config);
        let store_clone = store.clone();

        let (frame_tx, mut frame_rx) = mpsc::channel::<Frame>(1000);

        let connection_config: ConnectionConfig = self.config.clone().into();
        let connection = ConnectionManager::new(self.url, connection_config, frame_tx).await;

        tokio::spawn(async move {
            while let Some(frame) = frame_rx.recv().await {
                store_clone.apply_frame(frame).await;
            }
        });

        let view_builder = crate::view::ViewBuilder::new(
            connection.clone(),
            store.clone(),
            self.config.initial_data_timeout,
        );
        let views = S::Views::from_builder(view_builder);

        Ok(HyperStack {
            connection,
            store,
            config: self.config,
            views,
            _stack: PhantomData,
        })
    }
}
