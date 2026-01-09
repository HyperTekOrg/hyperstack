//! Parsing module for hyperstack-macros.
//!
//! This module contains all parsing logic including:
//! - `attributes` - Parsing of #[map], #[event], #[snapshot], etc. macro attributes
//! - `idl` - Parsing of Anchor IDL JSON files
//! - `proto` - Parsing of Protocol Buffer (.proto) files
//! - `conditions` - Parsing of condition expressions

pub mod attributes;
pub mod conditions;
pub mod idl;
pub mod proto;

// Re-export commonly used types from attributes module
pub use attributes::*;
