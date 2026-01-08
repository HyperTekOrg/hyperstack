//! Stream spec processing module.
//!
//! This module handles the processing of `#[stream_spec]` macro attributes,
//! converting entity struct definitions into stream specifications.
//!
//! ## Module Structure
//!
//! - `module` - Entry point for module-level macro processing
//! - `entity` - Core entity struct processing logic (2,072 LOC)
//! - `handlers` - Handler generation for events and resolvers (559 LOC)
//! - `sections` - Nested struct and section processing (464 LOC)
//! - `computed` - Computed field expression parsing (461 LOC)
//! - `ast_writer` - AST JSON file generation at compile time (620 LOC)
//! - `idl_spec` - IDL-based spec processing (~300 LOC)
//! - `proto_struct` - Proto-based struct processing (~380 LOC)
//!
//! ## Processing Flow
//!
//! 1. `process_module` receives a module with `#[stream_spec]` attribute
//! 2. Parses IDL or proto files based on attribute arguments
//! 3. Processes each entity struct to extract field mappings
//! 4. Generates handler functions for each source type
//! 5. Writes AST JSON file for runtime consumption
//! 6. Returns generated code including state struct and spec function
//!
//! ## Example
//!
//! ```rust,ignore
//! #[stream_spec(idl = "idl.json")]
//! pub mod my_pipeline {
//!     #[entity(name = "MyEntity")]
//!     struct MyEntity {
//!         #[map(from = MyAccount::mint, primary_key)]
//!         pub mint: String,
//!     }
//! }
//! ```

mod ast_writer;
mod computed;
mod entity;
mod handlers;
mod idl_spec;
mod module;
mod proto_struct;
mod sections;

// Re-export module processing functions (used by lib.rs)
pub use module::process_module;

// Re-export proto struct processing (used by lib.rs)
pub use proto_struct::process_struct_with_context;
