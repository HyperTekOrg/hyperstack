//! Module processing for hyperstack streams.
//!
//! This module handles processing of `#[hyperstack]` attributes applied to modules,
//! coordinating the processing of multiple entity structs within a module.

use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Item, ItemMod};

use crate::ast::SerializableStackSpec;
use crate::codegen::generate_multi_entity_builder;
use crate::parse;
use crate::parse::proto as proto_parser;
use crate::proto_codegen;
use crate::utils::to_pascal_case;

use super::entity::process_entity_struct;
use super::proto_struct::process_struct_with_context;

// ============================================================================
// Module Processing
// ============================================================================

/// Process a module annotated with `#[hyperstack(...)]`.
///
/// This handles:
/// - Proto-based streams with `proto = ["file.proto"]` attribute
/// - IDL-based streams with `idl = "file.json"` attribute
/// - Multi-entity modules with multiple `#[entity]` structs
pub fn process_module(mut module: ItemMod, attr: TokenStream) -> TokenStream {
    let mut section_structs = HashMap::new();
    let mut main_struct = None;
    let mut entity_structs = Vec::new();
    let mut has_game_event = false;

    let (proto_analyses, skip_decoders, idl_files) = parse_proto_files_from_attr(attr.clone());

    if !idl_files.is_empty() {
        return super::idl_spec::process_idl_spec(module, &idl_files);
    }

    if let Some((_, items)) = &module.content {
        for item in items {
            if let Item::Struct(item_struct) = item {
                if item_struct.ident == "GameEvent" {
                    has_game_event = true;
                }

                let has_stream = item_struct.attrs.iter().any(|attr| {
                    if attr.path().is_ident("derive") {
                        if let syn::Meta::List(meta_list) = &attr.meta {
                            return meta_list.tokens.to_string().contains("Stream");
                        }
                    }
                    false
                });

                let has_hyperstack = item_struct
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("hyperstack"));

                let has_entity = parse::has_entity_attribute(&item_struct.attrs);

                if has_entity {
                    entity_structs.push(item_struct.clone());
                } else if has_hyperstack {
                    main_struct = Some(item_struct.clone());
                } else if has_stream {
                    section_structs.insert(item_struct.ident.to_string(), item_struct.clone());
                } else if main_struct.is_none() && entity_structs.is_empty() {
                    main_struct = Some(item_struct.clone());
                }
            }
        }
    }

    if !entity_structs.is_empty() {
        let stack_name = to_pascal_case(&module.ident.to_string());
        let mut all_outputs = Vec::new();
        let mut entity_names = Vec::new();

        for entity_struct in &entity_structs {
            let entity_name = parse::parse_entity_name(&entity_struct.attrs)
                .unwrap_or_else(|| entity_struct.ident.to_string());
            entity_names.push(entity_name.clone());
            let output = process_entity_struct(
                entity_struct.clone(),
                entity_name,
                section_structs.clone(),
                has_game_event,
                &stack_name,
            );
            all_outputs.push(output);
        }

        if let Some((_brace, items)) = &mut module.content {
            items.retain(|item| {
                if let Item::Struct(s) = item {
                    !parse::has_entity_attribute(&s.attrs)
                } else {
                    true
                }
            });

            // Insert proto module declarations at the beginning
            if !proto_analyses.is_empty() {
                let proto_modules =
                    proto_codegen::generate_proto_module_declarations(&proto_analyses);
                if let Ok(generated_items) = syn::parse::<syn::File>(proto_modules.into()) {
                    for gen_item in generated_items.items.into_iter().rev() {
                        items.insert(0, gen_item);
                    }
                }
            }

            for output in &all_outputs {
                if let Ok(generated_items) = syn::parse::<syn::File>(output.token_stream.clone()) {
                    for gen_item in generated_items.items {
                        items.push(gen_item);
                    }
                }
            }

            let entity_asts: Vec<crate::ast::SerializableStreamSpec> = all_outputs
                .iter()
                .filter_map(|result| result.ast_spec.clone())
                .collect();

            let stack_spec = SerializableStackSpec {
                stack_name: stack_name.clone(),
                program_ids: vec![],
                idls: vec![],
                entities: entity_asts,
                pdas: std::collections::BTreeMap::new(),
                instructions: vec![],
                content_hash: None,
            }
            .with_content_hash();

            if let Err(e) = crate::ast::writer::write_stack_to_file(&stack_spec, &stack_name) {
                eprintln!("Warning: Failed to write stack AST: {}", e);
            }

            let multi_entity_builder = generate_multi_entity_builder(
                &entity_names,
                &proto_analyses,
                skip_decoders,
                &stack_name,
            );
            if let Ok(generated_items) = syn::parse::<syn::File>(multi_entity_builder.into()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }
        }

        quote! { #module }.into()
    } else if let Some(main) = main_struct {
        let output = process_struct_with_context(main, section_structs, has_game_event);

        if let Some((_brace, items)) = &mut module.content {
            items.retain(|item| {
                if let Item::Struct(s) = item {
                    !s.attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("hyperstack"))
                } else {
                    true
                }
            });

            if let Ok(generated_items) = syn::parse::<syn::File>(output.clone()) {
                for gen_item in generated_items.items {
                    items.push(gen_item);
                }
            }
        }

        quote! { #module }.into()
    } else {
        quote! { #module }.into()
    }
}

// ============================================================================
// Attribute Parsing
// ============================================================================

pub fn parse_proto_files_from_attr(
    attr: TokenStream,
) -> (
    Vec<(String, proto_parser::ProtoAnalysis)>,
    bool,
    Vec<String>,
) {
    let hyperstack_attr = match parse::parse_stream_spec_attribute(attr) {
        Ok(attr) => attr,
        Err(_) => return (Vec::new(), false, Vec::new()),
    };

    let idl_files = hyperstack_attr.idl_files.clone();

    if hyperstack_attr.proto_files.is_empty() {
        return (Vec::new(), hyperstack_attr.skip_decoders, idl_files);
    }

    let mut analyses = Vec::new();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    for proto_path in &hyperstack_attr.proto_files {
        let full_path = std::path::Path::new(&manifest_dir).join(proto_path);

        match proto_parser::parse_proto_file(&full_path) {
            Ok(analysis) => {
                analyses.push((proto_path.clone(), analysis));
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to parse proto file {} (full path: {:?}): {}",
                    proto_path, full_path, e
                );
            }
        }
    }

    (analyses, hyperstack_attr.skip_decoders, idl_files)
}
