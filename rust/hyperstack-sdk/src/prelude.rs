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
    Entity, EntityData, EntityStream, Filterable, HyperStack, HyperStackBuilder, HyperStackConfig,
    HyperStackError, RichEntityStream, RichUpdate, Update,
};

pub use futures_util::StreamExt;
