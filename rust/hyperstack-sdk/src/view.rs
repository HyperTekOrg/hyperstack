//! View abstractions for unified access to views.
//!
//! All views return collections (Vec<T>). Use `.first()` on the result
//! if you need a single item.
//!
//! # Example
//!
//! ```ignore
//! use hyperstack_sdk::prelude::*;
//! use my_stack::OreRoundViews;
//!
//! let hs = HyperStack::connect("wss://example.com").await?;
//!
//! // Access views through the generated views struct
//! let views = OreRoundViews::new(&hs);
//!
//! // Get latest round - use .first() for single item
//! let latest = views.latest().get().await.first().cloned();
//!
//! // List all rounds
//! let rounds = views.list().get().await;
//!
//! // Get specific round by key
//! let round = views.state().get("round_key").await;
//!
//! // Watch for updates
//! let mut stream = views.latest().watch();
//! while let Some(update) = stream.next().await {
//!     println!("Latest round updated: {:?}", update);
//! }
//! ```

use crate::connection::ConnectionManager;
use crate::entity::Entity;
use crate::store::SharedStore;
use crate::stream::{EntityStream, KeyFilter, RichEntityStream};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use std::time::Duration;

/// A handle to a view that provides get/watch operations.
///
/// All views return collections (Vec<T>). Use `.first()` on the result
/// if you need a single item from views with a `take` limit.
pub struct ViewHandle<T> {
    connection: ConnectionManager,
    store: SharedStore,
    view_path: String,
    initial_data_timeout: Duration,
    _marker: PhantomData<T>,
}

impl<T> ViewHandle<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    /// Get all items from this view.
    ///
    /// For views with a `take` limit defined in the stack, this returns
    /// up to that many items. Use `.first()` on the result if you need
    /// a single item.
    pub async fn get(&self) -> Vec<T> {
        self.connection
            .ensure_subscription(&self.view_path, None)
            .await;
        self.store
            .wait_for_view_ready(&self.view_path, self.initial_data_timeout)
            .await;
        self.store.list::<T>(&self.view_path).await
    }

    /// Watch for updates to this view.
    pub fn watch(&self) -> EntityStream<T> {
        EntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            self.view_path.clone(),
            self.view_path.clone(),
            KeyFilter::None,
            None,
        )
    }

    /// Watch for updates filtered to specific keys.
    pub fn watch_keys(&self, keys: &[&str]) -> EntityStream<T> {
        EntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            self.view_path.clone(),
            self.view_path.clone(),
            KeyFilter::Multiple(keys.iter().map(|s| s.to_string()).collect()),
            None,
        )
    }

    /// Watch for updates with before/after diffs.
    pub fn watch_rich(&self) -> RichEntityStream<T> {
        RichEntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            self.view_path.clone(),
            self.view_path.clone(),
            KeyFilter::None,
            None,
        )
    }
}

/// Builder for creating view handles.
///
/// This is used internally by generated code to create properly configured view handles.
pub struct ViewBuilder {
    connection: ConnectionManager,
    store: SharedStore,
    initial_data_timeout: Duration,
}

impl ViewBuilder {
    pub fn new(
        connection: ConnectionManager,
        store: SharedStore,
        initial_data_timeout: Duration,
    ) -> Self {
        Self {
            connection,
            store,
            initial_data_timeout,
        }
    }

    pub fn connection(&self) -> &ConnectionManager {
        &self.connection
    }

    pub fn store(&self) -> &SharedStore {
        &self.store
    }

    pub fn initial_data_timeout(&self) -> Duration {
        self.initial_data_timeout
    }

    /// Create a view handle.
    pub fn view<T>(&self, view_path: &str) -> ViewHandle<T>
    where
        T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    {
        ViewHandle {
            connection: self.connection.clone(),
            store: self.store.clone(),
            view_path: view_path.to_string(),
            initial_data_timeout: self.initial_data_timeout,
            _marker: PhantomData,
        }
    }
}

/// Trait for generated view accessor structs.
///
/// This trait is implemented by generated code (e.g., `OreRoundViews`) to provide
/// type-safe access to all views for an entity.
pub trait Views: Sized {
    type Entity: Entity;

    fn from_builder(builder: ViewBuilder) -> Self;
}

/// A state view handle that requires a key for access.
pub struct StateView<T> {
    connection: ConnectionManager,
    store: SharedStore,
    view_path: String,
    initial_data_timeout: Duration,
    _marker: PhantomData<T>,
}

impl<T> StateView<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    pub fn new(
        connection: ConnectionManager,
        store: SharedStore,
        view_path: String,
        initial_data_timeout: Duration,
    ) -> Self {
        Self {
            connection,
            store,
            view_path,
            initial_data_timeout,
            _marker: PhantomData,
        }
    }

    /// Get an entity by key.
    pub async fn get(&self, key: &str) -> Option<T> {
        self.connection
            .ensure_subscription(&self.view_path, Some(key))
            .await;
        self.store
            .wait_for_view_ready(&self.view_path, self.initial_data_timeout)
            .await;
        self.store.get::<T>(&self.view_path, key).await
    }

    /// Watch for updates to a specific key.
    pub fn watch(&self, key: &str) -> EntityStream<T> {
        EntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            self.view_path.clone(),
            self.view_path.clone(),
            KeyFilter::Single(key.to_string()),
            Some(key.to_string()),
        )
    }

    /// Watch for updates with before/after diffs.
    pub fn watch_rich(&self, key: &str) -> RichEntityStream<T> {
        RichEntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            self.view_path.clone(),
            self.view_path.clone(),
            KeyFilter::Single(key.to_string()),
            Some(key.to_string()),
        )
    }
}
