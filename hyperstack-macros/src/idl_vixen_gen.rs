//! IDL-based Vixen runtime generation.
//!
//! This module provides resolver registry generation for IDL-based streams.
//! The VmHandler and spec function generation is delegated to the unified
//! `codegen::vixen_runtime` module.

#![allow(dead_code)]

use crate::codegen::vixen_runtime::{self, RuntimeGenConfig};
use crate::parse::idl::*;
use crate::parse::{ResolverHookKind, ResolverHookSpec};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Path;

fn path_to_event_type(path: &Path, is_instruction: bool) -> String {
    let type_name = path
        .segments
        .last()
        .map(|seg| seg.ident.to_string())
        .unwrap_or_default();

    if is_instruction {
        format!("{}IxState", type_name)
    } else {
        format!("{}State", type_name)
    }
}

pub fn generate_resolver_registries(resolver_hooks: &[ResolverHookSpec]) -> TokenStream {
    let resolver_registry = generate_resolver_registry(resolver_hooks);
    let instruction_hook_registry = generate_instruction_hook_registry(resolver_hooks);

    quote! {
        #resolver_registry
        #instruction_hook_registry
    }
}

fn generate_resolver_registry(resolver_hooks: &[ResolverHookSpec]) -> TokenStream {
    let key_resolvers: Vec<_> = resolver_hooks
        .iter()
        .filter(|hook| matches!(hook.kind, ResolverHookKind::KeyResolver))
        .collect();

    if key_resolvers.is_empty() {
        return quote! {
            fn get_resolver_for_account_type(_account_type: &str) -> Option<fn(&str, &hyperstack::runtime::serde_json::Value, &mut hyperstack::runtime::hyperstack_interpreter::resolvers::ResolveContext) -> hyperstack::runtime::hyperstack_interpreter::resolvers::KeyResolution> {
                None
            }
        };
    }

    let resolver_arms = key_resolvers.iter().map(|hook| {
        let event_type = path_to_event_type(&hook.account_type_path, false);
        let fn_name = &hook.fn_name;

        quote! {
            #event_type => {
                Some(#fn_name)
            }
        }
    });

    quote! {
        fn get_resolver_for_account_type(account_type: &str) -> Option<fn(&str, &hyperstack::runtime::serde_json::Value, &mut hyperstack::runtime::hyperstack_interpreter::resolvers::ResolveContext) -> hyperstack::runtime::hyperstack_interpreter::resolvers::KeyResolution> {
            match account_type {
                #(#resolver_arms)*
                _ => None
            }
        }
    }
}

fn generate_instruction_hook_registry(resolver_hooks: &[ResolverHookSpec]) -> TokenStream {
    let instruction_hooks: Vec<_> = resolver_hooks
        .iter()
        .filter(|hook| matches!(hook.kind, ResolverHookKind::AfterInstruction))
        .collect();

    if instruction_hooks.is_empty() {
        return quote! {
            fn get_instruction_hooks(_instruction_type: &str) -> Vec<fn(&mut hyperstack::runtime::hyperstack_interpreter::resolvers::InstructionContext)> {
                Vec::new()
            }
        };
    }

    use std::collections::HashMap as StdHashMap;
    let mut hooks_by_instruction: StdHashMap<String, Vec<&syn::Ident>> = StdHashMap::new();

    for hook in &instruction_hooks {
        let event_type = path_to_event_type(&hook.account_type_path, true);
        hooks_by_instruction
            .entry(event_type)
            .or_default()
            .push(&hook.fn_name);
    }

    let hook_arms = hooks_by_instruction.iter().map(|(event_type, hook_fns)| {
        quote! {
            #event_type => {
                vec![#(#hook_fns),*]
            }
        }
    });

    quote! {
        fn get_instruction_hooks(instruction_type: &str) -> Vec<fn(&mut hyperstack::runtime::hyperstack_interpreter::resolvers::InstructionContext)> {
            match instruction_type {
                #(#hook_arms)*
                _ => Vec::new()
            }
        }
    }
}

pub fn generate_spec_function(
    idl: &IdlSpec,
    program_id: &str,
    resolver_hooks: &[ResolverHookSpec],
) -> TokenStream {
    let registries = generate_resolver_registries(resolver_hooks);
    let spec_fn = generate_spec_function_without_registries(idl, program_id);

    quote! {
        #registries
        #spec_fn
    }
}

pub fn generate_spec_function_without_registries(idl: &IdlSpec, _program_id: &str) -> TokenStream {
    let program_name = idl.get_name();
    let state_enum_name = format!("{}State", to_pascal_case(program_name));
    let instruction_enum_name = format!("{}Instruction", to_pascal_case(program_name));

    let config = RuntimeGenConfig::for_idl();

    let vm_handler =
        vixen_runtime::generate_vm_handler(&state_enum_name, &instruction_enum_name, program_name);

    let spec_fn = vixen_runtime::generate_spec_function(
        &state_enum_name,
        &instruction_enum_name,
        program_name,
        &config,
    );

    quote! {
        #vm_handler
        #spec_fn
    }
}
