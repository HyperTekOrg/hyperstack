//! Prelude module for convenient imports.
//!
//! # Usage
//!
//! Import everything commonly needed in one line:
//!
//! ```rust,ignore
//! use hyperstack_sdk::prelude::*;
//!
//! let hs = HyperStack::connect("wss://example.com").await?;
//! let mut stream = hs.watch::<MyEntity>().await;
//! while let Some(update) = stream.next().await {
//!     // StreamExt methods available without separate import
//! }
//! ```

pub use crate::{
    Entity, EntityData, EntityStream, FilterMapStream, Filterable, FilteredStream, HyperStack,
    HyperStackBuilder, HyperStackConfig, HyperStackError, MapStream, RichEntityStream, RichUpdate,
    StateView, Update, ViewBuilder, ViewHandle, Views,
};

pub use futures_util::StreamExt;
