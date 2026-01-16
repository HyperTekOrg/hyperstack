//! IDL-based stream processing.
//!
//! This module handles processing of `#[hyperstack(idl = "...")]` modules,
//! which generate SDK types, parsers, and entity processing from an Anchor IDL file.

use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Item, ItemMod};

use crate::codegen::generate_multi_entity_builder;
use crate::idl_codegen;
use crate::idl_parser_gen;
use crate::idl_vixen_gen;
use crate::parse;
use crate::parse::idl as idl_parser;
use crate::utils::to_snake_case;

use super::entity::process_entity_struct_with_idl;

// ============================================================================
// IDL Spec Processing
// ============================================================================

/// Process a module with IDL-based spec.
///
/// This handles:
/// - Parsing the IDL file
/// - Generating SDK types from IDL accounts/instructions
/// - Generating Vixen parsers
/// - Processing entity structs with IDL context
/// - Generating resolver registries and runtime functions
pub fn process_idl_spec(mut module: ItemMod, idl_path: &str) -> TokenStream {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let full_path = std::path::Path::new(&manifest_dir).join(idl_path);

    let idl = match idl_parser::parse_idl_file(&full_path) {
        Ok(idl) => idl,
        Err(e) => {
            let error_msg = format!("Failed to parse IDL file {}: {}", idl_path, e);
            return quote! {
                compile_error!(#error_msg);
            }
            .into();
        }
    };

    // Get program ID from IDL address or metadata
    let program_id = idl
        .address
        .as_ref()
        .or_else(|| idl.metadata.as_ref().and_then(|m| m.address.as_ref()))
        .map(|s| s.as_str())
        .unwrap_or("11111111111111111111111111111111");

    // Generate IDL SDK types and parsers first
    let sdk_types = idl_codegen::generate_sdk_types(&idl);
    let parsers = idl_parser_gen::generate_parsers(&idl, program_id);

    // Now process entity structs (similar to proto path)
    let mut section_structs = HashMap::new();
    let mut entity_structs = Vec::new();
    let mut impl_blocks = Vec::new();
    let mut has_game_event = false;

    if let Some((_, items)) = &module.content {
        for item in items {
            if let Item::Struct(item_struct) = item {
                if item_struct.ident == "GameEvent" {
                    has_game_event = true;
                }

                let has_stream_section = item_struct.attrs.iter().any(|attr| {
                    if attr.path().is_ident("derive") {
                        if let syn::Meta::List(meta_list) = &attr.meta {
                            return meta_list.tokens.to_string().contains("Stream");
                        }
                    }
                    false
                });

                let has_entity = parse::has_entity_attribute(&item_struct.attrs);

                if has_entity {
                    entity_structs.push(item_struct.clone());
                } else if has_stream_section {
                    section_structs.insert(item_struct.ident.to_string(), item_struct.clone());
                }
            } else if let Item::Impl(impl_item) = item {
                // Collect impl blocks for resolver hook processing
                impl_blocks.push(impl_item.clone());
            }
        }
    }

    // Extract resolver hooks from impl blocks and standalone functions
    let mut all_resolver_hooks = Vec::new();
    for impl_block in &impl_blocks {
        let hooks = parse::extract_resolver_hooks(impl_block);
        all_resolver_hooks.extend(hooks);
    }

    // Also extract hooks from standalone functions
    if let Some((_, items)) = &module.content {
        for item in items {
            if let Item::Fn(item_fn) = item {
                let hooks = parse::extract_resolver_hooks_from_fn(item_fn);
                all_resolver_hooks.extend(hooks);
            }
        }
    }

    // Scan module items for declarative #[resolve_key] and #[register_pda] marker structs
    let mut resolver_hooks: Vec<parse::ResolveKeyAttribute> = Vec::new();
    let mut pda_registrations: Vec<parse::RegisterPdaAttribute> = Vec::new();

    if let Some((_, items)) = &module.content {
        for item in items {
            if let Item::Struct(item_struct) = item {
                // Check each struct for our declarative attributes
                for attr in &item_struct.attrs {
                    // Parse #[resolve_key(...)]
                    if let Ok(Some(resolve_attr)) = parse::parse_resolve_key_attribute(attr) {
                        resolver_hooks.push(resolve_attr);
                    }

                    // Parse #[register_pda(...)]
                    if let Ok(Some(register_attr)) = parse::parse_register_pda_attribute(attr) {
                        pda_registrations.push(register_attr);
                    }
                }
            }
        }
    }

    // Convert declarative #[resolve_key] attributes to ResolverHookSpec entries
    // so they get included in the resolver registry used by get_resolver_for_account_type
    for resolve_attr in &resolver_hooks {
        // Extract account type name from path (e.g., generated_sdk::accounts::BondingCurve -> BondingCurve)
        let account_name = resolve_attr
            .account_path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let fn_name = syn::Ident::new(
            &format!("resolve_{}_key", to_snake_case(&account_name)),
            proc_macro2::Span::call_site(),
        );

        // Create a dummy signature for the resolver function
        let fn_sig: syn::Signature = syn::parse_quote! {
            fn #fn_name(
                account_address: &str,
                _account_data: &serde_json::Value,
                ctx: &mut hyperstack_interpreter::resolvers::ResolveContext
            ) -> hyperstack_interpreter::resolvers::KeyResolution
        };

        all_resolver_hooks.push(parse::ResolverHookSpec {
            kind: parse::ResolverHookKind::KeyResolver,
            account_type_path: resolve_attr.account_path.clone(),
            fn_name,
            fn_sig,
        });
    }

    // Convert declarative #[register_pda] attributes to ResolverHookSpec entries
    // so they get included in the instruction hook registry used by get_instruction_hooks
    for (i, pda_attr) in pda_registrations.iter().enumerate() {
        let fn_name = syn::Ident::new(
            &format!("register_pda_{}", i),
            proc_macro2::Span::call_site(),
        );

        // Create a signature for the PDA registration function
        let fn_sig: syn::Signature = syn::parse_quote! {
            fn #fn_name(ctx: &mut hyperstack_interpreter::resolvers::InstructionContext)
        };

        all_resolver_hooks.push(parse::ResolverHookSpec {
            kind: parse::ResolverHookKind::AfterInstruction,
            account_type_path: pda_attr.instruction_path.clone(),
            fn_name,
            fn_sig,
        });
    }

    // Process entities and generate StreamSpec + bytecode
    if !entity_structs.is_empty() {
        let mut all_outputs = Vec::new();
        let mut entity_names = Vec::new();

        for entity_struct in &entity_structs {
            let entity_name = parse::parse_entity_name(&entity_struct.attrs)
                .unwrap_or_else(|| entity_struct.ident.to_string());
            entity_names.push(entity_name.clone());

            // Process entity with IDL context (use empty proto_analyses)
            let output = process_entity_struct_with_idl(
                entity_struct.clone(),
                entity_name,
                section_structs.clone(),
                has_game_event,
                Some(&idl),
                resolver_hooks.clone(),
                pda_registrations.clone(),
            );
            all_outputs.push(output);
        }

        // Remove entity structs and transform impl blocks with resolver hooks
        if let Some((_brace, items)) = &mut module.content {
            // First pass: remove entity structs and marker structs with declarative attributes
            items.retain(|item| {
                if let Item::Struct(s) = item {
                    // Remove entity structs
                    if parse::has_entity_attribute(&s.attrs) {
                        return false;
                    }
                    // Remove marker structs with #[resolve_key] or #[register_pda]
                    let has_declarative_attr = s.attrs.iter().any(|attr| {
                        attr.path().is_ident("resolve_key") || attr.path().is_ident("register_pda")
                    });
                    !has_declarative_attr
                } else {
                    true
                }
            });

            // Second pass: transform impl blocks and functions to remove resolver hook attributes
            for item in items.iter_mut() {
                if let Item::Impl(impl_item) = item {
                    for impl_item_inner in &mut impl_item.items {
                        if let syn::ImplItem::Fn(method) = impl_item_inner {
                            // Remove #[resolve_key_for] and #[after_instruction] attributes
                            method.attrs.retain(|attr| {
                                !attr.path().is_ident("resolve_key_for")
                                    && !attr.path().is_ident("after_instruction")
                            });
                        }
                    }
                } else if let Item::Fn(item_fn) = item {
                    // Remove #[resolve_key_for] and #[after_instruction] attributes from standalone functions
                    item_fn.attrs.retain(|attr| {
                        !attr.path().is_ident("resolve_key_for")
                            && !attr.path().is_ident("after_instruction")
                    });
                }
            }

            // Add SDK types and parsers first
            if let Ok(generated_items) = syn::parse::<syn::File>(sdk_types.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }

            if let Ok(generated_items) = syn::parse::<syn::File>(parsers.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }

            // Add entity processing outputs
            for output in all_outputs {
                if let Ok(generated_items) = syn::parse::<syn::File>(output) {
                    for gen_item in generated_items.items {
                        items.push(gen_item);
                    }
                }
            }

            // Generate multi-entity builder (without proto dependencies)
            let multi_entity_builder = generate_multi_entity_builder(&entity_names, &[], false);
            if let Ok(generated_items) = syn::parse::<syn::File>(multi_entity_builder.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }

            // Generate resolver registries (used by spec function)
            let resolver_registries =
                idl_vixen_gen::generate_resolver_registries(&all_resolver_hooks);
            if let Ok(generated_items) = syn::parse::<syn::File>(resolver_registries.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }

            // Generate spec function for hyperstack-server integration
            let spec_function =
                idl_vixen_gen::generate_spec_function_without_registries(&idl, program_id);
            if let Ok(generated_items) = syn::parse::<syn::File>(spec_function.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }
        }
    } else {
        // No entities - just add SDK and parsers
        if let Some((_brace, items)) = &mut module.content {
            if let Ok(generated_items) = syn::parse::<syn::File>(sdk_types.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }

            if let Ok(generated_items) = syn::parse::<syn::File>(parsers.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }
        }
    }

    quote! {
        #module
    }
    .into()
}
