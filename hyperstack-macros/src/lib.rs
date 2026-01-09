//! # hyperstack-macros
//!
//! Procedural macros for defining HyperStack streams.
//!
//! This crate provides the `#[hyperstack]` attribute macro that transforms
//! annotated Rust structs into full streaming pipeline specifications, including:
//!
//! - State struct generation with field accessors
//! - Handler creation functions for event processing
//! - IDL/Proto parser integration for Solana programs
//! - Automatic AST serialization for deployment
//!
//! ## Module Usage (IDL-based)
//!
//! ```rust,ignore
//! use hyperstack_macros::{hyperstack, Stream};
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
//!
//! ## Supported Attributes
//!
//! - `#[map(...)]` - Map from account fields
//! - `#[from_instruction(...)]` - Map from instruction fields
//! - `#[event(...)]` - Capture instruction events
//! - `#[snapshot(...)]` - Capture entire source data
//! - `#[aggregate(...)]` - Aggregate field values
//! - `#[computed(...)]` - Computed fields from other fields
//! - `#[derive_from(...)]` - Derive values from instructions

// Public modules - AST types needed for SDK generation
pub(crate) mod ast;

// Internal modules - not exposed publicly
mod codegen;
mod idl_codegen;
mod idl_parser_gen;
mod idl_vixen_gen;
mod parse;
mod proto_codegen;
mod stream_spec;
mod utils;

use proc_macro::TokenStream;
use std::collections::HashMap;
use syn::{parse_macro_input, ItemMod, ItemStruct};

// Use the stream_spec module functions
use stream_spec::{process_module, process_struct_with_context};

/// Process a `#[hyperstack(...)]` attribute.
///
/// This macro can be applied to:
/// - A module containing entity structs
/// - A single struct (legacy usage)
///
/// ## Module Usage (IDL-based)
///
/// ```rust,ignore
/// #[hyperstack(idl = "idl.json")]
/// pub mod my_stream {
///     #[entity(name = "MyEntity")]
///     struct Entity {
///         // fields with mapping attributes
///     }
/// }
/// ```
///
/// ## Proto-based Usage
///
/// ```rust,ignore
/// #[hyperstack(proto = ["events.proto"])]
/// pub mod my_stream {
///     // entity structs
/// }
/// ```
#[proc_macro_attribute]
pub fn hyperstack(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(module) = syn::parse::<ItemMod>(item.clone()) {
        return process_module(module, attr);
    }

    let input = parse_macro_input!(item as ItemStruct);
    process_struct_with_context(input, HashMap::new(), false)
}

/// Derive macro for `Stream`.
///
/// This is a marker derive that enables the following attributes on struct fields:
/// - `#[map(...)]` - Map from account fields
/// - `#[from_instruction(...)]` - Map from instruction fields
/// - `#[event(...)]` - Capture instruction events
/// - `#[snapshot(...)]` - Capture entire source
/// - `#[aggregate(...)]` - Aggregate field values
/// - `#[computed(...)]` - Computed fields from other fields
/// - `#[derive_from(...)]` - Derive values from instructions
#[proc_macro_derive(
    Stream,
    attributes(
        map,
        from_instruction,
        event,
        snapshot,
        aggregate,
        computed,
        derive_from
    )
)]
pub fn stream_derive(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
