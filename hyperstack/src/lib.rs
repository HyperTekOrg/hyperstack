//! # HyperStack
//!
//! Real-time streaming data pipelines for Solana - transform on-chain events
//! into typed state projections.
//!
//! ## Features
//!
//! - **`interpreter`** (default) - AST transformation runtime and VM
//! - **`macros`** (default) - Proc-macros for defining stream specifications
//! - **`server`** (default) - WebSocket server and projection handlers
//! - **`sdk`** - Rust client for connecting to HyperStack servers
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//! hyperstack = "0.2"
//! ```
//!
//! Or with specific features:
//!
//! ```toml
//! [dependencies]
//! hyperstack = { version = "0.1", features = ["full"] }
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use hyperstack::prelude::*;
//!
//! #[hyperstack(idl = "idl.json")]
//! pub mod my_stream {
//!     #[entity(name = "MyEntity")]
//!     #[derive(Stream)]
//!     struct Entity {
//!         #[map(from = "MyAccount", field = "value")]
//!         pub value: u64,
//!     }
//! }
//! ```

// Re-export interpreter (AST runtime and VM)
#[cfg(feature = "interpreter")]
pub use hyperstack_interpreter as interpreter;

// Re-export macros
#[cfg(feature = "macros")]
pub use hyperstack_macros as macros;

// Re-export server components
#[cfg(feature = "server")]
pub use hyperstack_server as server;

// Re-export SDK client
#[cfg(feature = "sdk")]
pub use hyperstack_sdk as sdk;

#[cfg(feature = "runtime")]
#[doc(hidden)]
pub mod runtime {
    pub use anyhow;
    pub use bs58;
    pub use bytemuck;
    pub use dotenvy;
    pub use hyperstack_interpreter;
    pub use hyperstack_server;
    pub use serde;
    pub use serde_json;
    pub use smallvec;
    pub use tokio;
    pub use tracing;
    pub use yellowstone_vixen;
    pub use yellowstone_vixen_core;
    pub use yellowstone_vixen_yellowstone_grpc_source;

    pub mod serde_helpers {
        pub mod pubkey_base58 {
            use serde::{Deserialize, Deserializer, Serializer};

            pub fn serialize<S: Serializer>(bytes: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(&bs58::encode(bytes).into_string())
            }

            pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
                let s = String::deserialize(d)?;
                let bytes = bs58::decode(&s)
                    .into_vec()
                    .map_err(serde::de::Error::custom)?;
                let arr: [u8; 32] = bytes.try_into().map_err(|v: Vec<u8>| {
                    serde::de::Error::custom(format!("expected 32 bytes, got {}", v.len()))
                })?;
                Ok(arr)
            }
        }
    }
}

/// Prelude module for convenient imports
pub mod prelude {
    // Re-export commonly used items from interpreter
    #[cfg(feature = "interpreter")]
    pub use hyperstack_interpreter::{
        ast::{SerializableStreamSpec, TypedStreamSpec},
        compiler::MultiEntityBytecode,
        vm::VmContext,
        Mutation, UpdateContext,
    };

    #[cfg(feature = "macros")]
    pub use hyperstack_macros::{hyperstack, Stream};

    // Re-export server components
    #[cfg(feature = "server")]
    pub use hyperstack_server::{bus::BusManager, config::ServerConfig, projector::Projector};

    // Re-export SDK client
    #[cfg(feature = "sdk")]
    pub use hyperstack_sdk::HyperStack;
}
