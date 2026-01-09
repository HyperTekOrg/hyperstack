//! # hyperstack-sdk
//!
//! Rust client SDK for connecting to HyperStack streaming servers.
//!
//! This crate provides a WebSocket client for subscribing to real-time
//! entity updates from HyperStack servers.
//!
//! ## Example
//!
//! ```rust,ignore
//! use hyperstack_sdk::{HyperStackClient, Subscription};
//!
//! let client = HyperStackClient::connect("ws://localhost:8877").await?;
//! let sub = client.subscribe("MyEntity/kv", Some(key)).await?;
//!
//! while let Some(frame) = sub.next().await {
//!     println!("Update: {:?}", frame);
//! }
//! ```
//!
//! ## Streaming Modes
//!
//! - **State** - Single shared state object
//! - **KV** - Key-value lookups by entity key
//! - **List** - All entities matching filters
//! - **Append** - Append-only event log

mod client;
mod mutation;
mod state;

pub use client::{HyperStackClient, Subscription};
pub use mutation::{Frame, Mode};
pub use state::EntityStore;

pub use serde_json::Value;
