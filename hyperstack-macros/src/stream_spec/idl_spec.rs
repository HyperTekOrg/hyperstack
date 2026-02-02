//! IDL-based stream processing.
//!
//! This module handles processing of `#[hyperstack(idl = "...")]` modules,
//! which generate SDK types, parsers, and entity processing from Anchor IDL files.
//! Supports multiple IDLs for multi-program stacks.

use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Item, ItemMod};

use crate::ast::SerializableStackSpec;
use crate::codegen::generate_multi_entity_builder;
use crate::idl_codegen;
use crate::idl_parser_gen;
use crate::idl_vixen_gen;
use crate::parse;
use crate::parse::idl as idl_parser;
use crate::utils::{to_pascal_case, to_snake_case};

use super::entity::process_entity_struct_with_idl;
use super::handlers::generate_auto_resolver_functions;

struct IdlInfo {
    idl: idl_parser::IdlSpec,
    program_id: String,
    program_name: String,
    sdk_module_name: String,
    parser_module_name: String,
}

pub fn process_idl_spec(mut module: ItemMod, idl_paths: &[String]) -> TokenStream {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    let mut idl_infos: Vec<IdlInfo> = Vec::new();

    for idl_path in idl_paths {
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

        let program_id = idl
            .address
            .as_ref()
            .or_else(|| idl.metadata.as_ref().and_then(|m| m.address.as_ref()))
            .map(|s| s.to_string())
            .unwrap_or_else(|| "11111111111111111111111111111111".to_string());

        let program_name = idl.get_name().to_string();
        let sdk_module_name = format!("{}_sdk", program_name);

        let parser_module_name = if idl_paths.len() == 1 {
            "parsers".to_string()
        } else {
            format!("{}_parsers", program_name)
        };

        idl_infos.push(IdlInfo {
            idl,
            program_id,
            program_name,
            sdk_module_name,
            parser_module_name,
        });
    }

    let primary = &idl_infos[0];

    let mut all_sdk_tokens: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut all_parser_tokens: Vec<proc_macro2::TokenStream> = Vec::new();

    for info in &idl_infos {
        let sdk_types = idl_codegen::generate_sdk_types(&info.idl, &info.sdk_module_name);
        all_sdk_tokens.push(sdk_types);

        let parsers = idl_parser_gen::generate_named_parsers(
            &info.idl,
            &info.program_id,
            &info.sdk_module_name,
            &info.parser_module_name,
        );
        all_parser_tokens.push(parsers);
    }

    let stack_name = to_pascal_case(&module.ident.to_string());

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
                impl_blocks.push(impl_item.clone());
            }
        }
    }

    let mut all_resolver_hooks = Vec::new();
    for impl_block in &impl_blocks {
        let hooks = parse::extract_resolver_hooks(impl_block);
        all_resolver_hooks.extend(hooks);
    }

    if let Some((_, items)) = &module.content {
        for item in items {
            if let Item::Fn(item_fn) = item {
                let hooks = parse::extract_resolver_hooks_from_fn(item_fn);
                all_resolver_hooks.extend(hooks);
            }
        }
    }

    let mut resolver_hooks: Vec<parse::ResolveKeyAttribute> = Vec::new();
    let mut pda_registrations: Vec<parse::RegisterPdaAttribute> = Vec::new();

    if let Some((_, items)) = &module.content {
        for item in items {
            if let Item::Struct(item_struct) = item {
                for attr in &item_struct.attrs {
                    if let Ok(Some(resolve_attr)) = parse::parse_resolve_key_attribute(attr) {
                        resolver_hooks.push(resolve_attr);
                    }

                    if let Ok(Some(register_attr)) = parse::parse_register_pda_attribute(attr) {
                        pda_registrations.push(register_attr);
                    }
                }
            }
        }
    }

    for resolve_attr in &resolver_hooks {
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

    for (i, pda_attr) in pda_registrations.iter().enumerate() {
        let fn_name = syn::Ident::new(
            &format!("register_pda_{}", i),
            proc_macro2::Span::call_site(),
        );

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

    if !entity_structs.is_empty() {
        let mut all_outputs = Vec::new();
        let mut entity_names = Vec::new();

        let idl_lookup: Vec<(String, &idl_parser::IdlSpec)> = idl_infos
            .iter()
            .map(|info| (info.sdk_module_name.clone(), &info.idl))
            .collect();

        for entity_struct in &entity_structs {
            let entity_name = parse::parse_entity_name(&entity_struct.attrs)
                .unwrap_or_else(|| entity_struct.ident.to_string());
            entity_names.push(entity_name.clone());

            let result = process_entity_struct_with_idl(
                entity_struct.clone(),
                entity_name,
                section_structs.clone(),
                has_game_event,
                &stack_name,
                &idl_lookup,
                resolver_hooks.clone(),
                pda_registrations.clone(),
            );

            for hook in &result.auto_resolver_hooks {
                let account_name =
                    crate::event_type_helpers::strip_event_type_suffix(&hook.account_type);
                let fn_name = syn::Ident::new(
                    &format!("resolve_{}_key", to_snake_case(account_name)),
                    proc_macro2::Span::call_site(),
                );
                let fn_sig: syn::Signature = syn::parse_quote! {
                    fn #fn_name(
                        account_address: &str,
                        _account_data: &serde_json::Value,
                        ctx: &mut hyperstack_interpreter::resolvers::ResolveContext
                    ) -> hyperstack_interpreter::resolvers::KeyResolution
                };
                let account_type_path: syn::Path =
                    syn::parse_str(account_name).unwrap_or_else(|_| syn::parse_quote!(#fn_name));
                all_resolver_hooks.push(parse::ResolverHookSpec {
                    kind: parse::ResolverHookKind::KeyResolver,
                    account_type_path,
                    fn_name,
                    fn_sig,
                });
            }

            all_outputs.push(result);
        }

        if let Some((_brace, items)) = &mut module.content {
            items.retain(|item| {
                if let Item::Struct(s) = item {
                    if parse::has_entity_attribute(&s.attrs) {
                        return false;
                    }
                    let has_declarative_attr = s.attrs.iter().any(|attr| {
                        attr.path().is_ident("resolve_key") || attr.path().is_ident("register_pda")
                    });
                    !has_declarative_attr
                } else {
                    true
                }
            });

            for item in items.iter_mut() {
                if let Item::Impl(impl_item) = item {
                    for impl_item_inner in &mut impl_item.items {
                        if let syn::ImplItem::Fn(method) = impl_item_inner {
                            method.attrs.retain(|attr| {
                                !attr.path().is_ident("resolve_key_for")
                                    && !attr.path().is_ident("after_instruction")
                            });
                        }
                    }
                } else if let Item::Fn(item_fn) = item {
                    item_fn.attrs.retain(|attr| {
                        !attr.path().is_ident("resolve_key_for")
                            && !attr.path().is_ident("after_instruction")
                    });
                }
            }

            for sdk_tokens in &all_sdk_tokens {
                if let Ok(generated_items) = syn::parse::<syn::File>(sdk_tokens.clone().into()) {
                    for gen_item in generated_items.items {
                        items.push(gen_item);
                    }
                }
            }

            for parser_tokens in &all_parser_tokens {
                if let Ok(generated_items) = syn::parse::<syn::File>(parser_tokens.clone().into()) {
                    for gen_item in generated_items.items {
                        items.push(gen_item);
                    }
                }
            }

            for result in &all_outputs {
                if let Ok(generated_items) = syn::parse::<syn::File>(result.token_stream.clone()) {
                    for gen_item in generated_items.items {
                        items.push(gen_item);
                    }
                }
                if !result.auto_resolver_hooks.is_empty() {
                    let auto_fns = generate_auto_resolver_functions(&result.auto_resolver_hooks);
                    if let Ok(generated_items) = syn::parse::<syn::File>(auto_fns.into()) {
                        for gen_item in generated_items.items {
                            items.push(gen_item);
                        }
                    }
                }
            }

            let entity_asts: Vec<crate::ast::SerializableStreamSpec> = all_outputs
                .iter()
                .filter_map(|result| result.ast_spec.clone())
                .collect();

            let all_program_ids: Vec<String> = idl_infos
                .iter()
                .map(|info| info.program_id.clone())
                .collect();

            let all_idl_snapshots: Vec<_> = entity_asts
                .first()
                .and_then(|e| e.idl.clone())
                .into_iter()
                .collect();

            let stack_spec = SerializableStackSpec {
                stack_name: stack_name.clone(),
                program_ids: all_program_ids,
                idls: all_idl_snapshots,
                entities: entity_asts
                    .into_iter()
                    .map(|mut e| {
                        e.idl = None;
                        e
                    })
                    .collect(),
                content_hash: None,
            }
            .with_content_hash();

            if let Err(e) = crate::ast::writer::write_stack_to_file(&stack_spec, &stack_name) {
                eprintln!("Warning: Failed to write stack AST: {}", e);
            }

            let multi_entity_builder =
                generate_multi_entity_builder(&entity_names, &[], false, &stack_name);
            if let Ok(generated_items) = syn::parse::<syn::File>(multi_entity_builder.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }

            let resolver_registries = idl_vixen_gen::generate_resolver_registries(
                &all_resolver_hooks,
                &primary.program_name,
            );
            if let Ok(generated_items) = syn::parse::<syn::File>(resolver_registries.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }

            let spec_function = idl_vixen_gen::generate_multi_idl_spec_function(
                &idl_infos
                    .iter()
                    .map(|info| {
                        (
                            &info.idl,
                            info.program_id.as_str(),
                            info.parser_module_name.as_str(),
                        )
                    })
                    .collect::<Vec<_>>(),
            );
            if let Ok(generated_items) = syn::parse::<syn::File>(spec_function.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }
        }
    } else {
        if let Some((_brace, items)) = &mut module.content {
            for sdk_tokens in &all_sdk_tokens {
                if let Ok(generated_items) = syn::parse::<syn::File>(sdk_tokens.clone().into()) {
                    for gen_item in generated_items.items {
                        items.push(gen_item);
                    }
                }
            }

            for parser_tokens in &all_parser_tokens {
                if let Ok(generated_items) = syn::parse::<syn::File>(parser_tokens.clone().into()) {
                    for gen_item in generated_items.items {
                        items.push(gen_item);
                    }
                }
            }
        }
    }

    quote! {
        #module
    }
    .into()
}
