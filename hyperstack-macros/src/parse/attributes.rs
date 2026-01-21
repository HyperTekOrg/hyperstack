//! Attribute parsing for hyperstack macros.
//!
//! This module parses macro attributes like #[map], #[event], #[snapshot], etc.

#![allow(dead_code)]

use std::collections::HashMap;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Path, Token};

#[derive(Debug, Clone)]
pub struct MapAttribute {
    pub source_type_path: Path,
    pub source_field_name: String,
    pub target_field_name: String,
    pub is_primary_key: bool,
    pub is_lookup_index: bool,
    pub temporal_field: Option<String>,
    pub strategy: String,
    pub join_on: Option<String>,
    pub transform: Option<String>,
    pub is_instruction: bool,
    pub is_whole_source: bool, // NEW: true for whole instruction capture
    pub lookup_by: Option<FieldSpec>, // For aggregate/event lookup
}

#[derive(Debug, Clone)]
pub struct EventAttribute {
    // New type-safe fields
    pub from_instruction: Option<Path>, // Explicit source via `from = ...`
    pub inferred_instruction: Option<Path>, // Inferred from field type
    pub capture_fields: Vec<FieldSpec>, // Field identifiers with location info
    pub field_transforms: HashMap<String, syn::Ident>, // Identifier-based transforms

    // Backward compatibility - old string-based syntax
    pub instruction: String, // Legacy: "program::Instruction" format
    pub capture_fields_legacy: Vec<String>, // Legacy: string-based fields
    pub field_transforms_legacy: HashMap<String, String>, // Legacy: string-based transforms

    // Common fields
    pub strategy: String,
    pub target_field_name: String,
    pub join_on: Option<FieldSpec>,
    pub lookup_by: Option<FieldSpec>,
}

#[derive(Debug, Clone)]
pub struct CaptureAttribute {
    // Type-safe fields for account capture
    pub from_account: Option<Path>, // Explicit source via `from = ...`
    pub inferred_account: Option<Path>, // Inferred from field type

    // Field transformations
    pub field_transforms: HashMap<String, syn::Ident>, // Map field name to transformation

    // Common fields
    pub strategy: String, // Only SetOnce or LastWrite allowed
    pub target_field_name: String,
    pub join_on: Option<FieldSpec>,
    pub lookup_by: Option<FieldSpec>,
}

#[derive(Debug, Clone)]
pub struct FieldSpec {
    pub ident: syn::Ident,
    pub explicit_location: Option<FieldLocation>, // For accounts::mint syntax
}

#[derive(Debug, Clone)]
pub enum FieldLocation {
    InstructionArg,
    Account,
}

impl MapAttribute {
    pub fn source_type_string(&self) -> String {
        self.source_type_path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }
}

struct MapAttributeArgs {
    source_paths: Vec<Path>,
    is_primary_key: bool,
    is_lookup_index: bool,
    temporal_field: Option<String>,
    strategy: Option<String>,
    rename: Option<String>,
    join_on: Option<String>,
    transform: Option<String>,
}

impl Parse for MapAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse single path or array of paths (like AggregateAttributeArgs::from)
        let mut source_paths = Vec::new();
        if input.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);
            while !content.is_empty() {
                source_paths.push(content.parse()?);
                if !content.is_empty() {
                    content.parse::<Token![,]>()?;
                }
            }
        } else {
            source_paths.push(input.parse()?);
        }

        let mut is_primary_key = false;
        let mut is_lookup_index = false;
        let mut temporal_field = None;
        let mut strategy = None;
        let mut rename = None;
        let mut join_on = None;
        let mut transform = None;

        while !input.is_empty() {
            input.parse::<Token![,]>()?;

            if input.is_empty() {
                break;
            }

            if input.peek(syn::Ident) {
                let ident: syn::Ident = input.parse()?;
                let ident_str = ident.to_string();

                if ident_str == "primary_key" {
                    is_primary_key = true;
                } else if ident_str == "lookup_index" {
                    is_lookup_index = true;
                } else if ident_str == "temporal_field" {
                    input.parse::<Token![=]>()?;
                    let temporal_lit: syn::LitStr = input.parse()?;
                    temporal_field = Some(temporal_lit.value());
                } else if ident_str == "strategy" {
                    input.parse::<Token![=]>()?;
                    let strategy_ident: syn::Ident = input.parse()?;
                    strategy = Some(strategy_ident.to_string());
                } else if ident_str == "rename" {
                    input.parse::<Token![=]>()?;
                    let rename_lit: syn::LitStr = input.parse()?;
                    rename = Some(rename_lit.value());
                } else if ident_str == "join_on" {
                    input.parse::<Token![=]>()?;
                    let join_on_lit: syn::LitStr = input.parse()?;
                    join_on = Some(join_on_lit.value());
                } else if ident_str == "transform" {
                    input.parse::<Token![=]>()?;
                    let transform_ident: syn::Ident = input.parse()?;
                    transform = Some(transform_ident.to_string());
                } else {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("Unknown attribute argument: {}", ident_str),
                    ));
                }
            } else {
                return Err(input.error("Expected identifier"));
            }
        }

        Ok(MapAttributeArgs {
            source_paths,
            is_primary_key,
            is_lookup_index,
            temporal_field,
            strategy,
            rename,
            join_on,
            transform,
        })
    }
}

pub fn parse_map_attribute(
    attr: &Attribute,
    target_field_name: &str,
) -> syn::Result<Option<Vec<MapAttribute>>> {
    if !attr.path().is_ident("map") {
        return Ok(None);
    }

    let args: MapAttributeArgs = attr.parse_args()?;

    if args.source_paths.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "#[map] requires at least one source path",
        ));
    }

    let strategy = args.strategy.unwrap_or_else(|| "SetOnce".to_string());
    let target_name = args.rename.unwrap_or_else(|| target_field_name.to_string());

    let mut results = Vec::new();
    for source_path in args.source_paths {
        let (source_type_path, source_field_name) = split_source_path(&source_path)?;

        results.push(MapAttribute {
            source_type_path,
            source_field_name,
            target_field_name: target_name.clone(),
            is_primary_key: args.is_primary_key,
            is_lookup_index: args.is_lookup_index,
            temporal_field: args.temporal_field.clone(),
            strategy: strategy.clone(),
            join_on: args.join_on.clone(),
            transform: args.transform.clone(),
            is_instruction: false,
            is_whole_source: false,
            lookup_by: None,
        });
    }

    Ok(Some(results))
}

pub fn parse_from_instruction_attribute(
    attr: &Attribute,
    target_field_name: &str,
) -> syn::Result<Option<Vec<MapAttribute>>> {
    if !attr.path().is_ident("from_instruction") {
        return Ok(None);
    }

    let args: MapAttributeArgs = attr.parse_args()?;

    if args.source_paths.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "#[from_instruction] requires at least one source path",
        ));
    }

    let strategy = args.strategy.unwrap_or_else(|| "SetOnce".to_string());
    let target_name = args.rename.unwrap_or_else(|| target_field_name.to_string());

    let mut results = Vec::new();
    for source_path in args.source_paths {
        let (source_type_path, source_field_name) = split_source_path(&source_path)?;

        results.push(MapAttribute {
            source_type_path,
            source_field_name,
            target_field_name: target_name.clone(),
            is_primary_key: args.is_primary_key,
            is_lookup_index: args.is_lookup_index,
            temporal_field: args.temporal_field.clone(),
            strategy: strategy.clone(),
            join_on: args.join_on.clone(),
            transform: args.transform.clone(),
            is_instruction: true,
            is_whole_source: false,
            lookup_by: None,
        });
    }

    Ok(Some(results))
}

fn split_source_path(path: &Path) -> syn::Result<(Path, String)> {
    if path.segments.len() < 2 {
        return Err(syn::Error::new_spanned(
            path,
            "Source path must be in format ModulePath::TypeName::field_name",
        ));
    }

    let field_name = path.segments.last().unwrap().ident.to_string();

    let mut type_path = path.clone();
    type_path.segments.pop();

    Ok((type_path, field_name))
}

struct EventAttributeArgs {
    // New type-safe syntax
    from: Option<Path>,
    fields: Option<Vec<FieldSpec>>,
    transforms: Option<Vec<FieldTransform>>,

    // Backward compatibility
    instruction: Option<String>,
    capture: Option<Vec<String>>,
    transforms_legacy: Option<HashMap<String, String>>,

    // Common fields
    strategy: Option<syn::Ident>,
    rename: Option<String>,
    join_on: Option<FieldSpec>,
    lookup_by: Option<FieldSpec>,
}

struct FieldTransform {
    field: syn::Ident,
    transform: syn::Ident,
}

impl Parse for EventAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut from = None;
        let mut fields = None;
        let mut transforms = None;
        let mut transforms_legacy = None;
        let mut instruction = None;
        let mut capture = None;
        let mut strategy = None;
        let mut rename = None;
        let mut join_on = None;
        let mut lookup_by = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let ident_str = ident.to_string();

            input.parse::<Token![=]>()?;

            if ident_str == "from" {
                // New: from = InstructionType
                from = Some(input.parse()?);
            } else if ident_str == "fields" {
                // New: fields = [ident1, ident2] or fields = [accounts::ident1, args::ident2]
                let content;
                syn::bracketed!(content in input);

                let mut field_specs = Vec::new();
                while !content.is_empty() {
                    field_specs.push(parse_field_spec(&content)?);

                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }
                fields = Some(field_specs);
            } else if ident_str == "instruction" {
                // Legacy: instruction = "string"
                let lit: syn::LitStr = input.parse()?;
                instruction = Some(lit.value());
            } else if ident_str == "capture" {
                // Legacy/Alias for fields: capture = ["string1", "string2"]
                let content;
                syn::bracketed!(content in input);

                // Try to parse as identifiers first, fall back to strings
                let mut field_specs = Vec::new();
                let mut string_fields = Vec::new();
                let mut is_string_mode = false;

                while !content.is_empty() {
                    if content.peek(syn::LitStr) {
                        // Legacy string mode
                        is_string_mode = true;
                        let field_lit: syn::LitStr = content.parse()?;
                        string_fields.push(field_lit.value());
                    } else {
                        // New identifier mode
                        field_specs.push(parse_field_spec(&content)?);
                    }

                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }

                if is_string_mode {
                    capture = Some(string_fields);
                } else {
                    fields = Some(field_specs);
                }
            } else if ident_str == "transforms" {
                // Try to parse as new syntax first: [(ident1, Transform1), (ident2, Transform2)]
                let content;
                syn::bracketed!(content in input);

                let mut new_transforms = Vec::new();
                let mut legacy_transforms = HashMap::new();
                let mut is_legacy = false;

                while !content.is_empty() {
                    let tuple_content;
                    syn::parenthesized!(tuple_content in content);

                    if tuple_content.peek(syn::LitStr) {
                        // Legacy: ("string", Transform)
                        is_legacy = true;
                        let field_name: syn::LitStr = tuple_content.parse()?;
                        tuple_content.parse::<Token![,]>()?;
                        let transform_type: syn::Ident = tuple_content.parse()?;
                        legacy_transforms.insert(field_name.value(), transform_type.to_string());
                    } else {
                        // New: (ident, Transform)
                        let field: syn::Ident = tuple_content.parse()?;
                        tuple_content.parse::<Token![,]>()?;
                        let transform_type: syn::Ident = tuple_content.parse()?;
                        new_transforms.push(FieldTransform {
                            field,
                            transform: transform_type,
                        });
                    }

                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }

                if is_legacy {
                    transforms_legacy = Some(legacy_transforms);
                } else {
                    transforms = Some(new_transforms);
                }
            } else if ident_str == "strategy" {
                strategy = Some(input.parse()?);
            } else if ident_str == "rename" {
                let rename_lit: syn::LitStr = input.parse()?;
                rename = Some(rename_lit.value());
            } else if ident_str == "join_on" {
                // Try identifier first, fall back to string
                if input.peek(syn::LitStr) {
                    let join_on_lit: syn::LitStr = input.parse()?;
                    let ident = syn::Ident::new(&join_on_lit.value(), join_on_lit.span());
                    join_on = Some(FieldSpec {
                        ident,
                        explicit_location: None,
                    });
                } else {
                    join_on = Some(parse_field_spec(input)?);
                }
            } else if ident_str == "lookup_by" {
                // Try identifier first, fall back to string
                if input.peek(syn::LitStr) {
                    let lookup_by_lit: syn::LitStr = input.parse()?;
                    let ident = syn::Ident::new(&lookup_by_lit.value(), lookup_by_lit.span());
                    lookup_by = Some(FieldSpec {
                        ident,
                        explicit_location: None,
                    });
                } else {
                    lookup_by = Some(parse_field_spec(input)?);
                }
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("Unknown event attribute argument: {}", ident_str),
                ));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(EventAttributeArgs {
            from,
            fields,
            transforms,
            instruction,
            capture,
            transforms_legacy,
            strategy,
            rename,
            join_on,
            lookup_by,
        })
    }
}

// Helper function to parse a field spec (ident or location::ident)
fn parse_field_spec(input: ParseStream) -> syn::Result<FieldSpec> {
    let lookahead = input.lookahead1();

    if lookahead.peek(syn::Ident) {
        let first_ident: syn::Ident = input.parse()?;

        // Check if there's a :: following
        if input.peek(Token![::]) {
            input.parse::<Token![::]>()?;
            let second_ident: syn::Ident = input.parse()?;

            // first_ident is the location (accounts, args, data)
            let location = match first_ident.to_string().as_str() {
                "accounts" => Some(FieldLocation::Account),
                "args" | "data" => Some(FieldLocation::InstructionArg),
                _ => {
                    return Err(syn::Error::new(
                        first_ident.span(),
                        format!(
                            "Invalid field location '{}'. Use 'accounts', 'args', or 'data'",
                            first_ident
                        ),
                    ));
                }
            };

            Ok(FieldSpec {
                ident: second_ident,
                explicit_location: location,
            })
        } else {
            // Just an identifier without location
            Ok(FieldSpec {
                ident: first_ident,
                explicit_location: None,
            })
        }
    } else {
        Err(lookahead.error())
    }
}

pub fn parse_event_attribute(
    attr: &Attribute,
    target_field_name: &str,
) -> syn::Result<Option<EventAttribute>> {
    if !attr.path().is_ident("event") {
        return Ok(None);
    }

    let args: EventAttributeArgs = attr.parse_args()?;

    let target_name = args.rename.unwrap_or_else(|| target_field_name.to_string());

    // Convert new-style transforms to HashMap
    let field_transforms = if let Some(transforms) = args.transforms {
        transforms
            .into_iter()
            .map(|ft| (ft.field.to_string(), ft.transform))
            .collect()
    } else {
        HashMap::new()
    };

    // For backward compatibility, convert legacy transforms
    let field_transforms_legacy = args.transforms_legacy.unwrap_or_default();

    // Determine strategy
    let strategy = args
        .strategy
        .map(|s| s.to_string())
        .unwrap_or_else(|| "SetOnce".to_string());

    // Handle legacy instruction string
    let instruction_str = args.instruction.unwrap_or_default();

    Ok(Some(EventAttribute {
        from_instruction: args.from,
        inferred_instruction: None, // Will be filled in later from field type
        capture_fields: args.fields.unwrap_or_default(),
        field_transforms,
        instruction: instruction_str,
        capture_fields_legacy: args.capture.unwrap_or_default(),
        field_transforms_legacy,
        strategy,
        target_field_name: target_name,
        join_on: args.join_on,
        lookup_by: args.lookup_by,
    }))
}

// Parse args for #[snapshot] attribute
struct SnapshotAttributeArgs {
    from: Option<Path>,
    strategy: Option<syn::Ident>,
    rename: Option<String>,
    join_on: Option<FieldSpec>,
    lookup_by: Option<FieldSpec>,
    transforms: Vec<(String, syn::Ident)>, // Field transformations: (field_name, transform)
}

impl Parse for SnapshotAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut from = None;
        let mut strategy = None;
        let mut rename = None;
        let mut join_on = None;
        let mut lookup_by = None;
        let mut transforms = Vec::new();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let ident_str = ident.to_string();

            input.parse::<Token![=]>()?;

            if ident_str == "from" {
                from = Some(input.parse()?);
            } else if ident_str == "strategy" {
                strategy = Some(input.parse()?);
            } else if ident_str == "rename" {
                let rename_lit: syn::LitStr = input.parse()?;
                rename = Some(rename_lit.value());
            } else if ident_str == "join_on" {
                if input.peek(syn::LitStr) {
                    let join_on_lit: syn::LitStr = input.parse()?;
                    let ident = syn::Ident::new(&join_on_lit.value(), join_on_lit.span());
                    join_on = Some(FieldSpec {
                        ident,
                        explicit_location: None,
                    });
                } else {
                    join_on = Some(parse_field_spec(input)?);
                }
            } else if ident_str == "lookup_by" {
                if input.peek(syn::LitStr) {
                    let lookup_by_lit: syn::LitStr = input.parse()?;
                    let ident = syn::Ident::new(&lookup_by_lit.value(), lookup_by_lit.span());
                    lookup_by = Some(FieldSpec {
                        ident,
                        explicit_location: None,
                    });
                } else {
                    lookup_by = Some(parse_field_spec(input)?);
                }
            } else if ident_str == "transforms" {
                // Parse transforms = [(field1, Transform1), (field2, Transform2)]
                let content;
                syn::bracketed!(content in input);

                while !content.is_empty() {
                    let tuple_content;
                    syn::parenthesized!(tuple_content in content);

                    let field_name: syn::Ident = tuple_content.parse()?;
                    tuple_content.parse::<Token![,]>()?;
                    let transform: syn::Ident = tuple_content.parse()?;

                    transforms.push((field_name.to_string(), transform));

                    if !content.is_empty() {
                        content.parse::<Token![,]>()?;
                    }
                }
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("Unknown snapshot attribute argument: {}", ident_str),
                ));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(SnapshotAttributeArgs {
            from,
            strategy,
            rename,
            join_on,
            lookup_by,
            transforms,
        })
    }
}

pub fn parse_snapshot_attribute(
    attr: &Attribute,
    target_field_name: &str,
) -> syn::Result<Option<CaptureAttribute>> {
    if !attr.path().is_ident("snapshot") {
        return Ok(None);
    }

    let args: SnapshotAttributeArgs = attr.parse_args()?;

    let target_name = args.rename.unwrap_or_else(|| target_field_name.to_string());

    // Determine strategy - only SetOnce or LastWrite allowed
    let strategy = args
        .strategy
        .as_ref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "SetOnce".to_string());

    // Validate strategy
    if strategy != "SetOnce" && strategy != "LastWrite" {
        if let Some(ref strategy_ident) = args.strategy {
            return Err(syn::Error::new_spanned(
                strategy_ident,
                format!("Invalid strategy '{}' for #[snapshot]. Only 'SetOnce' or 'LastWrite' are allowed. Account snapshots cannot use 'Append'.", strategy)
            ));
        }
    }

    Ok(Some(CaptureAttribute {
        from_account: args.from,
        inferred_account: None, // Will be filled in later from field type
        field_transforms: args.transforms.into_iter().collect(),
        strategy,
        target_field_name: target_name,
        join_on: args.join_on,
        lookup_by: args.lookup_by,
    }))
}

// ============================================================================
// Aggregate Macro - Declarative Aggregations
// ============================================================================

#[derive(Debug, Clone)]
pub struct AggregateAttribute {
    /// Instruction type(s) to aggregate from
    pub from_instructions: Vec<Path>,
    /// Field to aggregate (optional - if omitted, just count occurrences)
    pub field: Option<FieldSpec>,
    /// Aggregation strategy
    pub strategy: String,
    /// Transform to apply to the field before aggregating
    pub transform: Option<syn::Ident>,
    /// Target field name (defaults to struct field name)
    pub target_field_name: String,
    /// Join condition for multi-entity scenarios
    pub join_on: Option<FieldSpec>,
    /// Lookup field for key resolution
    pub lookup_by: Option<FieldSpec>,
    /// Condition expression for conditional aggregation (Level 1)
    pub condition: Option<String>,
}

struct AggregateAttributeArgs {
    from: Vec<Path>,
    field: Option<FieldSpec>,
    strategy: Option<syn::Ident>,
    transform: Option<syn::Ident>,
    rename: Option<String>,
    join_on: Option<FieldSpec>,
    lookup_by: Option<FieldSpec>,
    condition: Option<String>,
}

impl Parse for AggregateAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut from = Vec::new();
        let mut field = None;
        let mut strategy = None;
        let mut transform = None;
        let mut rename = None;
        let mut join_on = None;
        let mut lookup_by = None;
        let mut condition = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let ident_str = ident.to_string();

            input.parse::<Token![=]>()?;

            if ident_str == "from" {
                // Parse single instruction or array of instructions
                if input.peek(syn::token::Bracket) {
                    let content;
                    syn::bracketed!(content in input);
                    while !content.is_empty() {
                        from.push(content.parse()?);
                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                } else {
                    from.push(input.parse()?);
                }
            } else if ident_str == "field" {
                field = Some(parse_field_spec(input)?);
            } else if ident_str == "strategy" {
                strategy = Some(input.parse()?);
            } else if ident_str == "transform" {
                transform = Some(input.parse()?);
            } else if ident_str == "rename" {
                let rename_lit: syn::LitStr = input.parse()?;
                rename = Some(rename_lit.value());
            } else if ident_str == "join_on" {
                if input.peek(syn::LitStr) {
                    let join_on_lit: syn::LitStr = input.parse()?;
                    let ident = syn::Ident::new(&join_on_lit.value(), join_on_lit.span());
                    join_on = Some(FieldSpec {
                        ident,
                        explicit_location: None,
                    });
                } else {
                    join_on = Some(parse_field_spec(input)?);
                }
            } else if ident_str == "lookup_by" {
                if input.peek(syn::LitStr) {
                    let lookup_by_lit: syn::LitStr = input.parse()?;
                    let ident = syn::Ident::new(&lookup_by_lit.value(), lookup_by_lit.span());
                    lookup_by = Some(FieldSpec {
                        ident,
                        explicit_location: None,
                    });
                } else {
                    lookup_by = Some(parse_field_spec(input)?);
                }
            } else if ident_str == "condition" {
                let condition_lit: syn::LitStr = input.parse()?;
                condition = Some(condition_lit.value());
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("Unknown aggregate attribute argument: {}", ident_str),
                ));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(AggregateAttributeArgs {
            from,
            field,
            strategy,
            transform,
            rename,
            join_on,
            lookup_by,
            condition,
        })
    }
}

pub fn parse_aggregate_attribute(
    attr: &Attribute,
    target_field_name: &str,
) -> syn::Result<Option<AggregateAttribute>> {
    if !attr.path().is_ident("aggregate") {
        return Ok(None);
    }

    let args: AggregateAttributeArgs = attr.parse_args()?;

    if args.from.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "#[aggregate] requires 'from' parameter specifying instruction type(s)",
        ));
    }

    let target_name = args.rename.unwrap_or_else(|| target_field_name.to_string());

    // Determine strategy - default to Count if no field specified, Sum otherwise
    let strategy = if let Some(ref strategy_ident) = args.strategy {
        let strategy_str = strategy_ident.to_string();

        // Validate strategy
        let valid_strategies = ["Sum", "Count", "Min", "Max", "UniqueCount"];
        if !valid_strategies.contains(&strategy_str.as_str()) {
            return Err(syn::Error::new_spanned(
                strategy_ident,
                format!(
                    "Invalid aggregation strategy '{}'. Valid strategies: {}",
                    strategy_str,
                    valid_strategies.join(", ")
                ),
            ));
        }

        strategy_str
    } else {
        // Default strategy based on whether field is specified
        if args.field.is_none() {
            "Count".to_string()
        } else {
            "Sum".to_string()
        }
    };

    Ok(Some(AggregateAttribute {
        from_instructions: args.from,
        field: args.field,
        strategy,
        transform: args.transform,
        target_field_name: target_name,
        join_on: args.join_on,
        lookup_by: args.lookup_by,
        condition: args.condition,
    }))
}

// ============================================================================
// Computed Macro - Declarative Computed Fields
// ============================================================================

#[derive(Debug, Clone)]
pub struct ComputedAttribute {
    /// The expression to evaluate (stored as TokenStream for code generation)
    pub expression: proc_macro2::TokenStream,
    /// Target field name (defaults to struct field name)
    pub target_field_name: String,
}

/// Parse #[computed(expression)] attribute
pub fn parse_computed_attribute(
    attr: &Attribute,
    target_field_name: &str,
) -> syn::Result<Option<ComputedAttribute>> {
    if !attr.path().is_ident("computed") {
        return Ok(None);
    }

    // Parse the expression inside the attribute
    // e.g., #[computed(total_buy_volume.unwrap_or(0) + total_sell_volume.unwrap_or(0))]
    let expression: proc_macro2::TokenStream = attr.parse_args()?;

    Ok(Some(ComputedAttribute {
        expression,
        target_field_name: target_field_name.to_string(),
    }))
}

pub fn has_entity_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("entity"))
}

pub fn parse_entity_name(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("entity") {
            if let syn::Meta::List(meta_list) = &attr.meta {
                let tokens_str = meta_list.tokens.to_string();
                if tokens_str.contains("name") {
                    if let Ok(parsed) = syn::parse_str::<syn::ExprAssign>(&tokens_str) {
                        if let syn::Expr::Lit(expr_lit) = &*parsed.right {
                            if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                                return Some(lit_str.value());
                            }
                        }
                    }
                }
            }
            return None;
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct StreamSpecAttribute {
    pub proto_files: Vec<String>,
    pub idl_file: String,
    pub skip_decoders: bool,
}

struct StreamSpecAttributeArgs {
    proto_files: Vec<String>,
    idl_file: Option<String>,
    skip_decoders: bool,
}

impl Parse for StreamSpecAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut proto_files = Vec::new();
        let mut idl_file = None;
        let mut skip_decoders = false;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let ident_str = ident.to_string();

            if ident_str == "proto" {
                input.parse::<Token![=]>()?;

                if input.peek(syn::LitStr) {
                    let lit: syn::LitStr = input.parse()?;
                    proto_files.push(lit.value());
                } else if input.peek(syn::token::Bracket) {
                    let content;
                    syn::bracketed!(content in input);

                    while !content.is_empty() {
                        let file_lit: syn::LitStr = content.parse()?;
                        proto_files.push(file_lit.value());

                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                } else {
                    return Err(input.error("Expected string literal or array of string literals"));
                }
            } else if ident_str == "idl" {
                input.parse::<Token![=]>()?;
                let lit: syn::LitStr = input.parse()?;
                idl_file = Some(lit.value());
            } else if ident_str == "skip_decoders" {
                skip_decoders = true;
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("Unknown stream_spec attribute argument: {}", ident_str),
                ));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(StreamSpecAttributeArgs {
            proto_files,
            idl_file,
            skip_decoders,
        })
    }
}

pub fn parse_stream_spec_attribute(
    attr: proc_macro::TokenStream,
) -> Result<StreamSpecAttribute, syn::Error> {
    if attr.is_empty() {
        return Ok(StreamSpecAttribute {
            proto_files: Vec::new(),
            idl_file: String::new(),
            skip_decoders: false,
        });
    }

    let args: StreamSpecAttributeArgs = syn::parse(attr)?;

    Ok(StreamSpecAttribute {
        proto_files: args.proto_files,
        idl_file: args.idl_file.unwrap_or_default(),
        skip_decoders: args.skip_decoders,
    })
}

/// Parse #[resolve_key_for(AccountType)] attribute
pub fn parse_resolve_key_for_attribute(
    attr: &Attribute,
) -> syn::Result<Option<ResolverKeyForAttr>> {
    if !attr.path().is_ident("resolve_key_for") {
        return Ok(None);
    }

    // Parse the account type path inside the attribute
    let account_type_path: Path = attr.parse_args()?;

    Ok(Some(ResolverKeyForAttr { account_type_path }))
}

#[derive(Debug, Clone)]
pub struct ResolverKeyForAttr {
    pub account_type_path: Path,
}

/// Parse #[after_instruction(InstructionType)] attribute
///
/// NOTE: This attribute is no longer supported for direct use by users.
/// All instruction hooks must be defined using declarative macros:
/// - Use #[register_pda] to register PDA mappings
/// - Use #[derive_from] to derive fields from instructions
/// - Use #[aggregate] for aggregations with optional conditions
pub fn parse_after_instruction_attribute(
    attr: &Attribute,
) -> syn::Result<Option<AfterInstructionAttr>> {
    if !attr.path().is_ident("after_instruction") {
        return Ok(None);
    }

    // Reject all user-written #[after_instruction] attributes
    Err(syn::Error::new_spanned(
        attr,
        "Direct use of #[after_instruction] is not allowed.\n\
         \n\
         Use declarative macros instead:\n\
         \n\
         • Use #[register_pda(instruction = ..., pda_field = ..., primary_key = ...)] \
         to register PDA mappings\n\
         \n\
         • Use #[derive_from(from = [...], field = ..., strategy = ...)] \
         to derive fields from instructions\n\
         \n\
         • Use #[aggregate(from = [...], field = ..., strategy = ..., condition = \"...\")] \
         for conditional aggregations\n\
         \n\
          These declarative macros are captured in the AST and enable secure cloud deployment.\n\
         See documentation for examples.",
    ))
}

#[derive(Debug, Clone)]
pub struct AfterInstructionAttr {
    pub instruction_type_path: Path,
}

/// Extract resolver hooks from an impl block
pub fn extract_resolver_hooks(item_impl: &syn::ItemImpl) -> Vec<ResolverHookSpec> {
    let mut hooks = Vec::new();

    for item in &item_impl.items {
        if let syn::ImplItem::Fn(method) = item {
            for attr in &method.attrs {
                if let Ok(Some(resolver_attr)) = parse_resolve_key_for_attribute(attr) {
                    hooks.push(ResolverHookSpec {
                        kind: ResolverHookKind::KeyResolver,
                        account_type_path: resolver_attr.account_type_path,
                        fn_name: method.sig.ident.clone(),
                        fn_sig: method.sig.clone(),
                    });
                }

                if let Ok(Some(instruction_attr)) = parse_after_instruction_attribute(attr) {
                    hooks.push(ResolverHookSpec {
                        kind: ResolverHookKind::AfterInstruction,
                        account_type_path: instruction_attr.instruction_type_path,
                        fn_name: method.sig.ident.clone(),
                        fn_sig: method.sig.clone(),
                    });
                }
            }
        }
    }

    hooks
}

/// Extract resolver hooks from a standalone function
pub fn extract_resolver_hooks_from_fn(item_fn: &syn::ItemFn) -> Vec<ResolverHookSpec> {
    let mut hooks = Vec::new();

    for attr in &item_fn.attrs {
        if let Ok(Some(resolver_attr)) = parse_resolve_key_for_attribute(attr) {
            hooks.push(ResolverHookSpec {
                kind: ResolverHookKind::KeyResolver,
                account_type_path: resolver_attr.account_type_path,
                fn_name: item_fn.sig.ident.clone(),
                fn_sig: item_fn.sig.clone(),
            });
        }

        if let Ok(Some(instruction_attr)) = parse_after_instruction_attribute(attr) {
            hooks.push(ResolverHookSpec {
                kind: ResolverHookKind::AfterInstruction,
                account_type_path: instruction_attr.instruction_type_path,
                fn_name: item_fn.sig.ident.clone(),
                fn_sig: item_fn.sig.clone(),
            });
        }
    }

    hooks
}

#[derive(Debug, Clone)]
pub struct ResolverHookSpec {
    pub kind: ResolverHookKind,
    pub account_type_path: Path,
    pub fn_name: syn::Ident,
    pub fn_sig: syn::Signature,
}

#[derive(Debug, Clone)]
pub enum ResolverHookKind {
    KeyResolver,
    AfterInstruction,
}

// ============================================================================
// Level 1: New Declarative Macros
// ============================================================================

// #[derive_from] Attribute Parser
#[derive(Debug, Clone)]
pub struct DeriveFromAttribute {
    pub from_instructions: Vec<Path>,
    pub field: FieldSpec, // Can be special: __timestamp, __slot, __signature
    pub strategy: String, // LastWrite, SetOnce
    pub lookup_by: Option<FieldSpec>,
    pub condition: Option<String>,
    pub transform: Option<syn::Ident>,
    pub target_field_name: String,
}

struct DeriveFromAttributeArgs {
    from: Vec<Path>,
    field: Option<FieldSpec>,
    strategy: Option<syn::Ident>,
    lookup_by: Option<FieldSpec>,
    condition: Option<String>,
    transform: Option<syn::Ident>,
}

impl Parse for DeriveFromAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut from = Vec::new();
        let mut field = None;
        let mut strategy = None;
        let mut lookup_by = None;
        let mut condition = None;
        let mut transform = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let ident_str = ident.to_string();

            input.parse::<Token![=]>()?;

            if ident_str == "from" {
                // Parse single instruction or array of instructions
                if input.peek(syn::token::Bracket) {
                    let content;
                    syn::bracketed!(content in input);
                    while !content.is_empty() {
                        from.push(content.parse()?);
                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                } else {
                    from.push(input.parse()?);
                }
            } else if ident_str == "field" {
                field = Some(parse_field_spec(input)?);
            } else if ident_str == "strategy" {
                strategy = Some(input.parse()?);
            } else if ident_str == "lookup_by" {
                if input.peek(syn::LitStr) {
                    let lookup_by_lit: syn::LitStr = input.parse()?;
                    let ident = syn::Ident::new(&lookup_by_lit.value(), lookup_by_lit.span());
                    lookup_by = Some(FieldSpec {
                        ident,
                        explicit_location: None,
                    });
                } else {
                    lookup_by = Some(parse_field_spec(input)?);
                }
            } else if ident_str == "condition" {
                let condition_lit: syn::LitStr = input.parse()?;
                condition = Some(condition_lit.value());
            } else if ident_str == "transform" {
                transform = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("Unknown derive_from attribute argument: {}", ident_str),
                ));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(DeriveFromAttributeArgs {
            from,
            field,
            strategy,
            lookup_by,
            condition,
            transform,
        })
    }
}

pub fn parse_derive_from_attribute(
    attr: &Attribute,
    target_field_name: &str,
) -> syn::Result<Option<DeriveFromAttribute>> {
    if !attr.path().is_ident("derive_from") {
        return Ok(None);
    }

    let args: DeriveFromAttributeArgs = attr.parse_args()?;

    if args.from.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "#[derive_from] requires 'from' parameter specifying instruction type(s)",
        ));
    }

    let field = args.field.ok_or_else(|| {
        syn::Error::new_spanned(attr, "#[derive_from] requires 'field' parameter")
    })?;

    let strategy = args
        .strategy
        .map(|s| s.to_string())
        .unwrap_or_else(|| "LastWrite".to_string());

    Ok(Some(DeriveFromAttribute {
        from_instructions: args.from,
        field,
        strategy,
        lookup_by: args.lookup_by,
        condition: args.condition,
        transform: args.transform,
        target_field_name: target_field_name.to_string(),
    }))
}

// #[resolve_key] Attribute Parser
#[derive(Debug, Clone)]
pub struct ResolveKeyAttribute {
    pub account_path: Path,
    pub strategy: String, // "pda_reverse_lookup", "direct_field"
    pub lookup_name: Option<String>,
    pub queue_until: Vec<Path>, // Instruction paths
}

struct ResolveKeyAttributeArgs {
    account: Option<Path>,
    strategy: Option<syn::Ident>,
    lookup_name: Option<String>,
    queue_until: Vec<Path>,
}

impl Parse for ResolveKeyAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut account = None;
        let mut strategy = None;
        let mut lookup_name = None;
        let mut queue_until = Vec::new();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let ident_str = ident.to_string();

            input.parse::<Token![=]>()?;

            if ident_str == "account" {
                account = Some(input.parse()?);
            } else if ident_str == "strategy" {
                let strategy_lit: syn::LitStr = input.parse()?;
                strategy = Some(syn::Ident::new(&strategy_lit.value(), strategy_lit.span()));
            } else if ident_str == "lookup_name" {
                let lookup_name_lit: syn::LitStr = input.parse()?;
                lookup_name = Some(lookup_name_lit.value());
            } else if ident_str == "queue_until" {
                if input.peek(syn::token::Bracket) {
                    let content;
                    syn::bracketed!(content in input);
                    while !content.is_empty() {
                        queue_until.push(content.parse()?);
                        if !content.is_empty() {
                            content.parse::<Token![,]>()?;
                        }
                    }
                } else {
                    queue_until.push(input.parse()?);
                }
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("Unknown resolve_key attribute argument: {}", ident_str),
                ));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ResolveKeyAttributeArgs {
            account,
            strategy,
            lookup_name,
            queue_until,
        })
    }
}

pub fn parse_resolve_key_attribute(attr: &Attribute) -> syn::Result<Option<ResolveKeyAttribute>> {
    if !attr.path().is_ident("resolve_key") {
        return Ok(None);
    }

    let args: ResolveKeyAttributeArgs = attr.parse_args()?;

    let account_path = args.account.ok_or_else(|| {
        syn::Error::new_spanned(attr, "#[resolve_key] requires 'account' parameter")
    })?;

    let strategy = args
        .strategy
        .map(|s| s.to_string())
        .unwrap_or_else(|| "pda_reverse_lookup".to_string());

    Ok(Some(ResolveKeyAttribute {
        account_path,
        strategy,
        lookup_name: args.lookup_name,
        queue_until: args.queue_until,
    }))
}

// #[register_pda] Attribute Parser
#[derive(Debug, Clone)]
pub struct RegisterPdaAttribute {
    pub instruction_path: Path,
    pub pda_field: FieldSpec,
    pub primary_key_field: FieldSpec, // The primary key to associate with the PDA
    pub lookup_name: String,
}

struct RegisterPdaAttributeArgs {
    instruction: Option<Path>,
    pda_field: Option<FieldSpec>,
    primary_key: Option<FieldSpec>,
    lookup_name: Option<String>,
}

impl Parse for RegisterPdaAttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut instruction = None;
        let mut pda_field = None;
        let mut primary_key = None;
        let mut lookup_name = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let ident_str = ident.to_string();

            input.parse::<Token![=]>()?;

            if ident_str == "instruction" {
                instruction = Some(input.parse()?);
            } else if ident_str == "pda_field" {
                pda_field = Some(parse_field_spec(input)?);
            } else if ident_str == "primary_key" {
                primary_key = Some(parse_field_spec(input)?);
            } else if ident_str == "lookup_name" {
                let lookup_name_lit: syn::LitStr = input.parse()?;
                lookup_name = Some(lookup_name_lit.value());
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("Unknown register_pda attribute argument: {}", ident_str),
                ));
            }

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(RegisterPdaAttributeArgs {
            instruction,
            pda_field,
            primary_key,
            lookup_name,
        })
    }
}

pub fn parse_register_pda_attribute(attr: &Attribute) -> syn::Result<Option<RegisterPdaAttribute>> {
    if !attr.path().is_ident("register_pda") {
        return Ok(None);
    }

    let args: RegisterPdaAttributeArgs = attr.parse_args()?;

    let instruction_path = args.instruction.ok_or_else(|| {
        syn::Error::new_spanned(attr, "#[register_pda] requires 'instruction' parameter")
    })?;

    let pda_field = args.pda_field.ok_or_else(|| {
        syn::Error::new_spanned(attr, "#[register_pda] requires 'pda_field' parameter")
    })?;

    let primary_key_field = args.primary_key.ok_or_else(|| {
        syn::Error::new_spanned(attr, "#[register_pda] requires 'primary_key' parameter")
    })?;

    let lookup_name = args
        .lookup_name
        .unwrap_or_else(|| "default_pda_lookup".to_string());

    Ok(Some(RegisterPdaAttribute {
        instruction_path,
        pda_field,
        primary_key_field,
        lookup_name,
    }))
}

/// Parse #[view(name = "latest", sort_by = "id.round_id", order = "desc")] attributes
pub fn parse_view_attributes(attrs: &[Attribute]) -> Vec<crate::ast::ViewDef> {
    use crate::ast::{FieldPath, SortOrder, ViewDef, ViewOutput, ViewSource, ViewTransform};

    let mut views = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("view") {
            continue;
        }

        let mut name: Option<String> = None;
        let mut sort_by: Option<String> = None;
        let mut order = SortOrder::Desc;
        let mut take: Option<usize> = None;
        let mut output = ViewOutput::Single;

        if let syn::Meta::List(meta_list) = &attr.meta {
            let _ = meta_list.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    name = Some(value.value());
                } else if meta.path.is_ident("sort_by") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    sort_by = Some(value.value());
                } else if meta.path.is_ident("order") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    order = match value.value().to_lowercase().as_str() {
                        "asc" => SortOrder::Asc,
                        _ => SortOrder::Desc,
                    };
                } else if meta.path.is_ident("take") {
                    let value: syn::LitInt = meta.value()?.parse()?;
                    take = Some(value.base10_parse::<usize>()?);
                    output = ViewOutput::Collection;
                } else if meta.path.is_ident("output") {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    output = match value.value().to_lowercase().as_str() {
                        "collection" => ViewOutput::Collection,
                        _ => ViewOutput::Single,
                    };
                }
                Ok(())
            });
        }

        if let (Some(view_name), Some(sort_field)) = (name, sort_by) {
            // Convert each segment to camelCase to match the serialized JSON field names
            let segments: Vec<String> = sort_field
                .split('.')
                .map(|s| crate::utils::to_camel_case(s))
                .collect();
            let mut pipeline = vec![ViewTransform::Sort {
                key: FieldPath {
                    segments,
                    offsets: None,
                },
                order,
            }];

            if let Some(n) = take {
                pipeline.push(ViewTransform::Take { count: n });
            } else {
                pipeline.push(ViewTransform::First);
            }

            views.push(ViewDef {
                id: view_name,
                source: ViewSource::Entity {
                    name: String::new(),
                },
                pipeline,
                output,
            });
        }
    }

    views
}
