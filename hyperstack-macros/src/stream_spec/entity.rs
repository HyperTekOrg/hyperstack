//! Entity struct processing for stream specs.
//!
//! This module contains the main entity processing logic, including:
//! - `process_entity_struct` - Process an entity with default options
//! - `process_entity_struct_with_idl` - Process an entity with IDL context
//! - Handler generation using shared `codegen::generate_handlers_from_specs`
//!
//! ## Architecture
//!
//! Both `#[stream_spec]` and `#[ast_spec]` now use the same code generation path:
//! 1. Parse macro attributes into data structures
//! 2. Build `SerializableStreamSpec` via `ast_writer::build_and_write_ast`
//! 3. Generate handler code via `codegen::generate_handlers_from_specs`
//! 4. Write AST to disk for cloud compilation via `#[ast_spec]`

use std::collections::{HashMap, HashSet};

use quote::{format_ident, quote};
use syn::{Fields, ItemStruct, Type};

use crate::ast::{EntitySection, FieldTypeInfo};
use crate::codegen;
use crate::parse;
use crate::parse::idl as idl_parser;
use crate::utils::{path_to_string, to_pascal_case, to_snake_case};

use super::ast_writer::build_and_write_ast;
use super::computed::{
    extract_field_references_from_section, extract_section_references, parse_computed_expression,
    qualify_field_refs,
};
use super::handlers::{
    convert_event_to_map_attributes, determine_event_instruction, extract_account_type_from_field,
    generate_pda_registration_functions, generate_resolver_functions,
};
use super::sections::{
    self as sections, extract_section_from_struct_with_idl, is_primitive_or_wrapper,
    process_nested_struct,
};

// ============================================================================
// Entity Processing
// ============================================================================

/// Process an entity struct without IDL context.
pub fn process_entity_struct(
    input: ItemStruct,
    entity_name: String,
    section_structs: HashMap<String, ItemStruct>,
    skip_game_event: bool,
) -> proc_macro::TokenStream {
    process_entity_struct_with_idl(
        input,
        entity_name,
        section_structs,
        skip_game_event,
        None,
        Vec::new(), // No resolver_hooks in proto path
        Vec::new(), // No pda_registrations in proto path
    )
}

/// Process an entity struct with optional IDL context.
///
/// This is the main entry point for processing entity definitions. It:
/// 1. Extracts all field mappings from attributes
/// 2. Generates handlers for each source type
/// 3. Writes the AST file at compile time
/// 4. Generates the spec function and state struct
pub fn process_entity_struct_with_idl(
    input: ItemStruct,
    entity_name: String,
    section_structs: HashMap<String, ItemStruct>,
    skip_game_event: bool,
    idl: Option<&idl_parser::IdlSpec>,
    resolver_hooks: Vec<parse::ResolveKeyAttribute>,
    pda_registrations: Vec<parse::RegisterPdaAttribute>,
) -> proc_macro::TokenStream {
    let name = syn::Ident::new(&entity_name, input.ident.span());
    let state_name = syn::Ident::new(&format!("{}State", entity_name), input.ident.span());
    let spec_fn_name = format_ident!("create_{}_spec", to_snake_case(&entity_name));

    // We'll collect data for compile-time AST writing

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
    let mut computed_fields: Vec<(String, proc_macro2::TokenStream, syn::Type)> = Vec::new();

    // Level 1: Declarative hook macros passed from caller
    // resolver_hooks and pda_registrations are now passed as parameters
    let mut derive_from_mappings: HashMap<String, Vec<parse::DeriveFromAttribute>> = HashMap::new();
    let mut aggregate_conditions: HashMap<String, String> = HashMap::new();

    // Collect ALL section names from the entity struct FIRST
    // This is needed to properly detect cross-section references in #[computed] expressions
    let mut all_section_names: HashSet<String> = HashSet::new();
    if let Fields::Named(entity_fields) = &input.fields {
        for field in &entity_fields.named {
            let field_name = field.ident.as_ref().unwrap().to_string();
            let field_type = &field.ty;

            // Check if this field is a section type (non-primitive, non-wrapper)
            if !is_primitive_or_wrapper(field_type) {
                // This field represents a section - add its name
                all_section_names.insert(field_name);
            }
        }
    }

    // Task 3: Collect section definitions for AST
    let mut section_specs: Vec<EntitySection> = Vec::new();
    let mut root_fields: Vec<FieldTypeInfo> = Vec::new();

    if let Fields::Named(entity_fields) = &input.fields {
        for field in &entity_fields.named {
            let field_name = field.ident.as_ref().unwrap().to_string();
            let field_type = &field.ty;
            let rust_type_name = quote::quote!(#field_type).to_string();

            // Check if this field references a section struct
            if !is_primitive_or_wrapper(field_type) {
                if let Type::Path(type_path) = field_type {
                    if let Some(type_ident) = type_path.path.segments.last() {
                        let type_name = type_ident.ident.to_string();
                        // Look up the section struct definition
                        if let Some(section_struct) = section_structs.get(&type_name) {
                            let section = extract_section_from_struct_with_idl(
                                &field_name,
                                section_struct,
                                None, // Top-level section, no parent
                                idl,
                            );
                            section_specs.push(section);
                        } else {
                            // It's a non-section field (like capture fields) - add to root
                            let field_type_info = sections::analyze_field_type_with_idl(
                                &field_name,
                                &rust_type_name,
                                idl,
                            );
                            root_fields.push(field_type_info);
                        }
                    }
                }
            } else {
                // Even if it's a "wrapper", we might want its type info if it's not truly primitive
                // For example, Option<ComplexType> should be included
                let field_type_info =
                    sections::analyze_field_type_with_idl(&field_name, &rust_type_name, idl);
                // Only add if it has a resolved_type (meaning it's a complex type from IDL)
                if field_type_info.resolved_type.is_some()
                    || field_type_info.base_type == crate::ast::BaseType::Object
                {
                    root_fields.push(field_type_info);
                }
            }
        }
    }

    // Add root fields as a pseudo-section if there are any
    if !root_fields.is_empty() {
        section_specs.push(EntitySection {
            name: "root".to_string(),
            fields: root_fields,
            is_nested_struct: false,
            parent_field: None,
        });
    }

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
                } else if let Ok(Some(mut snapshot_attr)) =
                    parse::parse_snapshot_attribute(attr, &field_name.to_string())
                {
                    has_attrs = true;

                    state_fields.push(quote! {
                        pub #field_name: #field_type
                    });

                    // Infer account type from field type if not explicitly specified
                    let account_path = if let Some(ref path) = snapshot_attr.from_account {
                        Some(path.clone())
                    } else if let Some(inferred_path) = extract_account_type_from_field(field_type)
                    {
                        snapshot_attr.inferred_account = Some(inferred_path.clone());
                        Some(inferred_path)
                    } else {
                        None
                    };

                    if let Some(acct_path) = account_path {
                        let source_type_str = path_to_string(&acct_path);

                        // Check if we have field transforms - encode them in source_field_name
                        // so we can detect and process them differently during code generation
                        let source_field_marker = if !snapshot_attr.field_transforms.is_empty() {
                            format!(
                                "__snapshot_with_transforms:{}",
                                snapshot_attr
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
                            target_field_name: snapshot_attr.target_field_name.clone(),
                            is_primary_key: false,
                            is_lookup_index: false,
                            temporal_field: None,
                            strategy: snapshot_attr.strategy.clone(),
                            join_on: snapshot_attr
                                .join_on
                                .as_ref()
                                .map(|fs| fs.ident.to_string()),
                            transform: None,
                            is_instruction: false,
                            is_whole_source: true, // Mark as whole source snapshot
                            lookup_by: snapshot_attr.lookup_by.clone(),
                        };

                        sources_by_type
                            .entry(source_type_str)
                            .or_default()
                            .push(map_attr);
                    }
                } else if let Ok(Some(aggr_attr)) =
                    parse::parse_aggregate_attribute(attr, &field_name.to_string())
                {
                    has_attrs = true;

                    state_fields.push(quote! {
                        pub #field_name: #field_type
                    });

                    // Level 1: Store condition for later AST generation
                    if let Some(condition) = &aggr_attr.condition {
                        let field_path = format!("{}.{}", entity_name, field_name);
                        aggregate_conditions.insert(field_path, condition.clone());
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
                            .or_default()
                            .push(map_attr);
                    }
                } else if let Ok(Some(derive_attr)) =
                    parse::parse_derive_from_attribute(attr, &field_name.to_string())
                {
                    // Level 1: Process #[derive_from] attribute
                    // This code successfully parses derive_from attributes and adds them to derive_from_mappings.
                    // The AST writer then processes these mappings to populate instruction_hooks in the AST.
                    has_attrs = true;
                    state_fields.push(quote! { pub #field_name: #field_type });

                    // Group by instruction for handler merging
                    for instr_path in &derive_attr.from_instructions {
                        let source_type_str = path_to_string(instr_path);
                        derive_from_mappings
                            .entry(source_type_str)
                            .or_default()
                            .push(derive_attr.clone());
                    }
                } else if let Ok(Some(computed_attr)) =
                    parse::parse_computed_attribute(attr, &field_name.to_string())
                {
                    has_attrs = true;

                    state_fields.push(quote! {
                        pub #field_name: #field_type
                    });

                    // Store computed field for later processing (after aggregations)
                    computed_fields.push((
                        computed_attr.target_field_name.clone(),
                        computed_attr.expression.clone(),
                        field_type.clone(),
                    ));
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

    // === EVENT MERGING: Merge event mappings into sources_by_type ===
    // Convert events to map attributes and merge with sources_by_type
    // Events with lookup_by are kept separate for special handling in AST building

    for event_mappings in events_by_instruction.values() {
        for (target_field, event_attr, _field_type) in event_mappings {
            // Skip events with lookup_by - they need separate handler generation
            // (They stay in events_by_instruction for AST building)
            if event_attr.lookup_by.is_some() {
                continue;
            }

            // Get instruction path from event attribute
            let instruction_path = event_attr
                .from_instruction
                .as_ref()
                .or(event_attr.inferred_instruction.as_ref());

            if let Some(instr_path) = instruction_path {
                // Convert instruction path to string for sources_by_type key
                let source_type_str = path_to_string(instr_path);

                // Convert event to map attributes
                let map_attrs =
                    convert_event_to_map_attributes(target_field, event_attr, instr_path, idl);

                // Merge into sources_by_type
                sources_by_type
                    .entry(source_type_str)
                    .or_default()
                    .extend(map_attrs);
            }
        }
    }

    let mut views = parse::parse_view_attributes(&input.attrs);
    for view in &mut views {
        if let crate::ast::ViewSource::Entity { name } = &mut view.source {
            *name = entity_name.clone();
        }
        if !view.id.contains('/') {
            view.id = format!("{}/{}", entity_name, view.id);
        }
    }

    let ast = build_and_write_ast(
        &entity_name,
        &primary_keys,
        &lookup_indexes,
        &sources_by_type,
        &events_by_instruction,
        &resolver_hooks,
        &pda_registrations,
        &derive_from_mappings,
        &aggregate_conditions,
        &computed_fields,
        &section_specs,
        idl,
        views,
    );

    // Generate handler functions using the shared codegen
    let (handler_fns, _handler_calls) =
        codegen::generate_handlers_from_specs(&ast.handlers, &entity_name, &state_name);

    let game_event_struct = if has_events && !skip_game_event {
        quote! {
            #[derive(Debug, Clone, hyperstack::runtime::serde::Serialize, hyperstack::runtime::serde::Deserialize)]
            pub struct GameEvent {
                pub timestamp: i64,
                #[serde(flatten)]
                pub data: hyperstack::runtime::serde_json::Value,
            }
        }
    } else {
        quote! {}
    };

    let _lookup_index_creations: Vec<_> = lookup_indexes
        .iter()
        .map(|(field_name, temporal_field)| {
            if let Some(tf) = temporal_field {
                quote! {
                    hyperstack::runtime::hyperstack_interpreter::ast::LookupIndexSpec {
                        field_name: #field_name.to_string(),
                        temporal_field: Some(#tf.to_string()),
                    }
                }
            } else {
                quote! {
                    hyperstack::runtime::hyperstack_interpreter::ast::LookupIndexSpec {
                        field_name: #field_name.to_string(),
                        temporal_field: None,
                    }
                }
            }
        })
        .collect();

    // Generate computed fields evaluation function if there are any computed fields
    // This function will be called after aggregations complete to evaluate derived fields
    let computed_fields_hook = if !computed_fields.is_empty() {
        generate_computed_fields_hook(&computed_fields, &all_section_names)
    } else {
        // Generate a no-op function even when there are no computed fields
        // so the evaluator callback can still reference it
        quote! {
            /// No-op evaluate_computed_fields (no computed fields defined)
            pub fn evaluate_computed_fields(
                _state: &mut hyperstack::runtime::serde_json::Value
            ) -> Result<(), Box<dyn std::error::Error>> {
                Ok(())
            }

            /// Returns the list of computed field paths (section.field format)
            pub fn computed_field_paths() -> &'static [&'static str] {
                &[]
            }
        }
    };

    // Level 1: Generate functions from declarative PDA macros
    let resolver_fns = generate_resolver_functions(&resolver_hooks, idl);
    let pda_registration_fns = generate_pda_registration_functions(&pda_registrations);

    // Generate field accessors for type-safe view definitions
    let field_accessors = codegen::generate_field_accessors(&section_specs);

    let module_name = format_ident!("{}", to_snake_case(&entity_name));

    let output = quote! {
        #[derive(Debug, Clone, hyperstack::runtime::serde::Serialize, hyperstack::runtime::serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #state_name {
            #(#state_fields),*
        }

        #game_event_struct

        pub mod #module_name {
            use super::*;

            #(#accessor_defs)*
        }

        #field_accessors

        pub fn #spec_fn_name() -> hyperstack::runtime::hyperstack_interpreter::ast::TypedStreamSpec<#state_name> {
            // Load AST file at compile time (includes instruction_hooks!)
            let ast_json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.hyperstack/", stringify!(#name), ".ast.json"));

            // Deserialize the AST
            let serializable_spec: hyperstack::runtime::hyperstack_interpreter::ast::SerializableStreamSpec = hyperstack::runtime::serde_json::from_str(ast_json)
                .expect(&format!("Failed to parse AST file for {}", stringify!(#name)));

            // Convert to typed spec (this preserves instruction_hooks and all other fields!)
            hyperstack::runtime::hyperstack_interpreter::ast::TypedStreamSpec::from_serializable(serializable_spec)
        }

        // Generated from declarative PDA macros
        #resolver_fns
        #pda_registration_fns

        #(#handler_fns)*

        #computed_fields_hook
    };

    output.into()
}

// ============================================================================
// Map Attribute Processing
// ============================================================================

/// Process a map attribute and update all the relevant collections.
#[allow(clippy::too_many_arguments)]
pub fn process_map_attribute(
    map_attr: &parse::MapAttribute,
    field_name: &syn::Ident,
    field_type: &Type,
    state_fields: &mut Vec<proc_macro2::TokenStream>,
    accessor_defs: &mut Vec<proc_macro2::TokenStream>,
    accessor_names: &mut HashSet<String>,
    primary_keys: &mut Vec<String>,
    lookup_indexes: &mut Vec<(String, Option<String>)>,
    sources_by_type: &mut HashMap<String, Vec<parse::MapAttribute>>,
    field_mappings: &mut Vec<parse::MapAttribute>,
) {
    let target_field = &map_attr.target_field_name;

    state_fields.push(quote! {
        pub #field_name: #field_type
    });

    let accessor_name = to_pascal_case(&field_name.to_string());
    let accessor_ident = format_ident!("{}", accessor_name);

    if accessor_names.insert(accessor_name.clone()) {
        accessor_defs.push(quote! {
            pub struct #accessor_ident;

            impl #accessor_ident {
                pub fn path(&self) -> String {
                    #target_field.to_string()
                }
            }
        });
    }

    if map_attr.is_primary_key {
        primary_keys.push(target_field.clone());
    }

    if map_attr.is_lookup_index {
        lookup_indexes.push((target_field.clone(), map_attr.temporal_field.clone()));
    }

    let source_type = path_to_string(&map_attr.source_type_path);
    sources_by_type
        .entry(source_type.clone())
        .or_default()
        .push(map_attr.clone());

    field_mappings.push(map_attr.clone());
}

// ============================================================================
// Computed Fields Hook Generation
// ============================================================================

/// Generate the computed fields evaluation hook.
fn generate_computed_fields_hook(
    computed_fields: &[(String, proc_macro2::TokenStream, Type)],
    all_section_names: &HashSet<String>,
) -> proc_macro2::TokenStream {
    // Group computed fields by section
    let mut fields_by_section: HashMap<String, Vec<(String, proc_macro2::TokenStream, Type)>> =
        HashMap::new();

    for (target_field, expression, field_type) in computed_fields {
        // Extract section name from target_field (format: "section.field")
        let parts: Vec<&str> = target_field.split('.').collect();
        if parts.len() == 2 {
            let section = parts[0].to_string();
            let field = parts[1].to_string();
            fields_by_section.entry(section.clone()).or_default().push((
                field.clone(),
                expression.clone(),
                field_type.clone(),
            ));
        }
    }

    // Analyze expressions to determine which sections they reference
    // Use the all_section_names collected earlier to properly identify cross-section references
    let mut section_dependencies: HashMap<String, HashSet<String>> = HashMap::new();
    for (section, fields) in &fields_by_section {
        let mut deps = HashSet::new();
        for (_field_name, expression, _field_type) in fields {
            let referenced_sections = extract_section_references(expression);

            // Filter to only include references that are:
            // 1. NOT the current section (same-section references don't need special handling)
            // 2. ACTUALLY known section names (not method names like unwrap_or, max, etc.)
            let valid_refs: HashSet<String> = referenced_sections
                .into_iter()
                .filter(|ref_name| {
                    let is_known = all_section_names.contains(ref_name);
                    let is_not_current = ref_name != section;
                    is_known && is_not_current
                })
                .collect();

            deps.extend(valid_refs);
        }
        section_dependencies.insert(section.clone(), deps);
    }

    // First, collect ALL struct definitions that we need (from all sections' dependencies)
    let mut all_struct_defs: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut generated_structs: HashSet<String> = HashSet::new();

    for (section, fields) in &fields_by_section {
        let deps = section_dependencies
            .get(section)
            .cloned()
            .unwrap_or_default();

        for dep_section in &deps {
            // Only generate struct once per dependency section
            if generated_structs.contains(dep_section) {
                continue;
            }
            generated_structs.insert(dep_section.clone());

            let section_struct_ident = format_ident!("{}Section", dep_section);

            // Collect all fields accessed from this section across ALL expressions
            let mut section_fields = HashSet::new();
            for (_field_name, expression, _field_type) in fields {
                let field_refs = extract_field_references_from_section(expression, dep_section);
                section_fields.extend(field_refs);
            }

            // Generate struct definition for this section
            let field_defs: Vec<_> = section_fields
                .iter()
                .map(|field| {
                    let field_ident = format_ident!("{}", field);
                    quote! {
                        pub #field_ident: Option<u64>
                    }
                })
                .collect();

            // Generate field extraction in constructor
            let field_extractors: Vec<_> = section_fields
                .iter()
                .map(|field| {
                    let field_ident = format_ident!("{}", field);
                    let field_str = field.as_str();
                    quote! {
                        #field_ident: obj.get(#field_str)
                            .and_then(|v| hyperstack::runtime::serde_json::from_value(v.clone()).ok())
                    }
                })
                .collect();

            all_struct_defs.push(quote! {
                // Helper struct for cross-section field access
                #[allow(dead_code)]
                struct #section_struct_ident {
                    #(#field_defs),*
                }

                impl #section_struct_ident {
                    fn from_object(obj: &hyperstack::runtime::serde_json::Map<String, hyperstack::runtime::serde_json::Value>) -> Self {
                        Self {
                            #(#field_extractors),*
                        }
                    }
                }
            });
        }
    }

    // Now generate the evaluation functions
    let eval_functions: Vec<_> = fields_by_section.iter().map(|(section, fields)| {
        let _section_ident = format_ident!("{}", section);
        let eval_fn_name = format_ident!("evaluate_computed_fields_{}", section);

        // Get dependencies for this section
        let deps = section_dependencies.get(section).cloned().unwrap_or_default();

        let _section_str = section.as_str();
        let field_evaluations: Vec<_> = fields.iter().map(|(field_name, expression, field_type)| {
            let field_str = field_name.as_str();
            let field_ident = format_ident!("{}", field_name);

            // Parse the expression into an AST and generate evaluation code
            // This transforms expressions like `round_snapshot.slot_hash.to_bytes()` into
            // proper JSON state access code
            let parsed_expr = parse_computed_expression(expression);
            // Qualify the expression with the section prefix for unqualified field refs
            let qualified_expr = qualify_field_refs(parsed_expr, section);
            let expr_code = crate::codegen::generate_computed_expr_code(&qualified_expr);

            quote! {
                // Evaluate: #field_name
                let computed_value = {
                    // state is the full entity JSON state
                    let state = &section_parent_state;
                    #expr_code
                };
                let serialized_value = hyperstack::runtime::serde_json::to_value(&computed_value)?;
                section_obj.insert(#field_str.to_string(), serialized_value);

                let #field_ident: #field_type = section_obj
                    .get(#field_str)
                    .and_then(|v| hyperstack::runtime::serde_json::from_value(v.clone()).ok());
            }
        }).collect();

        // Generate function parameters for cross-section dependencies
        let cross_section_params: Vec<_> = deps.iter().map(|dep_section| {
            let dep_section_ident = format_ident!("{}", dep_section);
            let section_struct_ident = format_ident!("{}Section", dep_section);
            quote! {
                #dep_section_ident: &#section_struct_ident
            }
        }).collect();

        quote! {
            fn #eval_fn_name(
                section_obj: &mut hyperstack::runtime::serde_json::Map<String, hyperstack::runtime::serde_json::Value>,
                section_parent_state: &hyperstack::runtime::serde_json::Value,
                #(#cross_section_params),*
            ) -> Result<(), Box<dyn std::error::Error>> {
                // Create local bindings for all fields in the current section
                // Helper macro to get field values with proper type inference
                macro_rules! extract_field {
                    ($name:ident, $ty:ty) => {
                        let $name: Option<$ty> = section_obj
                            .get(stringify!($name))
                            .and_then(|v| hyperstack::runtime::serde_json::from_value(v.clone()).ok());
                    };
                }

                // Extract all numeric/common fields that might be referenced
                extract_field!(total_buy_volume, u64);
                extract_field!(total_sell_volume, u64);
                extract_field!(total_trades, u64);
                extract_field!(total_volume, u64);
                extract_field!(buy_count, u64);
                extract_field!(sell_count, u64);
                extract_field!(unique_traders, u64);
                extract_field!(largest_trade, u64);
                extract_field!(smallest_trade, u64);
                extract_field!(last_trade_timestamp, i64);
                extract_field!(last_trade_price, f64);
                extract_field!(whale_trade_count, u64);
                extract_field!(average_trade_size, f64);

                // Evaluate computed fields
                #(#field_evaluations)*

                Ok(())
            }
        }
    }).collect();

    // Generate the main hook function that applies to state after handlers execute
    // We need to extract cross-section data BEFORE getting mutable borrow of target section
    // to avoid borrow checker issues
    let eval_calls: Vec<_> = fields_by_section.keys().map(|section| {
        let section_str = section.as_str();
        let eval_fn_name = format_ident!("evaluate_computed_fields_{}", section);
        let deps = section_dependencies.get(section).cloned().unwrap_or_default();

        // Generate pre-extraction of cross-section data
        // These return Option so we can gracefully skip evaluation if a dependency doesn't exist yet
        let dep_extractions: Vec<_> = deps.iter().map(|dep_section| {
            let dep_section_ident = format_ident!("{}", dep_section);
            let dep_section_str = dep_section.as_str();
            let section_struct_ident = format_ident!("{}Section", dep_section);
            quote! {
                let #dep_section_ident = state
                    .get(#dep_section_str)
                    .and_then(|v| v.as_object())
                    .map(|obj| #section_struct_ident::from_object(obj));
            }
        }).collect();

        if dep_extractions.is_empty() {
            quote! {
                let state_snapshot = state.clone();

                if let Some(section_value) = state.get_mut(#section_str) {
                    if let Some(section_obj) = section_value.as_object_mut() {
                        #eval_fn_name(section_obj, &state_snapshot)?;
                    }
                }
            }
        } else {
            // Has cross-section dependencies - extract first, then compute
            // If ANY dependency is missing, skip evaluation (the computed fields will remain None)
            let dep_param_names: Vec<_> = deps.iter().map(|dep| format_ident!("{}", dep)).collect();
            let dep_checks: Vec<_> = deps.iter().map(|dep| {
                let dep_ident = format_ident!("{}", dep);
                quote! { #dep_ident.is_some() }
            }).collect();
            let dep_unwraps: Vec<_> = deps.iter().map(|dep| {
                let dep_ident = format_ident!("{}", dep);
                quote! { let #dep_ident = #dep_ident.unwrap(); }
            }).collect();
            quote! {
                // Clone the state for immutable reference during computation
                let state_snapshot = state.clone();

                // Extract cross-section data first (immutable borrow)
                #(#dep_extractions)*

                // Only evaluate if ALL cross-section dependencies exist
                // If any dependency is missing, skip evaluation (computed fields stay None)
                if #(#dep_checks)&&* {
                    #(#dep_unwraps)*

                    // Now get mutable borrow of target section and compute
                    if let Some(section_value) = state.get_mut(#section_str) {
                        if let Some(section_obj) = section_value.as_object_mut() {
                            #eval_fn_name(section_obj, &state_snapshot, #(&#dep_param_names),*)?;
                        }
                    }
                } else {
                    hyperstack::runtime::tracing::trace!("Skipping computed fields for section '{}' (dependencies not available)", #section_str);
                }
            }
        }
    }).collect();

    // Generate the list of computed field paths as string literals
    let computed_field_path_strs: Vec<&str> = computed_fields
        .iter()
        .map(|(path, _, _)| path.as_str())
        .collect();

    quote! {
        // Helper structs for cross-section field access (generated by macro)
        #(#all_struct_defs)*

        // Computed field evaluation functions (generated by macro)
        #(#eval_functions)*

        /// Evaluate all computed fields for the entity state
        /// This should be called after aggregations complete but before hooks run
        pub fn evaluate_computed_fields(
            state: &mut hyperstack::runtime::serde_json::Value
        ) -> Result<(), Box<dyn std::error::Error>> {
            #(#eval_calls)*
            Ok(())
        }

        /// Returns the list of computed field paths (section.field format)
        pub fn computed_field_paths() -> &'static [&'static str] {
            &[#(#computed_field_path_strs),*]
        }
    }
}
