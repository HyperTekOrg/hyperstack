//! Section processing for stream specs.
//!
//! This module handles processing of nested struct sections, including:
//! - Section extraction from struct definitions
//! - Field type analysis
//! - Nested struct attribute processing

use std::collections::{HashMap, HashSet};

use quote::quote;
use syn::{Fields, ItemStruct, Type};

use crate::ast::{BaseType, EntitySection, FieldTypeInfo};
use crate::parse;
use crate::utils::path_to_string;

use super::handlers::{determine_event_instruction, extract_account_type_from_field};

// ============================================================================
// Section Extraction
// ============================================================================

/// Extract section information from a struct definition.
pub fn extract_section_from_struct(
    section_name: &str,
    item_struct: &ItemStruct,
    parent_field: Option<String>,
) -> EntitySection {
    let mut fields = Vec::new();

    if let Fields::Named(struct_fields) = &item_struct.fields {
        for field in &struct_fields.named {
            if let Some(field_ident) = &field.ident {
                let field_name = field_ident.to_string();
                let field_ty = &field.ty;
                let rust_type_name = quote::quote!(#field_ty).to_string();
                let field_type_info = analyze_field_type(&field_name, &rust_type_name);
                fields.push(field_type_info);
            }
        }
    }

    EntitySection {
        name: section_name.to_string(),
        fields,
        is_nested_struct: parent_field.is_some(),
        parent_field,
    }
}

// ============================================================================
// Field Type Analysis
// ============================================================================

/// Analyze a Rust type string and extract field type information.
pub fn analyze_field_type(field_name: &str, rust_type: &str) -> FieldTypeInfo {
    let type_str = rust_type.trim();

    // Handle Option<T>
    if let Some(inner) = extract_generic_inner_type(type_str, "Option") {
        let inner_info = analyze_inner_type(&inner);
        return FieldTypeInfo {
            field_name: field_name.to_string(),
            rust_type_name: rust_type.to_string(),
            base_type: infer_semantic_type(field_name, inner_info.0),
            is_optional: true,
            is_array: inner_info.1,
            inner_type: Some(inner),
            source_path: None,
        };
    }

    // Handle Vec<T>
    if let Some(inner) = extract_generic_inner_type(type_str, "Vec") {
        let _inner_base_type = analyze_simple_type(&inner);
        return FieldTypeInfo {
            field_name: field_name.to_string(),
            rust_type_name: rust_type.to_string(),
            base_type: BaseType::Array,
            is_optional: false,
            is_array: true,
            inner_type: Some(inner),
            source_path: None,
        };
    }

    // Handle primitive types
    let base_type = analyze_simple_type(type_str);
    FieldTypeInfo {
        field_name: field_name.to_string(),
        rust_type_name: rust_type.to_string(),
        base_type: infer_semantic_type(field_name, base_type),
        is_optional: false,
        is_array: false,
        inner_type: None,
        source_path: None,
    }
}

/// Analyze inner type and return (BaseType, is_array).
fn analyze_inner_type(type_str: &str) -> (BaseType, bool) {
    if let Some(_vec_inner) = extract_generic_inner_type(type_str, "Vec") {
        (BaseType::Array, true)
    } else {
        (analyze_simple_type(type_str), false)
    }
}

/// Analyze a simple (non-generic) type string.
fn analyze_simple_type(type_str: &str) -> BaseType {
    match type_str {
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            BaseType::Integer
        }
        "f32" | "f64" => BaseType::Float,
        "bool" => BaseType::Boolean,
        "String" | "&str" | "str" => BaseType::String,
        "Value" | "serde_json::Value" => BaseType::Any,
        _ => {
            if type_str.contains("Bytes") || type_str.contains("bytes") {
                BaseType::Binary
            } else {
                BaseType::Object
            }
        }
    }
}

/// Extract inner type from generic like "Option<T>" -> "T".
fn extract_generic_inner_type(type_str: &str, generic_name: &str) -> Option<String> {
    let pattern = format!("{} <", generic_name);
    let pattern_no_space = format!("{}<", generic_name);

    // Try with space (how quote renders it: "Option < u64 >")
    if type_str.starts_with(&pattern) && type_str.ends_with('>') {
        let start = pattern.len();
        let end = type_str.len() - 1;
        if end > start {
            return Some(type_str[start..end].trim().to_string());
        }
    }

    // Try without space (standard Rust syntax: "Option<u64>")
    if type_str.starts_with(&pattern_no_space) && type_str.ends_with('>') {
        let start = pattern_no_space.len();
        let end = type_str.len() - 1;
        if end > start {
            return Some(type_str[start..end].trim().to_string());
        }
    }

    None
}

/// Infer semantic type based on field name patterns.
fn infer_semantic_type(field_name: &str, base_type: BaseType) -> BaseType {
    let lower_name = field_name.to_lowercase();

    // If already classified as integer, check if it should be timestamp
    if base_type == BaseType::Integer {
        if lower_name.ends_with("_at")
            || lower_name.ends_with("_time")
            || lower_name.contains("timestamp")
            || lower_name.contains("created")
            || lower_name.contains("settled")
            || lower_name.contains("activated")
        {
            return BaseType::Timestamp;
        }
    }

    base_type
}

// ============================================================================
// Primitive Type Checking
// ============================================================================

/// Check if a type is a primitive or common wrapper type.
///
/// Returns true for numeric types, bool, String, Option, and Vec.
/// These types don't represent nested structs that need special processing.
pub fn is_primitive_or_wrapper(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();
                matches!(
                    type_name.as_str(),
                    "u8" | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "String"
                        | "Option"
                        | "Vec"
                )
            } else {
                false
            }
        }
        _ => true,
    }
}

// ============================================================================
// Nested Struct Processing
// ============================================================================

/// Process a nested struct (section) and extract all its field mappings.
#[allow(clippy::too_many_arguments)]
pub fn process_nested_struct(
    nested_struct: &ItemStruct,
    section_field_name: &syn::Ident,
    section_field_type: &Type,
    state_fields: &mut Vec<proc_macro2::TokenStream>,
    accessor_defs: &mut Vec<proc_macro2::TokenStream>,
    accessor_names: &mut HashSet<String>,
    primary_keys: &mut Vec<String>,
    lookup_indexes: &mut Vec<(String, Option<String>)>,
    sources_by_type: &mut HashMap<String, Vec<parse::MapAttribute>>,
    field_mappings: &mut Vec<parse::MapAttribute>,
    events_by_instruction: &mut HashMap<String, Vec<(String, parse::EventAttribute, Type)>>,
    has_events: &mut bool,
    computed_fields: &mut Vec<(String, proc_macro2::TokenStream, Type)>,
    track_from_mappings: &mut HashMap<String, Vec<parse::TrackFromAttribute>>,
    aggregate_conditions: &mut HashMap<String, String>,
) {
    let section_name = section_field_name.to_string();

    let mut nested_fields = Vec::new();

    if let Fields::Named(fields) = &nested_struct.fields {
        for field in &fields.named {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;

            nested_fields.push(quote! {
                pub #field_name: #field_type
            });

            for attr in &field.attrs {
                if let Ok(Some(map_attrs)) =
                    parse::parse_map_attribute(attr, &field_name.to_string())
                {
                    for mut map_attr in map_attrs {
                        if !map_attr.target_field_name.contains('.') {
                            map_attr.target_field_name =
                                format!("{}.{}", section_name, map_attr.target_field_name);
                        }

                        super::entity::process_map_attribute(
                            &map_attr,
                            field_name,
                            field_type,
                            &mut Vec::new(),
                            accessor_defs,
                            accessor_names,
                            primary_keys,
                            lookup_indexes,
                            sources_by_type,
                            field_mappings,
                        );
                    }
                } else if let Ok(Some(map_attrs)) =
                    parse::parse_map_instruction_attribute(attr, &field_name.to_string())
                {
                    for mut map_attr in map_attrs {
                        if !map_attr.target_field_name.contains('.') {
                            map_attr.target_field_name =
                                format!("{}.{}", section_name, map_attr.target_field_name);
                        }

                        super::entity::process_map_attribute(
                            &map_attr,
                            field_name,
                            field_type,
                            &mut Vec::new(),
                            accessor_defs,
                            accessor_names,
                            primary_keys,
                            lookup_indexes,
                            sources_by_type,
                            field_mappings,
                        );
                    }
                } else if let Ok(Some(mut event_attr)) =
                    parse::parse_event_attribute(attr, &field_name.to_string())
                {
                    *has_events = true;

                    if !event_attr.target_field_name.contains('.') {
                        event_attr.target_field_name =
                            format!("{}.{}", section_name, event_attr.target_field_name);
                    }

                    // Determine instruction path (type-safe or legacy)
                    if let Some((_instruction_path, instruction_str)) =
                        determine_event_instruction(&mut event_attr, field_type)
                    {
                        events_by_instruction
                            .entry(instruction_str)
                            .or_insert_with(Vec::new)
                            .push((
                                event_attr.target_field_name.clone(),
                                event_attr,
                                field_type.clone(),
                            ));
                    } else {
                        // Fallback to legacy instruction string
                        events_by_instruction
                            .entry(event_attr.instruction.clone())
                            .or_insert_with(Vec::new)
                            .push((
                                event_attr.target_field_name.clone(),
                                event_attr,
                                field_type.clone(),
                            ));
                    }
                } else if let Ok(Some(mut capture_attr)) =
                    parse::parse_capture_attribute(attr, &field_name.to_string())
                {
                    // Add section prefix if needed
                    if !capture_attr.target_field_name.contains('.') {
                        capture_attr.target_field_name =
                            format!("{}.{}", section_name, capture_attr.target_field_name);
                    }

                    // Infer account type from field type if not explicitly specified
                    let account_path = if let Some(ref path) = capture_attr.from_account {
                        Some(path.clone())
                    } else if let Some(inferred_path) = extract_account_type_from_field(field_type) {
                        capture_attr.inferred_account = Some(inferred_path.clone());
                        Some(inferred_path)
                    } else {
                        None
                    };

                    if let Some(acct_path) = account_path {
                        let source_type_str = path_to_string(&acct_path);

                        // Check if we have field transforms - encode them in source_field_name
                        let source_field_marker = if !capture_attr.field_transforms.is_empty() {
                            format!(
                                "__capture_with_transforms:{}",
                                capture_attr
                                    .field_transforms
                                    .iter()
                                    .map(|(k, v)| format!("{}={}", k, v))
                                    .collect::<Vec<_>>()
                                    .join(",")
                            )
                        } else {
                            String::new()
                        };

                        let map_attr = parse::MapAttribute {
                            source_type_path: acct_path,
                            source_field_name: source_field_marker,
                            target_field_name: capture_attr.target_field_name.clone(),
                            is_primary_key: false,
                            is_lookup_index: false,
                            temporal_field: None,
                            strategy: capture_attr.strategy.clone(),
                            join_on: capture_attr.join_on.as_ref().map(|fs| fs.ident.to_string()),
                            transform: None,
                            is_instruction: false,
                            is_whole_source: true, // Mark as whole source capture
                            lookup_by: capture_attr.lookup_by.clone(),
                        };

                        sources_by_type
                            .entry(source_type_str)
                            .or_insert_with(Vec::new)
                            .push(map_attr);
                    }
                } else if let Ok(Some(mut aggr_attr)) =
                    parse::parse_aggregate_attribute(attr, &field_name.to_string())
                {
                    // Add section prefix if needed
                    if !aggr_attr.target_field_name.contains('.') {
                        aggr_attr.target_field_name =
                            format!("{}.{}", section_name, aggr_attr.target_field_name);
                    }

                    // Store condition for later AST generation (with section prefix)
                    if let Some(condition) = &aggr_attr.condition {
                        aggregate_conditions
                            .insert(aggr_attr.target_field_name.clone(), condition.clone());
                    }

                    // Convert aggregate to map attributes for each instruction
                    for instr_path in &aggr_attr.from_instructions {
                        let source_field_name = aggr_attr
                            .field
                            .as_ref()
                            .map(|fs| fs.ident.to_string())
                            .unwrap_or_default();

                        let map_attr = parse::MapAttribute {
                            source_type_path: instr_path.clone(),
                            source_field_name,
                            target_field_name: aggr_attr.target_field_name.clone(),
                            is_primary_key: false,
                            is_lookup_index: false,
                            temporal_field: None,
                            strategy: aggr_attr.strategy.clone(),
                            join_on: aggr_attr.join_on.as_ref().map(|fs| fs.ident.to_string()),
                            transform: aggr_attr.transform.as_ref().map(|t| t.to_string()),
                            is_instruction: true,
                            is_whole_source: false,
                            lookup_by: aggr_attr.lookup_by.clone(),
                        };

                        // Add to sources_by_type for handler generation
                        let source_type_str = path_to_string(instr_path);
                        sources_by_type
                            .entry(source_type_str)
                            .or_insert_with(Vec::new)
                            .push(map_attr);
                    }
                } else if let Ok(Some(mut track_attr)) =
                    parse::parse_track_from_attribute(attr, &field_name.to_string())
                {
                    // Add section prefix if needed
                    if !track_attr.target_field_name.contains('.') {
                        track_attr.target_field_name =
                            format!("{}.{}", section_name, track_attr.target_field_name);
                    }

                    // Group by instruction for handler merging
                    for instr_path in &track_attr.from_instructions {
                        let source_type_str = path_to_string(instr_path);
                        track_from_mappings
                            .entry(source_type_str)
                            .or_insert_with(Vec::new)
                            .push(track_attr.clone());
                    }
                } else if let Ok(Some(mut computed_attr)) =
                    parse::parse_computed_attribute(attr, &field_name.to_string())
                {
                    // Add section prefix if needed
                    if !computed_attr.target_field_name.contains('.') {
                        computed_attr.target_field_name =
                            format!("{}.{}", section_name, computed_attr.target_field_name);
                    }

                    // Store computed field for later processing (after aggregations)
                    computed_fields.push((
                        computed_attr.target_field_name.clone(),
                        computed_attr.expression.clone(),
                        field_type.clone(),
                    ));
                }
            }
        }
    }

    state_fields.push(quote! {
        pub #section_field_name: #section_field_type
    });
}
