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
//! hyperstack = "0.1"
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
