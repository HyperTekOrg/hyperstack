use crate::ast::*;
use std::collections::{BTreeMap, HashSet};

/// Output structure for TypeScript generation
#[derive(Debug, Clone)]
pub struct TypeScriptOutput {
    pub interfaces: String,
    pub stack_definition: String,
    pub imports: String,
}

impl TypeScriptOutput {
    pub fn full_file(&self) -> String {
        format!(
            "{}\n\n{}\n\n{}",
            self.imports, self.interfaces, self.stack_definition
        )
    }
}

/// Configuration for TypeScript generation
#[derive(Debug, Clone)]
pub struct TypeScriptConfig {
    pub package_name: String,
    pub generate_helpers: bool,
    pub interface_prefix: String,
    pub export_const_name: String,
}

impl Default for TypeScriptConfig {
    fn default() -> Self {
        Self {
            package_name: "hyperstack-react".to_string(),
            generate_helpers: true,
            interface_prefix: "".to_string(),
            export_const_name: "STACK".to_string(),
        }
    }
}

/// Trait for generating TypeScript code from AST components
pub trait TypeScriptGenerator {
    fn generate_typescript(&self, config: &TypeScriptConfig) -> String;
}

/// Trait for generating TypeScript interfaces
pub trait TypeScriptInterfaceGenerator {
    fn generate_interface(&self, name: &str, config: &TypeScriptConfig) -> String;
}

/// Trait for generating TypeScript type mappings
pub trait TypeScriptTypeMapper {
    fn to_typescript_type(&self) -> String;
}

/// Main TypeScript compiler for stream specs
pub struct TypeScriptCompiler<S> {
    spec: TypedStreamSpec<S>,
    entity_name: String,
    config: TypeScriptConfig,
    idl: Option<serde_json::Value>, // IDL for enum type generation
    handlers_json: Option<serde_json::Value>, // Raw handlers for event interface generation
    views: Vec<ViewDef>,            // View definitions for derived views
}

impl<S> TypeScriptCompiler<S> {
    pub fn new(spec: TypedStreamSpec<S>, entity_name: String) -> Self {
        Self {
            spec,
            entity_name,
            config: TypeScriptConfig::default(),
            idl: None,
            handlers_json: None,
            views: Vec::new(),
        }
    }

    pub fn with_config(mut self, config: TypeScriptConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_idl(mut self, idl: Option<serde_json::Value>) -> Self {
        self.idl = idl;
        self
    }

    pub fn with_handlers_json(mut self, handlers: Option<serde_json::Value>) -> Self {
        self.handlers_json = handlers;
        self
    }

    pub fn with_views(mut self, views: Vec<ViewDef>) -> Self {
        self.views = views;
        self
    }

    pub fn compile(&self) -> TypeScriptOutput {
        let imports = self.generate_imports();
        let interfaces = self.generate_interfaces();
        let stack_definition = self.generate_stack_definition();

        TypeScriptOutput {
            imports,
            interfaces,
            stack_definition,
        }
    }

    fn generate_imports(&self) -> String {
        // No imports needed - generated file is self-contained
        String::new()
    }

    fn generate_view_helpers(&self) -> String {
        r#"// ============================================================================
// View Definition Types (framework-agnostic)
// ============================================================================

/** View definition with embedded entity type */
export interface ViewDef<T, TMode extends 'state' | 'list'> {
  readonly mode: TMode;
  readonly view: string;
  /** Phantom field for type inference - not present at runtime */
  readonly _entity?: T;
}

/** Helper to create typed state view definitions */
function stateView<T>(view: string): ViewDef<T, 'state'> {
  return { mode: 'state', view } as const;
}

/** Helper to create typed list view definitions */
function listView<T>(view: string): ViewDef<T, 'list'> {
  return { mode: 'list', view } as const;
}

/** Helper to create typed derived view definitions */
function derivedView<T>(view: string, output: 'single' | 'collection'): ViewDef<T, 'state' | 'list'> {
  return { mode: output === 'single' ? 'state' : 'list', view } as const;
}"#
        .to_string()
    }

    fn generate_interfaces(&self) -> String {
        let mut interfaces = Vec::new();
        let mut processed_types = HashSet::new();
        let mut all_sections: BTreeMap<String, Vec<TypeScriptField>> = BTreeMap::new();

        // Collect all interface sections from all handlers
        for handler in &self.spec.handlers {
            let interface_sections = self.extract_interface_sections_from_handler(handler);

            for (section_name, mut fields) in interface_sections {
                all_sections
                    .entry(section_name)
                    .or_default()
                    .append(&mut fields);
            }
        }

        // Add unmapped fields from spec.sections ONCE (not per handler)
        // These are fields without #[map] or #[event] attributes
        self.add_unmapped_fields(&mut all_sections);

        // Deduplicate fields within each section and generate interfaces
        // Skip root section - its fields will be flattened into main entity interface
        for (section_name, fields) in all_sections {
            if !is_root_section(&section_name) && processed_types.insert(section_name.clone()) {
                let deduplicated_fields = self.deduplicate_fields(fields);
                let interface =
                    self.generate_interface_from_fields(&section_name, &deduplicated_fields);
                interfaces.push(interface);
            }
        }

        // Generate main entity interface
        let main_interface = self.generate_main_entity_interface();
        interfaces.push(main_interface);

        // Generate nested interfaces for resolved types (instructions, accounts, etc.)
        let nested_interfaces = self.generate_nested_interfaces();
        interfaces.extend(nested_interfaces);

        // Generate EventWrapper interface if there are any event types
        if self.has_event_types() {
            interfaces.push(self.generate_event_wrapper_interface());
        }

        interfaces.join("\n\n")
    }

    fn deduplicate_fields(&self, mut fields: Vec<TypeScriptField>) -> Vec<TypeScriptField> {
        let mut seen = HashSet::new();
        let mut unique_fields = Vec::new();

        // Sort fields by name for consistent output
        fields.sort_by(|a, b| a.name.cmp(&b.name));

        for field in fields {
            if seen.insert(field.name.clone()) {
                unique_fields.push(field);
            }
        }

        unique_fields
    }

    fn extract_interface_sections_from_handler(
        &self,
        handler: &TypedHandlerSpec<S>,
    ) -> BTreeMap<String, Vec<TypeScriptField>> {
        let mut sections: BTreeMap<String, Vec<TypeScriptField>> = BTreeMap::new();

        for mapping in &handler.mappings {
            let parts: Vec<&str> = mapping.target_path.split('.').collect();

            if parts.len() > 1 {
                let section_name = parts[0];
                let field_name = parts[1];

                let ts_field = TypeScriptField {
                    name: field_name.to_string(),
                    ts_type: self.mapping_to_typescript_type(mapping),
                    optional: self.is_field_optional(mapping),
                    description: None,
                };

                sections
                    .entry(section_name.to_string())
                    .or_default()
                    .push(ts_field);
            } else {
                let ts_field = TypeScriptField {
                    name: mapping.target_path.clone(),
                    ts_type: self.mapping_to_typescript_type(mapping),
                    optional: self.is_field_optional(mapping),
                    description: None,
                };

                sections
                    .entry("Root".to_string())
                    .or_default()
                    .push(ts_field);
            }
        }

        sections
    }

    fn add_unmapped_fields(&self, sections: &mut BTreeMap<String, Vec<TypeScriptField>>) {
        // NEW: Enhanced approach using AST type information if available
        if !self.spec.sections.is_empty() {
            // Use type information from the enhanced AST
            for section in &self.spec.sections {
                let section_fields = sections.entry(section.name.clone()).or_default();

                for field_info in &section.fields {
                    // Check if field is already mapped
                    let already_exists = section_fields.iter().any(|f| {
                        f.name == field_info.field_name
                            || f.name == to_camel_case(&field_info.field_name)
                    });

                    if !already_exists {
                        section_fields.push(TypeScriptField {
                            name: field_info.field_name.clone(),
                            ts_type: self.field_type_info_to_typescript(field_info),
                            optional: field_info.is_optional,
                            description: None,
                        });
                    }
                }
            }
        } else {
            // FALLBACK: Use field mappings from spec if sections aren't available yet
            for (field_path, field_type_info) in &self.spec.field_mappings {
                let parts: Vec<&str> = field_path.split('.').collect();
                if parts.len() > 1 {
                    let section_name = parts[0];
                    let field_name = parts[1];

                    let section_fields = sections.entry(section_name.to_string()).or_default();

                    let already_exists = section_fields
                        .iter()
                        .any(|f| f.name == field_name || f.name == to_camel_case(field_name));

                    if !already_exists {
                        section_fields.push(TypeScriptField {
                            name: field_name.to_string(),
                            ts_type: self.base_type_to_typescript(
                                &field_type_info.base_type,
                                field_type_info.is_array,
                            ),
                            optional: field_type_info.is_optional,
                            description: None,
                        });
                    }
                }
            }
        }
    }

    fn generate_interface_from_fields(&self, name: &str, fields: &[TypeScriptField]) -> String {
        // Generate more descriptive interface names
        let interface_name = if name == "Root" {
            format!(
                "{}{}",
                self.config.interface_prefix,
                to_pascal_case(&self.entity_name)
            )
        } else {
            // Create compound names like GameEvents, GameStatus, etc.
            // Extract the base name (e.g., "Game" from "TestGame" or "SettlementGame")
            let base_name = if self.entity_name.contains("Game") {
                "Game"
            } else {
                &self.entity_name
            };
            format!(
                "{}{}{}",
                self.config.interface_prefix,
                base_name,
                to_pascal_case(name)
            )
        };

        // All fields are optional (?) since we receive patches - field may not yet exist
        // For spec-optional fields, we use `T | null` to distinguish "explicitly null" from "not received"
        let field_definitions: Vec<String> = fields
            .iter()
            .map(|field| {
                let field_name = to_camel_case(&field.name);
                let ts_type = if field.optional {
                    // Spec-optional: can be explicitly null
                    format!("{} | null", field.ts_type)
                } else {
                    field.ts_type.clone()
                };
                format!("  {}?: {};", field_name, ts_type)
            })
            .collect();

        format!(
            "export interface {} {{\n{}\n}}",
            interface_name,
            field_definitions.join("\n")
        )
    }

    fn generate_main_entity_interface(&self) -> String {
        let entity_name = to_pascal_case(&self.entity_name);

        // Extract all top-level sections from the handlers
        let mut sections = BTreeMap::new();

        for handler in &self.spec.handlers {
            for mapping in &handler.mappings {
                let parts: Vec<&str> = mapping.target_path.split('.').collect();
                if parts.len() > 1 {
                    sections.insert(parts[0], true);
                }
            }
        }

        if !self.spec.sections.is_empty() {
            for section in &self.spec.sections {
                sections.insert(&section.name, true);
            }
        } else {
            for mapping in &self.spec.handlers {
                for field_mapping in &mapping.mappings {
                    let parts: Vec<&str> = field_mapping.target_path.split('.').collect();
                    if parts.len() > 1 {
                        sections.insert(parts[0], true);
                    }
                }
            }
        }

        let mut fields = Vec::new();

        // Add non-root sections as nested interface references
        // All fields are optional since we receive patches
        for section in sections.keys() {
            if !is_root_section(section) {
                let base_name = if self.entity_name.contains("Game") {
                    "Game"
                } else {
                    &self.entity_name
                };
                let section_interface_name = format!("{}{}", base_name, to_pascal_case(section));
                fields.push(format!(
                    "  {}?: {};",
                    to_camel_case(section),
                    section_interface_name
                ));
            }
        }

        // Flatten root section fields directly into main interface
        // All fields are optional (?) since we receive patches
        for section in &self.spec.sections {
            if is_root_section(&section.name) {
                for field in &section.fields {
                    let field_name = to_camel_case(&field.field_name);
                    let base_ts_type = self.field_type_info_to_typescript(field);
                    let ts_type = if field.is_optional {
                        format!("{} | null", base_ts_type)
                    } else {
                        base_ts_type
                    };
                    fields.push(format!("  {}?: {};", field_name, ts_type));
                }
            }
        }

        if fields.is_empty() {
            fields.push("  // Generated interface - extend as needed".to_string());
        }

        format!(
            "export interface {} {{\n{}\n}}",
            entity_name,
            fields.join("\n")
        )
    }

    fn generate_stack_definition(&self) -> String {
        let stack_name = to_kebab_case(&self.entity_name);
        let entity_pascal = to_pascal_case(&self.entity_name);
        let export_name = format!(
            "{}_{}",
            self.entity_name.to_uppercase(),
            self.config.export_const_name
        );

        let view_helpers = self.generate_view_helpers();
        let derived_views = self.generate_derived_view_entries();

        format!(
            r#"{}

// ============================================================================
// Stack Definition
// ============================================================================

/** Stack definition for {} */
export const {} = {{
  name: '{}',
  views: {{
    {}: {{
      state: stateView<{}>('{}/state'),
      list: listView<{}>('{}/list'),{}
    }},
  }},
}} as const;

/** Type alias for the stack */
export type {}Stack = typeof {};

/** Default export for convenience */
export default {};"#,
            view_helpers,
            entity_pascal,
            export_name,
            stack_name,
            to_camel_case(&self.entity_name),
            entity_pascal,
            self.entity_name,
            entity_pascal,
            self.entity_name,
            derived_views,
            entity_pascal,
            export_name,
            export_name
        )
    }

    fn generate_derived_view_entries(&self) -> String {
        let derived_views: Vec<&ViewDef> = self
            .views
            .iter()
            .filter(|v| {
                !v.id.ends_with("/state")
                    && !v.id.ends_with("/list")
                    && v.id.starts_with(&self.entity_name)
            })
            .collect();

        if derived_views.is_empty() {
            return String::new();
        }

        let entity_pascal = to_pascal_case(&self.entity_name);
        let mut entries = Vec::new();

        for view in derived_views {
            let view_name = view.id.split('/').nth(1).unwrap_or("unknown");
            let output_mode = match view.output {
                ViewOutput::Single => "single",
                ViewOutput::Collection => "collection",
                ViewOutput::Keyed { .. } => "single",
            };

            entries.push(format!(
                "\n      {}: derivedView<{}>('{}', '{}'),",
                to_camel_case(view_name),
                entity_pascal,
                view.id,
                output_mode
            ));
        }

        entries.join("")
    }

    fn mapping_to_typescript_type(&self, mapping: &TypedFieldMapping<S>) -> String {
        // First, try to resolve from AST field mappings
        if let Some(field_info) = self.spec.field_mappings.get(&mapping.target_path) {
            let ts_type = self.field_type_info_to_typescript(field_info);

            // If it's an Append strategy, wrap in array
            if matches!(mapping.population, PopulationStrategy::Append) {
                return if ts_type.ends_with("[]") {
                    ts_type
                } else {
                    format!("{}[]", ts_type)
                };
            }

            return ts_type;
        }

        // Fallback to legacy inference
        match &mapping.population {
            PopulationStrategy::Append => {
                // For arrays, try to infer the element type
                match &mapping.source {
                    MappingSource::AsEvent { .. } => "any[]".to_string(),
                    _ => "any[]".to_string(),
                }
            }
            _ => {
                // Infer type from source and field name
                let base_type = match &mapping.source {
                    MappingSource::FromSource { .. } => {
                        self.infer_type_from_field_name(&mapping.target_path)
                    }
                    MappingSource::Constant(value) => value_to_typescript_type(value),
                    MappingSource::AsEvent { .. } => "any".to_string(),
                    _ => "any".to_string(),
                };

                // Apply transformations to type
                if let Some(transform) = &mapping.transform {
                    match transform {
                        Transformation::HexEncode | Transformation::HexDecode => {
                            "string".to_string()
                        }
                        Transformation::Base58Encode | Transformation::Base58Decode => {
                            "string".to_string()
                        }
                        Transformation::ToString => "string".to_string(),
                        Transformation::ToNumber => "number".to_string(),
                    }
                } else {
                    base_type
                }
            }
        }
    }

    /// Convert FieldTypeInfo from AST to TypeScript type string
    fn field_type_info_to_typescript(&self, field_info: &FieldTypeInfo) -> String {
        // If we have resolved type information (complex types from IDL), use it
        if let Some(resolved) = &field_info.resolved_type {
            let interface_name = self.resolved_type_to_interface_name(resolved);

            // Wrap in EventWrapper if it's an event type
            let base_type = if resolved.is_event || (resolved.is_instruction && field_info.is_array)
            {
                format!("EventWrapper<{}>", interface_name)
            } else {
                interface_name
            };

            // Handle optional and array
            let with_array = if field_info.is_array {
                format!("{}[]", base_type)
            } else {
                base_type
            };

            return with_array;
        }

        // Check if this is an event field (has BaseType::Any or BaseType::Array with Value inner type)
        // We can detect event fields by looking for them in handlers with AsEvent mappings
        if field_info.base_type == BaseType::Any
            || (field_info.base_type == BaseType::Array
                && field_info.inner_type.as_deref() == Some("Value"))
        {
            if let Some(event_type) = self.find_event_interface_for_field(&field_info.field_name) {
                return if field_info.is_array {
                    format!("{}[]", event_type)
                } else if field_info.is_optional {
                    format!("{} | null", event_type)
                } else {
                    event_type
                };
            }
        }

        // Use base type mapping
        self.base_type_to_typescript(&field_info.base_type, field_info.is_array)
    }

    /// Find the generated event interface name for a given field
    fn find_event_interface_for_field(&self, field_name: &str) -> Option<String> {
        // Use the raw JSON handlers if available
        let handlers = self.handlers_json.as_ref()?.as_array()?;

        // Look through handlers to find event mappings for this field
        for handler in handlers {
            if let Some(mappings) = handler.get("mappings").and_then(|m| m.as_array()) {
                for mapping in mappings {
                    if let Some(target_path) = mapping.get("target_path").and_then(|t| t.as_str()) {
                        // Check if this mapping targets our field (e.g., "events.created")
                        let target_parts: Vec<&str> = target_path.split('.').collect();
                        if let Some(target_field) = target_parts.last() {
                            if *target_field == field_name {
                                // Check if this is an event mapping
                                if let Some(source) = mapping.get("source") {
                                    if self.extract_event_data(source).is_some() {
                                        // Generate the interface name (e.g., "created" -> "CreatedEvent")
                                        return Some(format!(
                                            "{}Event",
                                            to_pascal_case(field_name)
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Generate TypeScript interface name from resolved type
    fn resolved_type_to_interface_name(&self, resolved: &ResolvedStructType) -> String {
        to_pascal_case(&resolved.type_name)
    }

    /// Generate nested interfaces for all resolved types in the AST
    fn generate_nested_interfaces(&self) -> Vec<String> {
        let mut interfaces = Vec::new();
        let mut generated_types = HashSet::new();

        // Collect all resolved types from all sections
        for section in &self.spec.sections {
            for field_info in &section.fields {
                if let Some(resolved) = &field_info.resolved_type {
                    let type_name = resolved.type_name.clone();

                    // Only generate each type once
                    if generated_types.insert(type_name) {
                        let interface = self.generate_interface_for_resolved_type(resolved);
                        interfaces.push(interface);
                    }
                }
            }
        }

        // Generate event interfaces from instruction handlers
        interfaces.extend(self.generate_event_interfaces(&mut generated_types));

        // Also generate all enum types from the IDL (even if not directly referenced)
        if let Some(idl_value) = &self.idl {
            if let Some(types_array) = idl_value.get("types").and_then(|v| v.as_array()) {
                for type_def in types_array {
                    if let (Some(type_name), Some(type_obj)) = (
                        type_def.get("name").and_then(|v| v.as_str()),
                        type_def.get("type").and_then(|v| v.as_object()),
                    ) {
                        if type_obj.get("kind").and_then(|v| v.as_str()) == Some("enum") {
                            // Only generate if not already generated
                            if generated_types.insert(type_name.to_string()) {
                                if let Some(variants) =
                                    type_obj.get("variants").and_then(|v| v.as_array())
                                {
                                    let variant_names: Vec<String> = variants
                                        .iter()
                                        .filter_map(|v| {
                                            v.get("name")
                                                .and_then(|n| n.as_str())
                                                .map(|s| s.to_string())
                                        })
                                        .collect();

                                    if !variant_names.is_empty() {
                                        let interface_name = to_pascal_case(type_name);
                                        let variant_strings: Vec<String> = variant_names
                                            .iter()
                                            .map(|v| format!("\"{}\"", to_pascal_case(v)))
                                            .collect();

                                        let enum_type = format!(
                                            "export type {} = {};",
                                            interface_name,
                                            variant_strings.join(" | ")
                                        );
                                        interfaces.push(enum_type);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        interfaces
    }

    /// Generate TypeScript interfaces for event types from instruction handlers
    fn generate_event_interfaces(&self, generated_types: &mut HashSet<String>) -> Vec<String> {
        let mut interfaces = Vec::new();

        // Use the raw JSON handlers if available
        let handlers = match &self.handlers_json {
            Some(h) => h.as_array(),
            None => return interfaces,
        };

        let handlers_array = match handlers {
            Some(arr) => arr,
            None => return interfaces,
        };

        // Look through handlers to find instruction-based event mappings
        for handler in handlers_array {
            // Check if this handler has event mappings
            if let Some(mappings) = handler.get("mappings").and_then(|m| m.as_array()) {
                for mapping in mappings {
                    if let Some(target_path) = mapping.get("target_path").and_then(|t| t.as_str()) {
                        // Check if the target is an event field (contains ".events." or starts with "events.")
                        if target_path.contains(".events.") || target_path.starts_with("events.") {
                            // Check if the source is AsEvent
                            if let Some(source) = mapping.get("source") {
                                if let Some(event_data) = self.extract_event_data(source) {
                                    // Extract instruction name from handler source
                                    if let Some(handler_source) = handler.get("source") {
                                        if let Some(instruction_name) =
                                            self.extract_instruction_name(handler_source)
                                        {
                                            // Generate interface name from target path (e.g., "events.created" -> "CreatedEvent")
                                            let event_field_name =
                                                target_path.split('.').next_back().unwrap_or("");
                                            let interface_name = format!(
                                                "{}Event",
                                                to_pascal_case(event_field_name)
                                            );

                                            // Only generate once
                                            if generated_types.insert(interface_name.clone()) {
                                                if let Some(interface) = self
                                                    .generate_event_interface_from_idl(
                                                        &interface_name,
                                                        &instruction_name,
                                                        &event_data,
                                                    )
                                                {
                                                    interfaces.push(interface);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        interfaces
    }

    /// Extract event field data from a mapping source
    fn extract_event_data(
        &self,
        source: &serde_json::Value,
    ) -> Option<Vec<(String, Option<String>)>> {
        if let Some(as_event) = source.get("AsEvent") {
            if let Some(fields) = as_event.get("fields").and_then(|f| f.as_array()) {
                let mut event_fields = Vec::new();
                for field in fields {
                    if let Some(from_source) = field.get("FromSource") {
                        if let Some(path) = from_source
                            .get("path")
                            .and_then(|p| p.get("segments"))
                            .and_then(|s| s.as_array())
                        {
                            // Get the last segment as the field name (e.g., ["data", "game_id"] -> "game_id")
                            if let Some(field_name) = path.last().and_then(|v| v.as_str()) {
                                let transform = from_source
                                    .get("transform")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string());
                                event_fields.push((field_name.to_string(), transform));
                            }
                        }
                    }
                }
                return Some(event_fields);
            }
        }
        None
    }

    /// Extract instruction name from handler source
    fn extract_instruction_name(&self, source: &serde_json::Value) -> Option<String> {
        if let Some(source_obj) = source.get("Source") {
            if let Some(type_name) = source_obj.get("type_name").and_then(|t| t.as_str()) {
                // Convert "CreateGameIxState" -> "create_game"
                if let Some(instruction_part) = type_name.strip_suffix("IxState") {
                    return Some(to_snake_case(instruction_part));
                }
            }
        }
        None
    }

    /// Generate a TypeScript interface for an event from IDL instruction data
    fn generate_event_interface_from_idl(
        &self,
        interface_name: &str,
        instruction_name: &str,
        captured_fields: &[(String, Option<String>)],
    ) -> Option<String> {
        // If no fields are captured, generate an empty interface
        if captured_fields.is_empty() {
            return Some(format!("export interface {} {{}}", interface_name));
        }

        let idl_value = self.idl.as_ref()?;
        let instructions = idl_value.get("instructions")?.as_array()?;

        // Find the instruction in the IDL
        for instruction in instructions {
            if let Some(name) = instruction.get("name").and_then(|n| n.as_str()) {
                if name == instruction_name {
                    // Get the args
                    if let Some(args) = instruction.get("args").and_then(|a| a.as_array()) {
                        let mut fields = Vec::new();

                        // Only include captured fields
                        for (field_name, transform) in captured_fields {
                            // Find this arg in the instruction
                            for arg in args {
                                if let Some(arg_name) = arg.get("name").and_then(|n| n.as_str()) {
                                    if arg_name == field_name {
                                        if let Some(arg_type) = arg.get("type") {
                                            let ts_type = self.idl_type_to_typescript(
                                                arg_type,
                                                transform.as_deref(),
                                            );
                                            let camel_name = to_camel_case(field_name);
                                            fields.push(format!("  {}: {};", camel_name, ts_type));
                                        }
                                        break;
                                    }
                                }
                            }
                        }

                        if !fields.is_empty() {
                            return Some(format!(
                                "export interface {} {{\n{}\n}}",
                                interface_name,
                                fields.join("\n")
                            ));
                        }
                    }
                }
            }
        }

        None
    }

    /// Convert an IDL type (from JSON) to TypeScript, considering transforms
    fn idl_type_to_typescript(
        &self,
        idl_type: &serde_json::Value,
        transform: Option<&str>,
    ) -> String {
        #![allow(clippy::only_used_in_recursion)]
        // If there's a HexEncode transform, the result is always a string
        if transform == Some("HexEncode") {
            return "string".to_string();
        }

        // Handle different IDL type formats
        if let Some(type_str) = idl_type.as_str() {
            return match type_str {
                "u8" | "u16" | "u32" | "u64" | "u128" | "i8" | "i16" | "i32" | "i64" | "i128" => {
                    "number".to_string()
                }
                "f32" | "f64" => "number".to_string(),
                "bool" => "boolean".to_string(),
                "string" => "string".to_string(),
                "pubkey" | "publicKey" => "string".to_string(),
                "bytes" => "string".to_string(),
                _ => "any".to_string(),
            };
        }

        // Handle complex types (option, vec, etc.)
        if let Some(type_obj) = idl_type.as_object() {
            if let Some(option_type) = type_obj.get("option") {
                let inner = self.idl_type_to_typescript(option_type, None);
                return format!("{} | null", inner);
            }
            if let Some(vec_type) = type_obj.get("vec") {
                let inner = self.idl_type_to_typescript(vec_type, None);
                return format!("{}[]", inner);
            }
        }

        "any".to_string()
    }

    /// Generate a TypeScript interface from a resolved struct type
    fn generate_interface_for_resolved_type(&self, resolved: &ResolvedStructType) -> String {
        let interface_name = to_pascal_case(&resolved.type_name);

        // Handle enums as TypeScript union types
        if resolved.is_enum {
            let variants: Vec<String> = resolved
                .enum_variants
                .iter()
                .map(|v| format!("\"{}\"", to_pascal_case(v)))
                .collect();

            return format!("export type {} = {};", interface_name, variants.join(" | "));
        }

        // Handle structs as interfaces
        // All fields are optional since we receive patches
        let fields: Vec<String> = resolved
            .fields
            .iter()
            .map(|field| {
                let field_name = to_camel_case(&field.field_name);
                let base_ts_type = self.resolved_field_to_typescript(field);
                let ts_type = if field.is_optional {
                    format!("{} | null", base_ts_type)
                } else {
                    base_ts_type
                };
                format!("  {}?: {};", field_name, ts_type)
            })
            .collect();

        format!(
            "export interface {} {{\n{}\n}}",
            interface_name,
            fields.join("\n")
        )
    }

    /// Convert a resolved field to TypeScript type
    fn resolved_field_to_typescript(&self, field: &ResolvedField) -> String {
        let base_ts = self.base_type_to_typescript(&field.base_type, false);

        if field.is_array {
            format!("{}[]", base_ts)
        } else {
            base_ts
        }
    }

    /// Check if the spec has any event types
    fn has_event_types(&self) -> bool {
        for section in &self.spec.sections {
            for field_info in &section.fields {
                if let Some(resolved) = &field_info.resolved_type {
                    if resolved.is_event || (resolved.is_instruction && field_info.is_array) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Generate the EventWrapper interface
    fn generate_event_wrapper_interface(&self) -> String {
        r#"/**
 * Wrapper for event data that includes context metadata.
 * Events are automatically wrapped in this structure at runtime.
 */
export interface EventWrapper<T> {
  /** Unix timestamp when the event was processed */
  timestamp: number;
  /** The event-specific data */
  data: T;
  /** Optional blockchain slot number */
  slot?: number;
  /** Optional transaction signature */
  signature?: string;
}"#
        .to_string()
    }

    fn infer_type_from_field_name(&self, field_name: &str) -> String {
        let lower_name = field_name.to_lowercase();

        // Special case for event fields - these are typically Option<Value> and should be 'any'
        if lower_name.contains("events.") {
            // For fields in the events section, default to 'any' since they're typically Option<Value>
            return "any".to_string();
        }

        // Common patterns for type inference
        if lower_name.contains("id")
            || lower_name.contains("count")
            || lower_name.contains("number")
            || lower_name.contains("timestamp")
            || lower_name.contains("time")
            || lower_name.contains("at")
            || lower_name.contains("volume")
            || lower_name.contains("amount")
            || lower_name.contains("ev")
            || lower_name.contains("fee")
            || lower_name.contains("payout")
            || lower_name.contains("distributed")
            || lower_name.contains("claimable")
            || lower_name.contains("total")
            || lower_name.contains("rate")
            || lower_name.contains("ratio")
            || lower_name.contains("current")
            || lower_name.contains("state")
        {
            "number".to_string()
        } else if lower_name.contains("status")
            || lower_name.contains("hash")
            || lower_name.contains("address")
            || lower_name.contains("key")
        {
            "string".to_string()
        } else {
            "any".to_string()
        }
    }

    fn is_field_optional(&self, mapping: &TypedFieldMapping<S>) -> bool {
        // Most fields should be optional by default since we're dealing with Option<T> types
        match &mapping.source {
            // Constants are typically non-optional
            MappingSource::Constant(_) => false,
            // Events are typically optional (Option<Value>)
            MappingSource::AsEvent { .. } => true,
            // For source fields, default to optional since most Rust fields are Option<T>
            MappingSource::FromSource { .. } => true,
            // Other cases default to optional
            _ => true,
        }
    }

    /// Convert language-agnostic base types to TypeScript types
    fn base_type_to_typescript(&self, base_type: &BaseType, is_array: bool) -> String {
        let base_ts_type = match base_type {
            BaseType::Integer => "number",
            BaseType::Float => "number",
            BaseType::String => "string",
            BaseType::Boolean => "boolean",
            BaseType::Timestamp => "number", // Unix timestamps as numbers
            BaseType::Binary => "string",    // Base64 encoded strings
            BaseType::Pubkey => "string",    // Solana public keys as Base58 strings
            BaseType::Array => "any[]",      // Default array type
            BaseType::Object => "Record<string, any>", // Generic object
            BaseType::Any => "any",
        };

        if is_array && !matches!(base_type, BaseType::Array) {
            format!("{}[]", base_ts_type)
        } else {
            base_ts_type.to_string()
        }
    }
}

/// Represents a TypeScript field in an interface
#[derive(Debug, Clone)]
struct TypeScriptField {
    name: String,
    ts_type: String,
    optional: bool,
    #[allow(dead_code)]
    description: Option<String>,
}

/// Convert serde_json::Value to TypeScript type string
fn value_to_typescript_type(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Array(_) => "any[]".to_string(),
        serde_json::Value::Object(_) => "Record<string, any>".to_string(),
        serde_json::Value::Null => "null".to_string(),
    }
}

/// Convert snake_case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split(['_', '-', '.'])
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Convert snake_case to camelCase
fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    let mut chars = pascal.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

/// Convert PascalCase/camelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for ch in s.chars() {
        if ch.is_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }

    result
}

/// Check if a section name is the root section (case-insensitive)
fn is_root_section(name: &str) -> bool {
    name.eq_ignore_ascii_case("root")
}

/// Convert PascalCase/camelCase to kebab-case
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();

    for ch in s.chars() {
        if ch.is_uppercase() && !result.is_empty() {
            result.push('-');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }

    result
}

/// CLI-friendly function to generate TypeScript from a spec function
/// This will be used by the CLI tool to generate TypeScript from discovered specs
pub fn generate_typescript_from_spec_fn<F, S>(
    spec_fn: F,
    entity_name: String,
    config: Option<TypeScriptConfig>,
) -> Result<TypeScriptOutput, String>
where
    F: Fn() -> TypedStreamSpec<S>,
{
    let spec = spec_fn();
    let compiler =
        TypeScriptCompiler::new(spec, entity_name).with_config(config.unwrap_or_default());

    Ok(compiler.compile())
}

/// Write TypeScript output to a file
pub fn write_typescript_to_file(
    output: &TypeScriptOutput,
    path: &std::path::Path,
) -> Result<(), std::io::Error> {
    std::fs::write(path, output.full_file())
}

/// Generate TypeScript from a SerializableStreamSpec (for CLI use)
/// This allows the CLI to compile TypeScript without needing the typed spec
pub fn compile_serializable_spec(
    spec: SerializableStreamSpec,
    entity_name: String,
    config: Option<TypeScriptConfig>,
) -> Result<TypeScriptOutput, String> {
    let idl = spec
        .idl
        .as_ref()
        .and_then(|idl_snapshot| serde_json::to_value(idl_snapshot).ok());

    let handlers = serde_json::to_value(&spec.handlers).ok();
    let views = spec.views.clone();

    let typed_spec: TypedStreamSpec<()> = TypedStreamSpec::from_serializable(spec);

    let compiler = TypeScriptCompiler::new(typed_spec, entity_name)
        .with_idl(idl)
        .with_handlers_json(handlers)
        .with_views(views)
        .with_config(config.unwrap_or_default());

    Ok(compiler.compile())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_conversions() {
        assert_eq!(to_pascal_case("settlement_game"), "SettlementGame");
        assert_eq!(to_camel_case("settlement_game"), "settlementGame");
        assert_eq!(to_kebab_case("SettlementGame"), "settlement-game");
    }

    #[test]
    fn test_value_to_typescript_type() {
        assert_eq!(value_to_typescript_type(&serde_json::json!(42)), "number");
        assert_eq!(
            value_to_typescript_type(&serde_json::json!("hello")),
            "string"
        );
        assert_eq!(
            value_to_typescript_type(&serde_json::json!(true)),
            "boolean"
        );
        assert_eq!(value_to_typescript_type(&serde_json::json!([])), "any[]");
    }

    #[test]
    fn test_derived_view_codegen() {
        let spec = SerializableStreamSpec {
            state_name: "OreRound".to_string(),
            program_id: None,
            idl: None,
            identity: IdentitySpec {
                primary_keys: vec!["id".to_string()],
                lookup_indexes: vec![],
            },
            handlers: vec![],
            sections: vec![],
            field_mappings: BTreeMap::new(),
            resolver_hooks: vec![],
            instruction_hooks: vec![],
            computed_fields: vec![],
            computed_field_specs: vec![],
            content_hash: None,
            views: vec![
                ViewDef {
                    id: "OreRound/latest".to_string(),
                    source: ViewSource::Entity {
                        name: "OreRound".to_string(),
                    },
                    pipeline: vec![ViewTransform::Last],
                    output: ViewOutput::Single,
                },
                ViewDef {
                    id: "OreRound/top10".to_string(),
                    source: ViewSource::Entity {
                        name: "OreRound".to_string(),
                    },
                    pipeline: vec![ViewTransform::Take { count: 10 }],
                    output: ViewOutput::Collection,
                },
            ],
        };

        let output =
            compile_serializable_spec(spec, "OreRound".to_string(), None).expect("should compile");

        let stack_def = &output.stack_definition;

        assert!(
            stack_def.contains("derivedView<OreRound>('OreRound/latest', 'single')"),
            "Expected 'latest' derived view with 'single' output, got:\n{}",
            stack_def
        );
        assert!(
            stack_def.contains("derivedView<OreRound>('OreRound/top10', 'collection')"),
            "Expected 'top10' derived view with 'collection' output, got:\n{}",
            stack_def
        );
        assert!(
            stack_def.contains("latest:"),
            "Expected 'latest' key, got:\n{}",
            stack_def
        );
        assert!(
            stack_def.contains("top10:"),
            "Expected 'top10' key, got:\n{}",
            stack_def
        );
        assert!(
            stack_def
                .contains("function derivedView<T>(view: string, output: 'single' | 'collection')"),
            "Expected derivedView helper function, got:\n{}",
            stack_def
        );
    }
}
