//! Rust client SDK for connecting to HyperStack streaming servers.
//!
//! ```rust,ignore
//! use hyperstack_sdk::{HyperStack, Entity, Update};
//! use futures_util::StreamExt;
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
mod store;
mod stream;
mod subscription;

pub use client::{HyperStack, HyperStackBuilder};
pub use config::HyperStackConfig;
pub use connection::ConnectionState;
pub use entity::{Entity, Filterable};
pub use error::HyperStackError;
pub use frame::{Frame, Mode, Operation};
pub use store::{SharedStore, StoreUpdate};
pub use stream::{EntityStream, Update};
pub use subscription::Subscription;

pub use serde_json::Value;
