//! Code generation module for hyperstack-macros (internal).
//!
//! This module consolidates all code generation logic used by `#[hyperstack]`.
//! All submodules are internal and not exposed publicly.

// Internal submodules - not exposed publicly
mod bytecode;
pub(crate) mod computed;
pub(crate) mod core;
mod generate_all;
mod handlers;
mod multi_entity;
mod parsers;
mod resolvers;
mod sdk;
mod spec_fn;
mod vm_handler;

// Internal re-exports for use within this crate only
pub(crate) use bytecode::generate_bytecode_from_spec;
pub(crate) use computed::generate_computed_evaluator;
pub(crate) use computed::generate_computed_expr_code;
pub(crate) use handlers::generate_handlers_from_specs;
pub(crate) use multi_entity::generate_multi_entity_builder;
pub(crate) use parsers::generate_parsers_from_idl;
pub(crate) use resolvers::generate_resolver_registries;
pub(crate) use sdk::generate_sdk_from_idl;
pub(crate) use spec_fn::generate_spec_function;
pub(crate) use vm_handler::generate_vm_handler;
