//! Proto-based struct processing for hyperstack streams.
//!
//! This module handles processing of structs with proto-based mapping attributes,
//! generating handler code and AST files for proto-based pipelines.

use std::collections::{HashMap, HashSet};

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Fields, ItemStruct, Type};

use crate::parse;
use crate::utils::{path_to_string, to_snake_case};

use super::handlers::{convert_event_to_map_attributes, determine_event_instruction};
use super::sections::{is_primitive_or_wrapper, process_nested_struct};
use super::entity::process_map_attribute;
use super::ast_writer::write_ast_at_compile_time;

// ============================================================================
// Proto Struct Processing
// ============================================================================

/// Process a struct with proto-based mapping attributes.
///
/// This is used for processing the main struct in a proto-based `#[stream_spec]` module
/// when there are no explicit `#[entity]` attributes.
pub fn process_struct_with_context(
    input: ItemStruct,
    section_structs: HashMap<String, ItemStruct>,
    skip_game_event: bool,
) -> TokenStream {
    let name = &input.ident;
    let state_name = syn::Ident::new(&format!("{}State", name), name.span());

    let mut field_mappings = Vec::new();
    let mut primary_keys = Vec::new();
    let mut lookup_indexes: Vec<(String, Option<String>)> = Vec::new();
    let mut accessor_defs = Vec::new();
    let mut accessor_names = HashSet::new();
    let mut state_fields = Vec::new();
    let mut sources_by_type: HashMap<String, Vec<parse::MapAttribute>> = HashMap::new();
    let mut events_by_instruction: HashMap<
        String,
        Vec<(String, parse::EventAttribute, syn::Type)>,
    > = HashMap::new();
    let mut has_events = false;
    let mut computed_fields: Vec<(String, proc_macro2::TokenStream, Type)> = Vec::new();
    let mut derive_from_mappings: HashMap<String, Vec<parse::DeriveFromAttribute>> = HashMap::new();
    let mut aggregate_conditions: HashMap<String, String> = HashMap::new();

    if let Fields::Named(fields) = &input.fields {
        for field in &fields.named {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;

            let mut has_attrs = false;
            for attr in &field.attrs {
                if let Ok(Some(map_attrs)) =
                    parse::parse_map_attribute(attr, &field_name.to_string())
                {
                    has_attrs = true;
                    for map_attr in map_attrs {
                        process_map_attribute(
                            &map_attr,
                            field_name,
                            field_type,
                            &mut state_fields,
                            &mut accessor_defs,
                            &mut accessor_names,
                            &mut primary_keys,
                            &mut lookup_indexes,
                            &mut sources_by_type,
                            &mut field_mappings,
                        );
                    }
                } else if let Ok(Some(map_attrs)) =
                    parse::parse_from_instruction_attribute(attr, &field_name.to_string())
                {
                    has_attrs = true;
                    for map_attr in map_attrs {
                        process_map_attribute(
                            &map_attr,
                            field_name,
                            field_type,
                            &mut state_fields,
                            &mut accessor_defs,
                            &mut accessor_names,
                            &mut primary_keys,
                            &mut lookup_indexes,
                            &mut sources_by_type,
                            &mut field_mappings,
                        );
                    }
                } else if let Ok(Some(mut event_attr)) =
                    parse::parse_event_attribute(attr, &field_name.to_string())
                {
                    has_attrs = true;
                    has_events = true;

                    state_fields.push(quote! {
                        pub #field_name: #field_type
                    });

                    // Determine instruction path (type-safe or legacy)
                    if let Some((_instruction_path, instruction_str)) =
                        determine_event_instruction(&mut event_attr, field_type)
                    {
                        events_by_instruction
                            .entry(instruction_str)
                            .or_default()
                            .push((
                                event_attr.target_field_name.clone(),
                                event_attr,
                                field_type.clone(),
                            ));
                    } else {
                        // Fallback to legacy instruction string
                        events_by_instruction
                            .entry(event_attr.instruction.clone())
                            .or_default()
                            .push((
                                event_attr.target_field_name.clone(),
                                event_attr,
                                field_type.clone(),
                            ));
                    }
                }
            }

            if !has_attrs && !is_primitive_or_wrapper(field_type) {
                if let Type::Path(type_path) = field_type {
                    if let Some(type_ident) = type_path.path.segments.last() {
                        let type_name = type_ident.ident.to_string();
                        if let Some(nested_struct) = section_structs.get(&type_name) {
                            process_nested_struct(
                                nested_struct,
                                field_name,
                                field_type,
                                &mut state_fields,
                                &mut accessor_defs,
                                &mut accessor_names,
                                &mut primary_keys,
                                &mut lookup_indexes,
                                &mut sources_by_type,
                                &mut field_mappings,
                                &mut events_by_instruction,
                                &mut has_events,
                                &mut computed_fields,
                                &mut derive_from_mappings,
                                &mut aggregate_conditions,
                            );
                        }
                    }
                }
            }
        }
    }

    let mut handler_fns = Vec::new();
    let mut handler_calls = Vec::new();

    // === HANDLER MERGING: Merge event mappings into sources_by_type ===
    // Convert events to map attributes and merge with sources_by_type (no IDL for proto-based specs)
    for event_mappings in events_by_instruction.values() {
        for (target_field, event_attr, _field_type) in event_mappings {
            // Get instruction path from event attribute
            let instruction_path = event_attr
                .from_instruction
                .as_ref()
                .or(event_attr.inferred_instruction.as_ref());

            if let Some(instr_path) = instruction_path {
                // Convert instruction path to string for sources_by_type key
                let source_type_str = path_to_string(instr_path);

                // Convert event to map attributes (without IDL)
                let map_attrs = convert_event_to_map_attributes(
                    target_field,
                    event_attr,
                    instr_path,
                    None, // No IDL for proto-based specs
                );

                // Merge into sources_by_type
                sources_by_type
                    .entry(source_type_str)
                    .or_default()
                    .extend(map_attrs);
            }
        }
    }

    let mut sources_by_type_and_join: HashMap<(String, Option<String>), Vec<parse::MapAttribute>> =
        HashMap::new();
    for (source_type, mappings) in &sources_by_type {
        for mapping in mappings {
            let key = (source_type.clone(), mapping.join_on.clone());
            sources_by_type_and_join
                .entry(key)
                .or_default()
                .push(mapping.clone());
        }
    }

    for ((source_type, join_key), mappings) in &sources_by_type_and_join {
        let handler_suffix = if let Some(ref join_field) = join_key {
            format!(
                "{}_{}",
                to_snake_case(source_type),
                to_snake_case(join_field)
            )
        } else {
            to_snake_case(source_type)
        };
        let handler_name = format_ident!("create_{}_handler", handler_suffix);
        let account_type = source_type.split("::").last().unwrap_or(source_type);

        // Check if any mapping is from an instruction
        let is_instruction = mappings.iter().any(|m| m.is_instruction);

        let mut field_mapping_code = Vec::new();
        let mut primary_field_path = quote! { hyperstack_interpreter::ast::FieldPath::new(&[]) };
        let mut has_primary_key = false;
        let mut lookup_primary_field = None;

        for mapping in mappings {
            let target_field = &mapping.target_field_name;
            let source_field = &mapping.source_field_name;
            let strategy_str = &mapping.strategy;
            let strategy_ident = format_ident!("{}", strategy_str);

            let mapping_expr = if mapping.is_whole_source && !is_instruction {
                // Whole account capture - use WholeSource for accounts (not instructions)
                quote! {
                    hyperstack_interpreter::ast::TypedFieldMapping::new(
                        #target_field.to_string(),
                        hyperstack_interpreter::ast::MappingSource::WholeSource,
                        hyperstack_interpreter::ast::PopulationStrategy::#strategy_ident,
                    )
                }
            } else if mapping.is_whole_source {
                // Whole instruction capture - use "data" path to capture entire instruction
                quote! {
                    hyperstack_interpreter::ast::TypedFieldMapping::new(
                        #target_field.to_string(),
                        hyperstack_interpreter::ast::MappingSource::FromSource {
                            path: hyperstack_interpreter::ast::FieldPath::new(&["data"]),
                            default: None,
                            transform: None,
                        },
                        hyperstack_interpreter::ast::PopulationStrategy::#strategy_ident,
                    )
                }
            } else {
                // Normal field mapping
                quote! {
                    hyperstack_interpreter::ast::TypedFieldMapping::new(
                        #target_field.to_string(),
                        hyperstack_interpreter::ast::MappingSource::FromSource {
                            path: hyperstack_interpreter::ast::FieldPath::new(&[#source_field]),
                            default: None,
                            transform: None,
                        },
                        hyperstack_interpreter::ast::PopulationStrategy::#strategy_ident,
                    )
                }
            };

            let mapping_expr = if let Some(ref transform_str) = mapping.transform {
                let transform_ident = format_ident!("{}", transform_str);
                quote! {
                    #mapping_expr.with_transform(hyperstack_interpreter::ast::Transformation::#transform_ident)
                }
            } else {
                mapping_expr
            };

            field_mapping_code.push(mapping_expr);

            if mapping.is_primary_key {
                has_primary_key = true;
                primary_field_path = quote! {
                    hyperstack_interpreter::ast::FieldPath::new(&[#source_field])
                };
            }

            if primary_keys.contains(&mapping.target_field_name) {
                lookup_primary_field = Some(quote! {
                    hyperstack_interpreter::ast::FieldPath::new(&[#source_field])
                });
            }
        }

        let key_resolution = if has_primary_key {
            quote! {
                hyperstack_interpreter::ast::KeyResolutionStrategy::Embedded {
                    primary_field: #primary_field_path,
                }
            }
        } else {
            let lookup_field = if let Some(ref join_field_name) = join_key {
                quote! {
                    hyperstack_interpreter::ast::FieldPath::new(&[#join_field_name])
                }
            } else {
                lookup_primary_field.unwrap_or_else(|| {
                    if let Some(pk) = primary_keys.first() {
                        for mapping in mappings {
                            if mapping.target_field_name == *pk {
                                let source_field = &mapping.source_field_name;
                                return quote! {
                                    hyperstack_interpreter::ast::FieldPath::new(&[#source_field])
                                };
                            }
                        }
                        let event_field = pk.split('.').next_back().unwrap_or(pk);
                        return quote! {
                            hyperstack_interpreter::ast::FieldPath::new(&[#event_field])
                        };
                    }
                    quote! { hyperstack_interpreter::ast::FieldPath::new(&[]) }
                })
            };

            quote! {
                hyperstack_interpreter::ast::KeyResolutionStrategy::Lookup {
                    primary_field: #lookup_field,
                }
            }
        };

        let type_suffix = if is_instruction { "IxState" } else { "State" };
        handler_fns.push(quote! {
            fn #handler_name() -> hyperstack_interpreter::ast::TypedHandlerSpec<#state_name> {
                hyperstack_interpreter::ast::TypedHandlerSpec::new(
                    hyperstack_interpreter::ast::SourceSpec::Source {
                        program_id: None,
                        discriminator: None,
                        type_name: format!("{}{}", #account_type, #type_suffix),
                    },
                    #key_resolution,
                    vec![
                        #(#field_mapping_code),*
                    ],
                    true,
                )
            }
        });

        handler_calls.push(quote! {
            #handler_name()
        });
    }

    let game_event_struct = if has_events && !skip_game_event {
        quote! {
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            pub struct GameEvent {
                pub timestamp: i64,
                #[serde(flatten)]
                pub data: serde_json::Value,
            }
        }
    } else {
        quote! {}
    };

    let lookup_index_creations: Vec<_> = lookup_indexes
        .iter()
        .map(|(field_name, temporal_field)| {
            if let Some(tf) = temporal_field {
                quote! {
                    hyperstack_interpreter::ast::LookupIndexSpec {
                        field_name: #field_name.to_string(),
                        temporal_field: Some(#tf.to_string()),
                    }
                }
            } else {
                quote! {
                    hyperstack_interpreter::ast::LookupIndexSpec {
                        field_name: #field_name.to_string(),
                        temporal_field: None,
                    }
                }
            }
        })
        .collect();

    // Write AST file at compile time (during macro expansion)
    write_ast_at_compile_time(
        &name.to_string(),
        &primary_keys,
        &lookup_indexes,
        &sources_by_type,
        &events_by_instruction,
        &[],             // No resolver hooks for proto-based specs
        &[],             // No PDA registrations for proto-based specs
        &HashMap::new(), // No derive_from mappings for proto-based specs
        &HashMap::new(), // No aggregate conditions for proto-based specs
        &[],             // No computed fields for proto-based specs
        &[],             // Empty sections for proto-based specs
        None,            // No IDL for proto-based specs
    );

    let output = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct #state_name {
            #(#state_fields),*
        }

        #game_event_struct

        pub mod fields {
            use super::*;

            #(#accessor_defs)*
        }

        pub fn create_spec() -> hyperstack_interpreter::ast::TypedStreamSpec<#state_name> {
            hyperstack_interpreter::ast::TypedStreamSpec::new(
                stringify!(#name).to_string(),
                hyperstack_interpreter::ast::IdentitySpec {
                    primary_keys: vec![#(#primary_keys.to_string()),*],
                    lookup_indexes: vec![
                        #(#lookup_index_creations),*
                    ],
                },
                vec![
                    #(#handler_calls),*
                ],
            )
        }

        #(#handler_fns)*
    };

    output.into()
}
