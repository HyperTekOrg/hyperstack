//! Resolver and instruction hook registry generation.
//!
//! Generates the `get_resolver_for_account_type` and `get_instruction_hooks` functions.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::core::{generate_hook_actions, to_snake_case};
use crate::ast::*;
use crate::parse::idl::IdlSpec;

/// Generate resolver registry and instruction hook registry.
///
/// This creates the `get_resolver_for_account_type` and `get_instruction_hooks` functions.
pub fn generate_resolver_registries(
    resolver_hooks: &[ResolverHook],
    instruction_hooks: &[InstructionHook],
    idl: Option<&IdlSpec>,
) -> TokenStream {
    let resolver_registry = generate_resolver_registry(resolver_hooks);
    let instruction_hook_registry = generate_instruction_hook_registry(instruction_hooks, idl);

    quote! {
        #resolver_registry
        #instruction_hook_registry
    }
}

/// Generate the resolver registry match statement.
fn generate_resolver_registry(resolver_hooks: &[ResolverHook]) -> TokenStream {
    if resolver_hooks.is_empty() {
        return quote! {
            /// Get resolver function for a given account type (no resolvers registered)
            fn get_resolver_for_account_type(_account_type: &str) -> Option<fn(&str, &serde_json::Value, &mut hyperstack_interpreter::resolvers::ResolveContext) -> hyperstack_interpreter::resolvers::KeyResolution> {
                None
            }
        };
    }

    let resolver_arms = resolver_hooks.iter().map(|hook| {
        // account_type already includes the "State" suffix (e.g., "BondingCurveState")
        let event_type = hook.account_type.clone();
        
        // Generate the resolver function name based on account type (strip "State" suffix for snake_case)
        let account_type_base = event_type.strip_suffix("State").unwrap_or(&event_type);
        let _fn_name = format_ident!("resolve_{}_key", to_snake_case(account_type_base));

        match &hook.strategy {
            ResolverStrategy::PdaReverseLookup { lookup_name: _, queue_discriminators } => {
                let disc_bytes: Vec<u8> = queue_discriminators.iter().flatten().copied().collect();
                
                quote! {
                    #event_type => {
                        Some(|account_address: &str, _account_data: &serde_json::Value, ctx: &mut hyperstack_interpreter::resolvers::ResolveContext| {
                            if let Some(key) = ctx.pda_reverse_lookup(account_address) {
                                return hyperstack_interpreter::resolvers::KeyResolution::Found(key);
                            }
                            hyperstack_interpreter::resolvers::KeyResolution::QueueUntil(&[#(#disc_bytes),*])
                        })
                    }
                }
            }
            ResolverStrategy::DirectField { field_path } => {
                let segments = &field_path.segments;
                quote! {
                    #event_type => {
                        Some(|_account_address: &str, account_data: &serde_json::Value, _ctx: &mut hyperstack_interpreter::resolvers::ResolveContext| {
                            // Navigate to the field using the path segments
                            let mut current = account_data;
                            #(
                                current = current.get(#segments).unwrap_or(&serde_json::Value::Null);
                            )*
                            if let Some(key) = current.as_str() {
                                hyperstack_interpreter::resolvers::KeyResolution::Found(key.to_string())
                            } else {
                                hyperstack_interpreter::resolvers::KeyResolution::Found(String::new())
                            }
                        })
                    }
                }
            }
        }
    });

    quote! {
        /// Get resolver function for a given account type
        fn get_resolver_for_account_type(account_type: &str) -> Option<fn(&str, &serde_json::Value, &mut hyperstack_interpreter::resolvers::ResolveContext) -> hyperstack_interpreter::resolvers::KeyResolution> {
            match account_type {
                #(#resolver_arms)*
                _ => None
            }
        }
    }
}

/// Generate the instruction hook registry match statement.
fn generate_instruction_hook_registry(
    instruction_hooks: &[InstructionHook],
    _idl: Option<&IdlSpec>,
) -> TokenStream {
    if instruction_hooks.is_empty() {
        return quote! {
            /// Get instruction hooks for a given instruction type (no hooks registered)
            fn get_instruction_hooks(_instruction_type: &str) -> Vec<fn(&mut hyperstack_interpreter::resolvers::InstructionContext)> {
                Vec::new()
            }
        };
    }

    // Group hooks by instruction type
    // NOTE: instruction_type already includes "IxState" suffix from AST (e.g., "BuyIxState")
    use std::collections::HashMap;
    let mut hooks_by_instruction: HashMap<String, Vec<&InstructionHook>> = HashMap::new();
    for hook in instruction_hooks {
        // instruction_type already has "IxState" suffix - use it directly
        let event_type = hook.instruction_type.clone();
        hooks_by_instruction
            .entry(event_type)
            .or_default()
            .push(hook);
    }

    let hook_arms = hooks_by_instruction.iter().map(|(event_type, hooks)| {
        // Generate function definitions and names separately
        let (hook_fn_defs, hook_fn_names): (Vec<_>, Vec<_>) = hooks
            .iter()
            .enumerate()
            .map(|(idx, hook)| {
                // Strip "IxState" suffix for cleaner function names
                let instruction_base = hook
                    .instruction_type
                    .strip_suffix("IxState")
                    .unwrap_or(&hook.instruction_type);
                let fn_name = format_ident!("hook_{}_{}", to_snake_case(instruction_base), idx);
                let actions = generate_hook_actions(&hook.actions, &hook.lookup_by);

                let fn_def = quote! {
                    fn #fn_name(ctx: &mut hyperstack_interpreter::resolvers::InstructionContext) {
                        #actions
                    }
                };

                (fn_def, fn_name)
            })
            .unzip();

        quote! {
            #event_type => {
                // Define hook functions
                #(#hook_fn_defs)*
                // Return vector of function pointers
                vec![#(#hook_fn_names),*]
            }
        }
    });

    quote! {
        /// Get instruction hooks for a given instruction type
        fn get_instruction_hooks(instruction_type: &str) -> Vec<fn(&mut hyperstack_interpreter::resolvers::InstructionContext)> {
            match instruction_type {
                #(#hook_arms)*
                _ => Vec::new()
            }
        }
    }
}
