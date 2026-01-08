use crate::ast::*;
use std::collections::{BTreeMap, HashMap, HashSet};

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
            package_name: "@hyperstack/sdk".to_string(),
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
}

impl<S> TypeScriptCompiler<S> {
    pub fn new(spec: TypedStreamSpec<S>, entity_name: String) -> Self {
        Self {
            spec,
            entity_name,
            config: TypeScriptConfig::default(),
        }
    }

    pub fn with_config(mut self, config: TypeScriptConfig) -> Self {
        self.config = config;
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
        format!(
            "import {{ defineStack, createStateView, createListView }} from '{}';",
            self.config.package_name
        )
    }

    fn generate_interfaces(&self) -> String {
        let mut interfaces = Vec::new();
        let mut processed_types = HashSet::new();
        let mut all_sections: BTreeMap<String, Vec<TypeScriptField>> = BTreeMap::new();

        // Collect all interface sections from all handlers
        for handler in &self.spec.handlers {
            let interface_sections = self.extract_interface_sections(handler);

            for (section_name, mut fields) in interface_sections {
                all_sections
                    .entry(section_name)
                    .or_insert_with(Vec::new)
                    .append(&mut fields);
            }
        }

        // Deduplicate fields within each section and generate interfaces
        for (section_name, fields) in all_sections {
            if processed_types.insert(section_name.clone()) {
                let deduplicated_fields = self.deduplicate_fields(fields);
                let interface =
                    self.generate_interface_from_fields(&section_name, &deduplicated_fields);
                interfaces.push(interface);
            }
        }

        // Generate main entity interface
        let main_interface = self.generate_main_entity_interface();
        interfaces.push(main_interface);

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

    fn extract_interface_sections(
        &self,
        handler: &TypedHandlerSpec<S>,
    ) -> BTreeMap<String, Vec<TypeScriptField>> {
        let mut sections: BTreeMap<String, Vec<TypeScriptField>> = BTreeMap::new();

        for mapping in &handler.mappings {
            let parts: Vec<&str> = mapping.target_path.split('.').collect();

            if parts.len() > 1 {
                // Nested field (e.g., "status.current")
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
                    .or_insert_with(Vec::new)
                    .push(ts_field);
            } else {
                // Top-level field
                let ts_field = TypeScriptField {
                    name: mapping.target_path.clone(),
                    ts_type: self.mapping_to_typescript_type(mapping),
                    optional: self.is_field_optional(mapping),
                    description: None,
                };

                sections
                    .entry("Root".to_string())
                    .or_insert_with(Vec::new)
                    .push(ts_field);
            }
        }

        // Add any unmapped fields that exist in the original spec
        // These are fields without #[map] or #[event] attributes
        self.add_unmapped_fields(&mut sections);

        sections
    }

    fn add_unmapped_fields(&self, sections: &mut BTreeMap<String, Vec<TypeScriptField>>) {
        // NEW: Enhanced approach using AST type information if available
        if !self.spec.sections.is_empty() {
            // Use type information from the enhanced AST
            for section in &self.spec.sections {
                let section_fields = sections.entry(section.name.clone()).or_insert_with(Vec::new);
                
                for field_info in &section.fields {
                    // Check if field is already mapped
                    let already_exists = section_fields.iter().any(|f| 
                        f.name == field_info.field_name || 
                        f.name == to_camel_case(&field_info.field_name)
                    );
                    
                    if !already_exists {
                        section_fields.push(TypeScriptField {
                            name: field_info.field_name.clone(),
                            ts_type: self.base_type_to_typescript(&field_info.base_type, field_info.is_array),
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
                    
                    let section_fields = sections.entry(section_name.to_string()).or_insert_with(Vec::new);
                    
                    let already_exists = section_fields.iter().any(|f| 
                        f.name == field_name || 
                        f.name == to_camel_case(field_name)
                    );
                    
                    if !already_exists {
                        section_fields.push(TypeScriptField {
                            name: field_name.to_string(),
                            ts_type: self.base_type_to_typescript(&field_type_info.base_type, field_type_info.is_array),
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

        let field_definitions: Vec<String> = fields
            .iter()
            .map(|field| {
                let optional_marker = if field.optional { "?" } else { "" };
                // Convert snake_case to camelCase for field names
                let field_name = to_camel_case(&field.name);
                format!("  {}{}: {};", field_name, optional_marker, field.ts_type)
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

        // NEW: Use actual section names from enhanced AST if available
        if !self.spec.sections.is_empty() {
            for section in &self.spec.sections {
                sections.insert(&section.name, true);
            }
        } else {
            // FALLBACK: Extract section names from field mappings  
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

        // Add each section as a field in the main interface (sorted by key)
        for section in sections.keys() {
            // Generate proper interface names like GameEvents, GameStatus, etc.
            let base_name = if self.entity_name.contains("Game") {
                "Game"
            } else {
                &self.entity_name
            };
            let section_interface_name = format!("{}{}", base_name, to_pascal_case(section));
            fields.push(format!(
                "  {}: {};",
                to_camel_case(section),
                section_interface_name
            ));
        }

        // If no sections found, create a basic structure
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

        let views = self.generate_view_definitions();
        let helpers = if self.config.generate_helpers {
            self.generate_helper_functions()
        } else {
            String::new()
        };

        let helpers_section = if helpers.is_empty() {
            String::new()
        } else {
            format!(",\n  helpers: {{\n{}\n  }}", helpers)
        };
        
        format!(
            r#"export const {} = defineStack({{
  name: '{}',
  views: {{
    {}: {{
      state: createStateView<{}>('{}State/state'),
      list: createListView<{}>('{}State/list')
    }}
  }}{}
}});"#,
            export_name,
            stack_name,
            to_camel_case(&self.entity_name),
            entity_pascal,
            self.entity_name,
            entity_pascal,
            self.entity_name,
            helpers_section
        )
    }

    fn generate_view_definitions(&self) -> String {
        // For now, generate basic state and list views
        // This can be enhanced to generate specific views based on the spec
        to_camel_case(&self.entity_name)
    }

    fn generate_helper_functions(&self) -> String {
        let mut helpers = Vec::new();

        // Generate helpers based on field types and transformations
        for handler in &self.spec.handlers {
            for mapping in &handler.mappings {
                if let Some(helper) = self.generate_helper_for_mapping(mapping) {
                    helpers.push(helper);
                }
            }
        }

        helpers.join(",\n")
    }

    fn generate_helper_for_mapping(&self, mapping: &TypedFieldMapping<S>) -> Option<String> {
        // Generate helpers based on transformations or field types
        if let Some(transform) = &mapping.transform {
            match transform {
                Transformation::HexEncode => {
                    let helper_name = format!(
                        "format{}",
                        to_pascal_case(&mapping.target_path.replace(".", ""))
                    );
                    Some(format!(
                        "    {}: (value: string) => value.startsWith('0x') ? value : `0x${{value}}`",
                        helper_name
                    ))
                }
                Transformation::HexDecode => {
                    let helper_name = format!(
                        "decode{}",
                        to_pascal_case(&mapping.target_path.replace(".", ""))
                    );
                    Some(format!(
                        "    {}: (value: string) => value.startsWith('0x') ? value.slice(2) : value",
                        helper_name
                    ))
                }
                _ => None,
            }
        } else {
            None
        }
    }

    fn mapping_to_typescript_type(&self, mapping: &TypedFieldMapping<S>) -> String {
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
        {
            "number".to_string()
        } else if lower_name.contains("timestamp")
            || lower_name.contains("time")
            || lower_name.contains("at")
        {
            "number".to_string()
        } else if lower_name.contains("volume")
            || lower_name.contains("amount")
            || lower_name.contains("ev")
            || lower_name.contains("fee")
            || lower_name.contains("payout")
            || lower_name.contains("distributed")
            || lower_name.contains("claimable")
            || lower_name.contains("total")
        {
            "number".to_string()
        } else if lower_name.contains("rate") || lower_name.contains("ratio") {
            "number".to_string()
        } else if lower_name.contains("current") || lower_name.contains("state") {
            "number".to_string()
        } else if lower_name.contains("status") {
            "string".to_string()
        } else if lower_name.contains("hash")
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
            BaseType::Binary => "string", // Base64 encoded strings
            BaseType::Array => "any[]", // Default array type
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
    description: Option<String>,
}

/// Convert serde_json::Value to TypeScript type string
fn value_to_typescript_type(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "number".to_string()
            } else {
                "number".to_string()
            }
        }
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Array(_) => "any[]".to_string(),
        serde_json::Value::Object(_) => "Record<string, any>".to_string(),
        serde_json::Value::Null => "null".to_string(),
    }
}

/// Convert snake_case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split(|c| c == '_' || c == '-' || c == '.')
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

/// Convert PascalCase/camelCase to kebab-case
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
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
    // Convert SerializableStreamSpec to TypedStreamSpec
    // We use () as the phantom type since it won't be used
    let typed_spec: TypedStreamSpec<()> = TypedStreamSpec::from_serializable(spec);
    
    let compiler = TypeScriptCompiler::new(typed_spec, entity_name)
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
}

