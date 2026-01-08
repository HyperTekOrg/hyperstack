//! # HyperStack
//!
//! Real-time streaming data pipelines for Solana - transform on-chain events
//! into typed state projections.
//!
//! ## Features
//!
//! - **`interpreter`** (default) - AST transformation runtime and VM
//! - **`spec-macros`** (default) - Proc-macros for defining stream specifications
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
//! // Define your stream specification using the proc-macro
//! stream_spec! {
//!     name: "MyToken",
//!     // ... specification
//! }
//! ```

// Re-export interpreter (AST runtime and VM)
#[cfg(feature = "interpreter")]
pub use hyperstack_interpreter as interpreter;

// Re-export spec macros
#[cfg(feature = "spec-macros")]
pub use hyperstack_spec_macros as spec_macros;

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

    // Re-export the stream_spec macro
    #[cfg(feature = "spec-macros")]
    pub use hyperstack_spec_macros::{stream_spec, StreamSection};

    // Re-export server components
    #[cfg(feature = "server")]
    pub use hyperstack_server::{
        bus::BusManager,
        config::ServerConfig,
        projector::Projector,
    };

    // Re-export SDK client
    #[cfg(feature = "sdk")]
    pub use hyperstack_sdk::HyperStackClient;
}
