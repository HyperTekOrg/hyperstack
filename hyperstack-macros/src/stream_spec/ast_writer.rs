//! AST building and file writing for hyperstack streams.
//!
//! This module handles:
//! 1. Building `SerializableStreamSpec` from parsed macro attributes
//! 2. Writing AST JSON files during macro expansion for cloud compilation
//!
//! The same AST is used for both inline code generation (via `codegen::generate_handlers_from_specs`)
//! and for the `#[ast_spec]` macro, ensuring identical output.

use std::collections::{BTreeMap, HashMap};

use crate::ast::writer::{
    convert_idl_to_snapshot, parse_population_strategy, parse_transformation,
};
use crate::ast::{
    ComputedFieldSpec, ConditionExpr, EntitySection, FieldPath, HookAction, IdentitySpec,
    InstructionHook, KeyResolutionStrategy, LookupIndexSpec, MappingSource, ResolverHook,
    ResolverStrategy, SerializableFieldMapping, SerializableHandlerSpec, SerializableStreamSpec,
    SourceSpec,
};
use crate::parse;
use crate::parse::conditions as condition_parser;
use crate::parse::idl as idl_parser;
use crate::utils::path_to_string;

use super::computed::{parse_computed_expression, qualify_field_refs};
use super::handlers::{find_field_in_instruction, get_join_on_field};

// ============================================================================
// AST Building
// ============================================================================

/// Build the complete AST from parsed macro attributes.
///
/// This is the single source of truth for building `SerializableStreamSpec`.
/// The returned AST is used for:
/// 1. Code generation via `codegen::generate_handlers_from_specs()`
/// 2. Writing to disk for `#[ast_spec]` cloud compilation
///
/// # Arguments
///
/// * `entity_name` - The name of the entity
/// * `primary_keys` - List of primary key field names
/// * `lookup_indexes` - List of lookup index definitions
/// * `sources_by_type` - Map of source type to field mappings
/// * `events_by_instruction` - Map of instruction to event mappings
/// * `resolver_hooks` - Resolver hook definitions
/// * `pda_registrations` - PDA registration definitions
/// * `derive_from_mappings` - Derive-from field mappings
/// * `aggregate_conditions` - Conditional aggregate definitions
/// * `computed_fields` - Computed field definitions
/// * `section_specs` - Entity section specifications
/// * `idl` - Optional IDL specification for field resolution
/// * `views` - View definitions for derived views
#[allow(clippy::too_many_arguments)]
pub fn build_ast(
    entity_name: &str,
    primary_keys: &[String],
    lookup_indexes: &[(String, Option<String>)],
    sources_by_type: &HashMap<String, Vec<parse::MapAttribute>>,
    events_by_instruction: &HashMap<String, Vec<(String, parse::EventAttribute, syn::Type)>>,
    resolver_hooks: &[parse::ResolveKeyAttribute],
    pda_registrations: &[parse::RegisterPdaAttribute],
    derive_from_mappings: &HashMap<String, Vec<parse::DeriveFromAttribute>>,
    aggregate_conditions: &HashMap<String, String>,
    computed_fields: &[(String, proc_macro2::TokenStream, syn::Type)],
    section_specs: &[EntitySection],
    idl: Option<&idl_parser::IdlSpec>,
    views: Vec<crate::ast::ViewDef>,
) -> SerializableStreamSpec {
    // Build handlers from sources and events
    let handlers = build_handlers(
        sources_by_type,
        events_by_instruction,
        primary_keys,
        lookup_indexes,
        aggregate_conditions,
        idl,
    );

    let resolver_hooks_ast = build_resolver_hooks_ast(resolver_hooks, idl);
    let instruction_hooks_ast = build_instruction_hooks_ast(
        pda_registrations,
        derive_from_mappings,
        aggregate_conditions,
        sources_by_type,
    );

    // Keep field paths in snake_case to match IDL - SDK generators handle case conversion
    let computed_field_paths: Vec<String> = computed_fields
        .iter()
        .map(|(path, _, _)| path.clone())
        .collect();

    // Extract program_id and convert IDL to snapshot for embedding
    let program_id = idl.and_then(|i| {
        i.address.clone().or_else(|| {
            i.metadata
                .as_ref()
                .and_then(|m| m.address.as_ref().cloned())
        })
    });
    let idl_snapshot = idl.map(convert_idl_to_snapshot);

    // Parse computed field expressions into ComputedFieldSpec
    let computed_field_specs: Vec<ComputedFieldSpec> = computed_fields
        .iter()
        .map(|(target_path, expr_tokens, field_type)| {
            let result_type = quote::quote!(#field_type).to_string();
            let expression = parse_computed_expression(expr_tokens);

            // Extract section name from target_path and qualify field references
            let section = target_path.split('.').next().unwrap_or("");
            let qualified_expression = if !section.is_empty() {
                qualify_field_refs(expression, section)
            } else {
                expression
            };

            ComputedFieldSpec {
                target_path: target_path.clone(),
                expression: qualified_expression,
                result_type,
            }
        })
        .collect();

    // Build field_mappings from sections - this provides type information for ALL fields
    let mut field_mappings = BTreeMap::new();
    for section in section_specs {
        for field_info in &section.fields {
            // Handle root-level fields (no section prefix)
            let field_path = if section.name == "root" {
                field_info.field_name.clone()
            } else {
                format!("{}.{}", section.name, field_info.field_name)
            };
            field_mappings.insert(field_path, field_info.clone());
        }
    }

    let mut spec = SerializableStreamSpec {
        state_name: entity_name.to_string(),
        program_id,
        idl: idl_snapshot,
        identity: IdentitySpec {
            primary_keys: primary_keys.to_vec(),
            lookup_indexes: lookup_indexes
                .iter()
                .map(|(field_name, temporal_field)| LookupIndexSpec {
                    field_name: field_name.clone(),
                    temporal_field: temporal_field.clone(),
                })
                .collect(),
        },
        handlers,
        sections: section_specs.to_vec(),
        field_mappings,
        resolver_hooks: resolver_hooks_ast,
        instruction_hooks: instruction_hooks_ast,
        computed_fields: computed_field_paths,
        computed_field_specs,
        content_hash: None,
        views,
    };
    // Compute and set the content hash
    spec.content_hash = Some(spec.compute_content_hash());
    spec
}

// ============================================================================
// AST Writing
// ============================================================================

/// Write AST to JSON file at compile time (during macro expansion).
///
/// This function calls `build_ast` and writes the result to disk.
/// The AST file can be used by `#[ast_spec]` for cloud compilation.
#[allow(clippy::too_many_arguments)]
pub fn write_ast_at_compile_time(
    entity_name: &str,
    primary_keys: &[String],
    lookup_indexes: &[(String, Option<String>)],
    sources_by_type: &HashMap<String, Vec<parse::MapAttribute>>,
    events_by_instruction: &HashMap<String, Vec<(String, parse::EventAttribute, syn::Type)>>,
    resolver_hooks: &[parse::ResolveKeyAttribute],
    pda_registrations: &[parse::RegisterPdaAttribute],
    derive_from_mappings: &HashMap<String, Vec<parse::DeriveFromAttribute>>,
    aggregate_conditions: &HashMap<String, String>,
    computed_fields: &[(String, proc_macro2::TokenStream, syn::Type)],
    section_specs: &[EntitySection],
    idl: Option<&idl_parser::IdlSpec>,
    views: Vec<crate::ast::ViewDef>,
) {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let ast_dir = std::path::Path::new(&manifest_dir).join(".hyperstack");

        if std::fs::create_dir_all(&ast_dir).is_err() {
            return;
        }

        let ast_file = ast_dir.join(format!("{}.ast.json", entity_name));

        let ast = build_ast(
            entity_name,
            primary_keys,
            lookup_indexes,
            sources_by_type,
            events_by_instruction,
            resolver_hooks,
            pda_registrations,
            derive_from_mappings,
            aggregate_conditions,
            computed_fields,
            section_specs,
            idl,
            views,
        );

        if let Ok(json) = serde_json::to_string_pretty(&ast) {
            let _ = std::fs::write(&ast_file, json);
            eprintln!("Generated AST: {}", ast_file.display());
        }
    }
}

/// Build AST and write to disk, returning the AST for code generation.
#[allow(clippy::too_many_arguments)]
pub fn build_and_write_ast(
    entity_name: &str,
    primary_keys: &[String],
    lookup_indexes: &[(String, Option<String>)],
    sources_by_type: &HashMap<String, Vec<parse::MapAttribute>>,
    events_by_instruction: &HashMap<String, Vec<(String, parse::EventAttribute, syn::Type)>>,
    resolver_hooks: &[parse::ResolveKeyAttribute],
    pda_registrations: &[parse::RegisterPdaAttribute],
    derive_from_mappings: &HashMap<String, Vec<parse::DeriveFromAttribute>>,
    aggregate_conditions: &HashMap<String, String>,
    computed_fields: &[(String, proc_macro2::TokenStream, syn::Type)],
    section_specs: &[EntitySection],
    idl: Option<&idl_parser::IdlSpec>,
    views: Vec<crate::ast::ViewDef>,
) -> SerializableStreamSpec {
    let ast = build_ast(
        entity_name,
        primary_keys,
        lookup_indexes,
        sources_by_type,
        events_by_instruction,
        resolver_hooks,
        pda_registrations,
        derive_from_mappings,
        aggregate_conditions,
        computed_fields,
        section_specs,
        idl,
        views,
    );

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let ast_dir = std::path::Path::new(&manifest_dir).join(".hyperstack");

        if std::fs::create_dir_all(&ast_dir).is_ok() {
            let ast_file = ast_dir.join(format!("{}.ast.json", entity_name));
            if let Ok(json) = serde_json::to_string_pretty(&ast) {
                let _ = std::fs::write(&ast_file, json);
                eprintln!("Generated AST: {}", ast_file.display());
            }
        }
    }

    ast
}

// ============================================================================
// Handler Building
// ============================================================================

fn build_handlers(
    sources_by_type: &HashMap<String, Vec<parse::MapAttribute>>,
    events_by_instruction: &HashMap<String, Vec<(String, parse::EventAttribute, syn::Type)>>,
    primary_keys: &[String],
    lookup_indexes: &[(String, Option<String>)],
    aggregate_conditions: &HashMap<String, String>,
    idl: Option<&idl_parser::IdlSpec>,
) -> Vec<SerializableHandlerSpec> {
    let mut handlers = Vec::new();

    // Group sources by type and join key
    let mut sources_by_type_and_join: BTreeMap<(String, Option<String>), Vec<parse::MapAttribute>> =
        BTreeMap::new();
    for (source_type, mappings) in sources_by_type {
        for mapping in mappings {
            let key = (source_type.clone(), mapping.join_on.clone());
            sources_by_type_and_join
                .entry(key)
                .or_default()
                .push(mapping.clone());
        }
    }

    // Process account sources
    for ((source_type, join_key), mappings) in &sources_by_type_and_join {
        if let Some(handler) = build_source_handler(
            source_type,
            join_key,
            mappings,
            aggregate_conditions,
            primary_keys,
            lookup_indexes,
            idl,
        ) {
            handlers.push(handler);
        }
    }

    // Group events by instruction and join key
    #[allow(clippy::type_complexity)]
    let mut events_by_instruction_and_join: BTreeMap<
        (String, Option<String>),
        Vec<(String, parse::EventAttribute, syn::Type)>,
    > = BTreeMap::new();
    for (instruction, event_mappings) in events_by_instruction {
        for event_mapping in event_mappings {
            let join_on_str = get_join_on_field(&event_mapping.1.join_on);
            let key = (instruction.clone(), join_on_str);
            events_by_instruction_and_join
                .entry(key)
                .or_default()
                .push(event_mapping.clone());
        }
    }

    // Process event sources
    for ((instruction, join_key), event_mappings) in &events_by_instruction_and_join {
        if let Some(handler) = build_event_handler(
            instruction,
            join_key,
            event_mappings,
            primary_keys,
            lookup_indexes,
            idl,
        ) {
            handlers.push(handler);
        }
    }

    handlers
}

fn build_source_handler(
    source_type: &str,
    join_key: &Option<String>,
    mappings: &[parse::MapAttribute],
    aggregate_conditions: &HashMap<String, String>,
    primary_keys: &[String],
    lookup_indexes: &[(String, Option<String>)],
    idl: Option<&idl_parser::IdlSpec>,
) -> Option<SerializableHandlerSpec> {
    let account_type = source_type.split("::").last().unwrap_or(source_type);
    let is_instruction = mappings.iter().any(|m| m.is_instruction);

    // Skip event-derived mappings
    if is_instruction
        && mappings
            .iter()
            .any(|m| m.target_field_name.starts_with("events."))
    {
        return None;
    }

    let mut serializable_mappings = Vec::new();
    let mut has_primary_key = false;
    let mut primary_field = None;

    for mapping in mappings {
        // Skip conditional aggregates
        if aggregate_conditions.contains_key(&mapping.target_field_name) {
            continue;
        }

        let source = if mapping.is_whole_source {
            let field_transforms = if mapping
                .source_field_name
                .starts_with("__snapshot_with_transforms:")
            {
                let transforms_str = mapping
                    .source_field_name
                    .strip_prefix("__snapshot_with_transforms:")
                    .unwrap_or("");
                transforms_str
                    .split(',')
                    .filter_map(|pair| {
                        let parts: Vec<&str> = pair.split('=').collect();
                        if parts.len() == 2 {
                            parse_transformation(parts[1]).map(|t| (parts[0].to_string(), t))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                BTreeMap::new()
            };

            MappingSource::AsCapture { field_transforms }
        } else {
            let field_path = if is_instruction {
                if mapping.source_field_name.is_empty() {
                    FieldPath::new(&["data"])
                } else {
                    let prefix = idl
                        .and_then(|idl| {
                            idl.get_instruction_field_prefix(
                                account_type,
                                &mapping.source_field_name,
                            )
                        })
                        .unwrap_or("data");
                    FieldPath::new(&[prefix, &mapping.source_field_name])
                }
            } else if mapping.source_field_name.is_empty() {
                FieldPath::new(&[])
            } else {
                FieldPath::new(&[&mapping.source_field_name])
            };

            MappingSource::FromSource {
                path: field_path,
                default: None,
                transform: mapping
                    .transform
                    .as_ref()
                    .and_then(|t| parse_transformation(t)),
            }
        };

        let population = parse_population_strategy(&mapping.strategy);

        serializable_mappings.push(SerializableFieldMapping {
            target_path: mapping.target_field_name.clone(),
            source,
            transform: None,
            population,
        });

        if mapping.is_primary_key {
            has_primary_key = true;
            if is_instruction {
                let prefix = idl
                    .and_then(|idl| {
                        idl.get_instruction_field_prefix(account_type, &mapping.source_field_name)
                    })
                    .unwrap_or("data");
                primary_field = Some(format!("{}.{}", prefix, mapping.source_field_name));
            } else {
                primary_field = Some(mapping.source_field_name.clone());
            }
        }
    }

    let is_aggregation = mappings.iter().any(|m| {
        matches!(
            m.strategy.as_str(),
            "Sum" | "Count" | "Min" | "Max" | "UniqueCount"
        )
    });

    // Try to find lookup_by from the first mapping that has it
    let lookup_by_field = mappings
        .iter()
        .find_map(|m| m.lookup_by.as_ref())
        .map(|fs| {
            // FieldSpec has explicit_location which tells us if it's accounts:: or data::
            let prefix = match &fs.explicit_location {
                Some(parse::FieldLocation::Account) => "accounts",
                Some(parse::FieldLocation::InstructionArg) => "data",
                None => "accounts", // Default to accounts for compatibility
            };
            format!("{}.{}", prefix, fs.ident)
        });

    let key_resolution = if has_primary_key {
        let primary_field_str = primary_field.as_deref().unwrap_or("");
        let segments: Vec<&str> = primary_field_str.split('.').collect();
        KeyResolutionStrategy::Embedded {
            primary_field: FieldPath::new(&segments),
        }
    } else if is_aggregation && is_instruction {
        // Use lookup_by if available, otherwise fall back to join_key or a sensible default
        if let Some(ref lookup_field) = lookup_by_field {
            // Check if lookup_by points directly to a field that matches the primary key.
            // If the lookup_by field name matches the primary key field name, it means we're
            // pointing directly to the primary key field itself (e.g., accounts.mint when
            // id.mint is the primary key), so we should use Embedded resolution.
            //
            // If it doesn't match the primary key, we need a Lookup resolution to do a
            // reverse lookup (e.g., accounts.bonding_curve -> mint via PDA lookup).
            let lookup_field_name = lookup_field.split('.').next_back().unwrap_or(lookup_field);

            // Check if any primary key field name matches the lookup_by field name
            // Primary keys are like "id.mint", so we compare the last segment
            let is_primary_key_field = primary_keys
                .iter()
                .any(|pk| pk.split('.').next_back().unwrap_or(pk) == lookup_field_name);

            if is_primary_key_field {
                // The lookup_by field IS the primary key itself - use Embedded
                let segments: Vec<&str> = lookup_field.split('.').collect();
                KeyResolutionStrategy::Embedded {
                    primary_field: FieldPath::new(&segments),
                }
            } else {
                // The lookup_by field is a PDA that needs reverse lookup
                let segments: Vec<&str> = lookup_field.split('.').collect();
                KeyResolutionStrategy::Lookup {
                    primary_field: FieldPath::new(&segments),
                }
            }
        } else if let Some(ref join_field) = join_key {
            KeyResolutionStrategy::Lookup {
                primary_field: FieldPath::new(&[join_field]),
            }
        } else {
            // No lookup_by specified - use embedded with empty path
            // The instruction handler will need the primary key from elsewhere
            KeyResolutionStrategy::Embedded {
                primary_field: FieldPath::new(&[]),
            }
        }
    } else if let Some(ref join_field) = join_key {
        KeyResolutionStrategy::Lookup {
            primary_field: FieldPath::new(&[join_field]),
        }
    } else {
        // Check if there's a lookup index configured for this account type
        // If so, use Lookup strategy with __account_address to enable PDA reverse lookup
        let account_has_lookup_index = lookup_indexes.iter().any(|(field_name, _)| {
            // Extract the account type from the lookup index field name
            // For example, "id.bonding_curve" -> check if this is "BondingCurve" account
            field_name
                .split('.')
                .next_back()
                .map(|last_part| {
                    // Convert to account type name (e.g., "bonding_curve" -> "BondingCurve")
                    let account_name = last_part
                        .split('_')
                        .map(|part| {
                            let mut chars = part.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => {
                                    first.to_uppercase().collect::<String>() + chars.as_str()
                                }
                            }
                        })
                        .collect::<String>();
                    account_name == account_type
                })
                .unwrap_or(false)
        });

        if account_has_lookup_index && !is_instruction {
            // This is an account handler with a lookup index configured
            // Use Lookup strategy with __account_address to enable PDA reverse lookup
            KeyResolutionStrategy::Lookup {
                primary_field: FieldPath::new(&["__account_address"]),
            }
        } else {
            KeyResolutionStrategy::Embedded {
                primary_field: FieldPath::new(&[]),
            }
        }
    };

    let type_suffix = if is_instruction { "IxState" } else { "State" };
    Some(SerializableHandlerSpec {
        source: SourceSpec::Source {
            program_id: None,
            discriminator: None,
            type_name: format!("{}{}", account_type, type_suffix),
        },
        key_resolution,
        mappings: serializable_mappings,
        conditions: Vec::new(),
        emit: true,
    })
}

fn build_event_handler(
    instruction: &str,
    join_key: &Option<String>,
    event_mappings: &[(String, parse::EventAttribute, syn::Type)],
    primary_keys: &[String],
    lookup_indexes: &[(String, Option<String>)],
    idl: Option<&idl_parser::IdlSpec>,
) -> Option<SerializableHandlerSpec> {
    let parts: Vec<&str> = instruction.split("::").collect();
    if parts.len() != 2 {
        return None;
    }

    let program_id = parts[0];
    let instruction_type = parts[1];
    let instruction_type_pascal = idl_parser::to_pascal_case(instruction_type);

    let mut serializable_mappings = Vec::new();

    for (target_field, event_attr, _field_type) in event_mappings {
        let has_fields =
            !event_attr.capture_fields.is_empty() || !event_attr.capture_fields_legacy.is_empty();

        let source = if !has_fields {
            MappingSource::AsEvent { fields: vec![] }
        } else if !event_attr.capture_fields.is_empty() {
            let captured_fields: Vec<MappingSource> = event_attr
                .capture_fields
                .iter()
                .map(|field_spec| {
                    let field_name = field_spec.ident.to_string();
                    let transform = event_attr
                        .field_transforms
                        .get(&field_name)
                        .and_then(|t| parse_transformation(&t.to_string()));

                    let field_location = if let Some(explicit_loc) = &field_spec.explicit_location {
                        explicit_loc.clone()
                    } else {
                        let instruction_path = event_attr
                            .from_instruction
                            .as_ref()
                            .or(event_attr.inferred_instruction.as_ref());

                        if let Some(instr_path) = instruction_path {
                            find_field_in_instruction(instr_path, &field_name, idl)
                                .unwrap_or(parse::FieldLocation::InstructionArg)
                        } else {
                            parse::FieldLocation::InstructionArg
                        }
                    };

                    let field_path = match field_location {
                        parse::FieldLocation::Account => FieldPath::new(&["accounts", &field_name]),
                        parse::FieldLocation::InstructionArg => {
                            FieldPath::new(&["data", &field_name])
                        }
                    };

                    MappingSource::FromSource {
                        path: field_path,
                        default: None,
                        transform,
                    }
                })
                .collect();

            MappingSource::AsEvent {
                fields: captured_fields,
            }
        } else {
            let captured_fields: Vec<MappingSource> = event_attr
                .capture_fields_legacy
                .iter()
                .map(|field| {
                    let transform = event_attr
                        .field_transforms_legacy
                        .get(field)
                        .and_then(|t| parse_transformation(t));
                    MappingSource::FromSource {
                        path: FieldPath::new(&["data", field]),
                        default: None,
                        transform,
                    }
                })
                .collect();

            MappingSource::AsEvent {
                fields: captured_fields,
            }
        };

        let population = parse_population_strategy(&event_attr.strategy);

        serializable_mappings.push(SerializableFieldMapping {
            target_path: target_field.clone(),
            source,
            transform: None,
            population,
        });
    }

    // Determine key resolution for events
    let (lookup_field_name, lookup_field_location) = if let Some(ref join_field_name) = join_key {
        (
            join_field_name.clone(),
            parse::FieldLocation::InstructionArg,
        )
    } else if let Some((_, first_event_attr, _)) = event_mappings.first() {
        if let Some(ref lookup_by_field_spec) = first_event_attr.lookup_by {
            let field_name = lookup_by_field_spec.ident.to_string();

            let field_location = if let Some(explicit_loc) = &lookup_by_field_spec.explicit_location
            {
                explicit_loc.clone()
            } else {
                let instruction_path = first_event_attr
                    .from_instruction
                    .as_ref()
                    .or(first_event_attr.inferred_instruction.as_ref());

                if let Some(instr_path) = instruction_path {
                    find_field_in_instruction(instr_path, &field_name, idl)
                        .unwrap_or(parse::FieldLocation::InstructionArg)
                } else {
                    parse::FieldLocation::InstructionArg
                }
            };

            (field_name, field_location)
        } else {
            (String::new(), parse::FieldLocation::InstructionArg)
        }
    } else {
        (String::new(), parse::FieldLocation::InstructionArg)
    };

    let is_temporal_lookup = lookup_indexes.iter().any(|(field, temporal_field)| {
        field.ends_with(&lookup_field_name) && temporal_field.is_some()
    });

    let lookup_field_prefix = match lookup_field_location {
        parse::FieldLocation::Account => "accounts",
        parse::FieldLocation::InstructionArg => "data",
    };

    let key_resolution = if is_temporal_lookup {
        let index_name = format!("{}_temporal_index", lookup_field_name);
        KeyResolutionStrategy::TemporalLookup {
            lookup_field: FieldPath::new(&[lookup_field_prefix, &lookup_field_name]),
            timestamp_field: FieldPath::new(&["timestamp"]),
            index_name,
        }
    } else if !lookup_field_name.is_empty() {
        // Check if lookup_by points directly to a field that matches a lookup_index
        // If the lookup_by field is NOT in the lookup_indexes, it means we're pointing
        // directly to the primary key field itself (e.g., accounts.mint when id.mint is the pk),
        // so we should use Embedded resolution instead of Lookup.
        //
        // Check if any primary key field name matches the lookup_by field name
        // Primary keys are like "id.mint", so we compare the last segment
        let is_primary_key_field = primary_keys
            .iter()
            .any(|pk| pk.split('.').next_back().unwrap_or(pk) == lookup_field_name);

        if is_primary_key_field {
            // The lookup_by field IS the primary key itself - use Embedded
            KeyResolutionStrategy::Embedded {
                primary_field: FieldPath::new(&[lookup_field_prefix, &lookup_field_name]),
            }
        } else {
            // The lookup_by field is NOT the primary key - needs reverse lookup
            KeyResolutionStrategy::Lookup {
                primary_field: FieldPath::new(&[lookup_field_prefix, &lookup_field_name]),
            }
        }
    } else {
        KeyResolutionStrategy::Lookup {
            primary_field: FieldPath::new(&[]),
        }
    };

    Some(SerializableHandlerSpec {
        source: SourceSpec::Source {
            program_id: Some(program_id.to_string()),
            discriminator: None,
            type_name: format!("{}IxState", instruction_type_pascal),
        },
        key_resolution,
        mappings: serializable_mappings,
        conditions: Vec::new(),
        emit: true,
    })
}

// ============================================================================
// Hook Building
// ============================================================================

fn build_resolver_hooks_ast(
    resolver_hooks: &[parse::ResolveKeyAttribute],
    idl: Option<&idl_parser::IdlSpec>,
) -> Vec<ResolverHook> {
    resolver_hooks
        .iter()
        .map(|hook| {
            let account_type = path_to_string(&hook.account_path);
            let account_type_state = format!("{}State", account_type.split("::").last().unwrap());

            let strategy = match hook.strategy.as_str() {
                "pda_reverse_lookup" => {
                    let discriminators = hook
                        .queue_until
                        .iter()
                        .filter_map(|instr_path| {
                            idl.and_then(|idl| {
                                let instr_name = instr_path.segments.last()?.ident.to_string();
                                idl.instructions
                                    .iter()
                                    .find(|instr| instr.name.eq_ignore_ascii_case(&instr_name))
                                    .map(|instr| instr.get_discriminator())
                            })
                        })
                        .collect();

                    ResolverStrategy::PdaReverseLookup {
                        lookup_name: hook
                            .lookup_name
                            .clone()
                            .unwrap_or_else(|| "default_pda_lookup".to_string()),
                        queue_discriminators: discriminators,
                    }
                }
                _ => ResolverStrategy::PdaReverseLookup {
                    lookup_name: "default_pda_lookup".to_string(),
                    queue_discriminators: Vec::new(),
                },
            };

            ResolverHook {
                account_type: account_type_state,
                strategy,
            }
        })
        .collect()
}

fn build_instruction_hooks_ast(
    pda_registrations: &[parse::RegisterPdaAttribute],
    derive_from_mappings: &HashMap<String, Vec<parse::DeriveFromAttribute>>,
    aggregate_conditions: &HashMap<String, String>,
    sources_by_type: &HashMap<String, Vec<parse::MapAttribute>>,
) -> Vec<InstructionHook> {
    // Use BTreeMap for deterministic ordering in the final output
    let mut instruction_hooks_map: BTreeMap<String, InstructionHook> = BTreeMap::new();

    // Process PDA registrations
    for registration in pda_registrations {
        let instr_type = path_to_string(&registration.instruction_path);
        let instr_type_state = format!("{}IxState", instr_type.split("::").last().unwrap());

        let action = HookAction::RegisterPdaMapping {
            pda_field: FieldPath::new(&["accounts", &registration.pda_field.ident.to_string()]),
            seed_field: FieldPath::new(&[
                "accounts",
                &registration.primary_key_field.ident.to_string(),
            ]),
            lookup_name: registration.lookup_name.clone(),
        };

        instruction_hooks_map
            .entry(instr_type_state.clone())
            .or_insert_with(|| InstructionHook {
                instruction_type: instr_type_state,
                actions: Vec::new(),
                lookup_by: None,
            })
            .actions
            .push(action);
    }

    // Process derive_from mappings (sorted for deterministic output)
    let mut sorted_derive_from: Vec<_> = derive_from_mappings.iter().collect();
    sorted_derive_from.sort_by_key(|(k, _)| *k);
    for (instruction_type, derive_attrs) in sorted_derive_from {
        let instr_type_state = format!("{}IxState", instruction_type.split("::").last().unwrap());

        for derive_attr in derive_attrs {
            let source = if derive_attr.field.ident.to_string().starts_with("__") {
                match derive_attr.field.ident.to_string().as_str() {
                    "__timestamp" => MappingSource::FromContext {
                        field: "timestamp".to_string(),
                    },
                    "__slot" => MappingSource::FromContext {
                        field: "slot".to_string(),
                    },
                    "__signature" => MappingSource::FromContext {
                        field: "signature".to_string(),
                    },
                    _ => continue,
                }
            } else {
                let path_prefix = match &derive_attr.field.explicit_location {
                    Some(parse::FieldLocation::Account) => "accounts",
                    Some(parse::FieldLocation::InstructionArg) | None => "data",
                };

                MappingSource::FromSource {
                    path: FieldPath::new(&[path_prefix, &derive_attr.field.ident.to_string()]),
                    default: None,
                    transform: derive_attr
                        .transform
                        .as_ref()
                        .and_then(|t| parse_transformation(&t.to_string())),
                }
            };

            let condition = derive_attr.condition.as_ref().map(|cond| ConditionExpr {
                expression: cond.clone(),
                parsed: condition_parser::parse_condition_expression(cond),
            });

            let action = HookAction::SetField {
                target_field: derive_attr.target_field_name.clone(),
                source,
                condition,
            };

            let lookup_by = derive_attr
                .lookup_by
                .as_ref()
                .map(|field_spec| FieldPath::new(&["accounts", &field_spec.ident.to_string()]));

            let hook = instruction_hooks_map
                .entry(instr_type_state.clone())
                .or_insert_with(|| InstructionHook {
                    instruction_type: instr_type_state.clone(),
                    actions: Vec::new(),
                    lookup_by: lookup_by.clone(),
                });

            hook.actions.push(action);

            if hook.lookup_by.is_none() {
                hook.lookup_by = lookup_by;
            }
        }
    }

    // Process aggregate conditions (sorted for deterministic output)
    let mut sorted_aggregate_conditions: Vec<_> = aggregate_conditions.iter().collect();
    sorted_aggregate_conditions.sort_by_key(|(k, _)| *k);
    let mut sorted_sources: Vec<_> = sources_by_type.iter().collect();
    sorted_sources.sort_by_key(|(k, _)| *k);
    for (field_path, condition_str) in sorted_aggregate_conditions {
        for (source_type, mappings) in &sorted_sources {
            for mapping in *mappings {
                if &mapping.target_field_name == field_path
                    && mapping.is_instruction
                    && matches!(
                        mapping.strategy.as_str(),
                        "Sum" | "Count" | "Min" | "Max" | "UniqueCount"
                    )
                {
                    let instr_type_state =
                        format!("{}IxState", source_type.split("::").last().unwrap());

                    let condition = ConditionExpr {
                        expression: condition_str.clone(),
                        parsed: condition_parser::parse_condition_expression(condition_str),
                    };

                    if mapping.strategy == "Count" {
                        let action = HookAction::IncrementField {
                            target_field: field_path.clone(),
                            increment_by: 1,
                            condition: Some(condition),
                        };

                        instruction_hooks_map
                            .entry(instr_type_state.clone())
                            .or_insert_with(|| InstructionHook {
                                instruction_type: instr_type_state,
                                actions: Vec::new(),
                                lookup_by: None,
                            })
                            .actions
                            .push(action);
                    }
                }
            }
        }
    }

    instruction_hooks_map.into_values().collect()
}
