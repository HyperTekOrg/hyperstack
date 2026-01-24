//! Rust client SDK for connecting to HyperStack streaming servers.
//!
//! ```rust,ignore
//! use hyperstack_sdk::prelude::*;
//! use my_stack::{PumpfunToken, PumpfunTokenEntity};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let hs = HyperStack::connect("wss://mainnet.hyperstack.xyz").await?;
//!     
//!     if let Some(token) = hs.get::<PumpfunTokenEntity>("mint_address").await {
//!         println!("Token: {:?}", token);
//!     }
//!     
//!     let mut stream = hs.watch::<PumpfunTokenEntity>().await;
//!     while let Some(update) = stream.next().await {
//!         match update {
//!             Update::Upsert { key, data } => println!("Updated: {}", key),
//!             Update::Delete { key } => println!("Deleted: {}", key),
//!             _ => {}
//!         }
//!     }
//!     
//!     Ok(())
//! }
//! ```

mod client;
mod config;
mod connection;
mod entity;
mod error;
mod frame;
pub mod prelude;
mod store;
mod stream;
mod subscription;
pub mod view;

pub use client::{HyperStack, HyperStackBuilder};
pub use config::HyperStackConfig;
pub use connection::ConnectionState;
pub use entity::{Entity, EntityData, Filterable};
pub use error::HyperStackError;
pub use frame::{Frame, Mode, Operation};
pub use store::{SharedStore, StoreUpdate};
pub use stream::{
    EntityStream, FilterMapStream, FilteredStream, KeyFilter, MapStream, RichEntityStream,
    RichUpdate, Update,
};
pub use subscription::Subscription;
pub use view::{StateView, ViewBuilder, ViewHandle, Views};

pub use serde_json::Value;
