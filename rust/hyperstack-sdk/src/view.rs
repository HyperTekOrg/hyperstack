//! View abstractions for unified access to state, list, and derived views.
//!
//! Views provide a consistent API for accessing HyperStack data regardless of
//! whether it's a state view, list view, or derived view.
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
//! // Get latest round (derived single view)
//! let latest = views.latest().get().await;
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

/// Output mode for a view - determines if it returns single or multiple items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewOutput {
    /// View returns a single item (state view or derived single)
    Single,
    /// View returns multiple items (list view or derived collection)
    Collection,
}

/// A handle to a specific view that provides get/watch operations.
///
/// This is the main interface for interacting with views. It's generic over
/// the data type and whether it returns single or multiple items.
pub struct ViewHandle<T, const SINGLE: bool> {
    connection: ConnectionManager,
    store: SharedStore,
    view_path: String,
    entity_name: String,
    initial_data_timeout: Duration,
    _marker: PhantomData<T>,
}

impl<T> ViewHandle<T, true>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    /// Get the current value from this single-item view.
    ///
    /// For state views, this requires a key. For derived single views,
    /// the key is optional (defaults to fetching the single result).
    pub async fn get(&self) -> Option<T> {
        self.connection
            .ensure_subscription(&self.view_path, None)
            .await;
        self.store
            .wait_for_view_ready(&self.entity_name, self.initial_data_timeout)
            .await;

        // For single views, get the first (and only) item
        let items = self.store.list::<T>(&self.view_path).await;
        items.into_iter().next()
    }

    /// Get the current value by key from this single-item view.
    pub async fn get_by_key(&self, key: &str) -> Option<T> {
        self.connection
            .ensure_subscription(&self.view_path, Some(key))
            .await;
        self.store
            .wait_for_view_ready(&self.entity_name, self.initial_data_timeout)
            .await;
        self.store.get::<T>(&self.view_path, key).await
    }

    /// Watch for updates to this single-item view.
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

impl<T> ViewHandle<T, false>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    /// Get all items from this collection view.
    pub async fn get(&self) -> Vec<T> {
        self.connection
            .ensure_subscription(&self.view_path, None)
            .await;
        self.store
            .wait_for_view_ready(&self.entity_name, self.initial_data_timeout)
            .await;
        self.store.list::<T>(&self.view_path).await
    }

    /// Watch for updates to this collection view.
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
    /// Create a new view builder.
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

    /// Get the connection manager.
    pub fn connection(&self) -> &ConnectionManager {
        &self.connection
    }

    /// Get the shared store.
    pub fn store(&self) -> &SharedStore {
        &self.store
    }

    /// Get the initial data timeout.
    pub fn initial_data_timeout(&self) -> Duration {
        self.initial_data_timeout
    }

    /// Create a single-item view handle.
    pub fn single<T>(&self, view_path: &str, entity_name: &str) -> ViewHandle<T, true>
    where
        T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    {
        ViewHandle {
            connection: self.connection.clone(),
            store: self.store.clone(),
            view_path: view_path.to_string(),
            entity_name: entity_name.to_string(),
            initial_data_timeout: self.initial_data_timeout,
            _marker: PhantomData,
        }
    }

    /// Create a collection view handle.
    pub fn collection<T>(&self, view_path: &str, entity_name: &str) -> ViewHandle<T, false>
    where
        T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    {
        ViewHandle {
            connection: self.connection.clone(),
            store: self.store.clone(),
            view_path: view_path.to_string(),
            entity_name: entity_name.to_string(),
            initial_data_timeout: self.initial_data_timeout,
            _marker: PhantomData,
        }
    }
}

/// Trait for generated view accessor structs.
///
/// This trait is implemented by generated code (e.g., `OreRoundViews`) to provide
/// type-safe access to all views for an entity.
///
/// # Example Generated Code
///
/// ```ignore
/// pub struct OreRoundViews {
///     builder: ViewBuilder,
/// }
///
/// impl OreRoundViews {
///     pub fn new(hs: &HyperStack) -> Self {
///         Self {
///             builder: hs.view_builder(),
///         }
///     }
///
///     pub fn state(&self) -> StateView<OreRound> { ... }
///     pub fn list(&self) -> ViewHandle<OreRound, false> { ... }
///     pub fn latest(&self) -> ViewHandle<OreRound, true> { ... }
/// }
/// ```
pub trait Views: Sized {
    /// The entity type these views are for.
    type Entity: Entity;

    /// Create a new views accessor from a HyperStack client.
    fn from_builder(builder: ViewBuilder) -> Self;
}

/// A state view handle that requires a key for access.
pub struct StateView<T> {
    connection: ConnectionManager,
    store: SharedStore,
    view_path: String,
    entity_name: String,
    initial_data_timeout: Duration,
    _marker: PhantomData<T>,
}

impl<T> StateView<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    /// Create a new state view.
    pub fn new(
        connection: ConnectionManager,
        store: SharedStore,
        view_path: String,
        entity_name: String,
        initial_data_timeout: Duration,
    ) -> Self {
        Self {
            connection,
            store,
            view_path,
            entity_name,
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
            .wait_for_view_ready(&self.entity_name, self.initial_data_timeout)
            .await;
        self.store.get::<T>(&self.entity_name, key).await
    }

    /// Watch for updates to a specific key.
    pub fn watch(&self, key: &str) -> EntityStream<T> {
        EntityStream::new_lazy(
            self.connection.clone(),
            self.store.clone(),
            self.entity_name.clone(),
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
            self.entity_name.clone(),
            self.view_path.clone(),
            KeyFilter::Single(key.to_string()),
            Some(key.to_string()),
        )
    }
}
