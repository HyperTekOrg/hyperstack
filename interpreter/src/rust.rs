use crate::ast::*;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct RustOutput {
    pub cargo_toml: String,
    pub lib_rs: String,
    pub types_rs: String,
    pub entity_rs: String,
}

impl RustOutput {
    pub fn full_lib(&self) -> String {
        format!(
            "{}\n\n// types.rs\n{}\n\n// entity.rs\n{}",
            self.lib_rs, self.types_rs, self.entity_rs
        )
    }
}

#[derive(Debug, Clone)]
pub struct RustConfig {
    pub crate_name: String,
    pub sdk_version: String,
}

impl Default for RustConfig {
    fn default() -> Self {
        Self {
            crate_name: "generated-stack".to_string(),
            sdk_version: "0.2".to_string(),
        }
    }
}

pub fn compile_serializable_spec(
    spec: SerializableStreamSpec,
    entity_name: String,
    config: Option<RustConfig>,
) -> Result<RustOutput, String> {
    let config = config.unwrap_or_default();
    let compiler = RustCompiler::new(spec, entity_name, config);
    Ok(compiler.compile())
}

pub fn write_rust_crate(
    output: &RustOutput,
    crate_dir: &std::path::Path,
) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(crate_dir.join("src"))?;
    std::fs::write(crate_dir.join("Cargo.toml"), &output.cargo_toml)?;
    std::fs::write(crate_dir.join("src/lib.rs"), &output.lib_rs)?;
    std::fs::write(crate_dir.join("src/types.rs"), &output.types_rs)?;
    std::fs::write(crate_dir.join("src/entity.rs"), &output.entity_rs)?;
    Ok(())
}

struct RustCompiler {
    spec: SerializableStreamSpec,
    entity_name: String,
    config: RustConfig,
}

impl RustCompiler {
    fn new(spec: SerializableStreamSpec, entity_name: String, config: RustConfig) -> Self {
        Self {
            spec,
            entity_name,
            config,
        }
    }

    fn compile(&self) -> RustOutput {
        RustOutput {
            cargo_toml: self.generate_cargo_toml(),
            lib_rs: self.generate_lib_rs(),
            types_rs: self.generate_types_rs(),
            entity_rs: self.generate_entity_rs(),
        }
    }

    fn generate_cargo_toml(&self) -> String {
        format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
hyperstack-sdk = "{}"
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
"#,
            self.config.crate_name, self.config.sdk_version
        )
    }

    fn generate_lib_rs(&self) -> String {
        format!(
            r#"mod types;
mod entity;

pub use types::*;
pub use entity::{entity_name}Entity;

pub use hyperstack_sdk::{{HyperStack, Entity, Update, ConnectionState}};
"#,
            entity_name = self.entity_name
        )
    }

    fn generate_types_rs(&self) -> String {
        let mut output = String::new();
        output.push_str("use serde::{Deserialize, Serialize};\n\n");

        let mut generated = HashSet::new();

        for section in &self.spec.sections {
            if !Self::is_root_section(&section.name) && generated.insert(section.name.clone()) {
                output.push_str(&self.generate_struct_for_section(section));
                output.push_str("\n\n");
            }
        }

        output.push_str(&self.generate_main_entity_struct());
        output.push_str(&self.generate_resolved_types(&mut generated));
        output.push_str(&self.generate_event_wrapper());

        output
    }

    fn generate_struct_for_section(&self, section: &EntitySection) -> String {
        let struct_name = format!("{}{}", self.entity_name, to_pascal_case(&section.name));
        let mut fields = Vec::new();

        for field in &section.fields {
            let field_name = to_snake_case(&field.field_name);
            let rust_type = self.field_type_to_rust(field);

            let serde_attr = if field_name != to_snake_case(&field.field_name)
                || field_name != field.field_name
            {
                let original = &field.field_name;
                if to_snake_case(original) != *original {
                    format!(
                        "    #[serde(rename = \"{}\", default)]\n",
                        to_camel_case(original)
                    )
                } else {
                    "    #[serde(default)]\n".to_string()
                }
            } else {
                "    #[serde(default)]\n".to_string()
            };

            fields.push(format!(
                "{}    pub {}: {},",
                serde_attr,
                to_snake_case(&field.field_name),
                rust_type
            ));
        }

        format!(
            "#[derive(Debug, Clone, Serialize, Deserialize, Default)]\npub struct {} {{\n{}\n}}",
            struct_name,
            fields.join("\n")
        )
    }

    /// Check if a section name is the root section (case-insensitive)
    fn is_root_section(name: &str) -> bool {
        name.eq_ignore_ascii_case("root")
    }

    fn generate_main_entity_struct(&self) -> String {
        let mut fields = Vec::new();

        for section in &self.spec.sections {
            if !Self::is_root_section(&section.name) {
                let field_name = to_snake_case(&section.name);
                let type_name = format!("{}{}", self.entity_name, to_pascal_case(&section.name));
                let serde_attr = if field_name != section.name {
                    format!(
                        "    #[serde(rename = \"{}\", default)]\n",
                        to_camel_case(&section.name)
                    )
                } else {
                    "    #[serde(default)]\n".to_string()
                };
                fields.push(format!(
                    "{}    pub {}: {},",
                    serde_attr, field_name, type_name
                ));
            }
        }

        for section in &self.spec.sections {
            if Self::is_root_section(&section.name) {
                for field in &section.fields {
                    let field_name = to_snake_case(&field.field_name);
                    let rust_type = self.field_type_to_rust(field);
                    fields.push(format!(
                        "    #[serde(default)]\n    pub {}: {},",
                        field_name, rust_type
                    ));
                }
            }
        }

        format!(
            "#[derive(Debug, Clone, Serialize, Deserialize, Default)]\npub struct {} {{\n{}\n}}",
            self.entity_name,
            fields.join("\n")
        )
    }

    fn generate_resolved_types(&self, generated: &mut HashSet<String>) -> String {
        let mut output = String::new();

        for section in &self.spec.sections {
            for field in &section.fields {
                if let Some(resolved) = &field.resolved_type {
                    if generated.insert(resolved.type_name.clone()) {
                        output.push_str("\n\n");
                        output.push_str(&self.generate_resolved_struct(resolved));
                    }
                }
            }
        }

        output
    }

    fn generate_resolved_struct(&self, resolved: &ResolvedStructType) -> String {
        if resolved.is_enum {
            let variants: Vec<String> = resolved
                .enum_variants
                .iter()
                .map(|v| format!("    {},", to_pascal_case(v)))
                .collect();

            format!(
                "#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\npub enum {} {{\n{}\n}}",
                to_pascal_case(&resolved.type_name),
                variants.join("\n")
            )
        } else {
            let fields: Vec<String> = resolved
                .fields
                .iter()
                .map(|f| {
                    let rust_type = self.resolved_field_to_rust(f);
                    let serde_attr = format!(
                        "    #[serde(rename = \"{}\", default)]\n",
                        to_camel_case(&f.field_name)
                    );
                    format!(
                        "{}    pub {}: {},",
                        serde_attr,
                        to_snake_case(&f.field_name),
                        rust_type
                    )
                })
                .collect();

            format!(
                "#[derive(Debug, Clone, Serialize, Deserialize, Default)]\npub struct {} {{\n{}\n}}",
                to_pascal_case(&resolved.type_name),
                fields.join("\n")
            )
        }
    }

    fn generate_event_wrapper(&self) -> String {
        r#"

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWrapper<T> {
    #[serde(default)]
    pub timestamp: i64,
    pub data: T,
    #[serde(default)]
    pub slot: Option<f64>,
    #[serde(default)]
    pub signature: Option<String>,
}

impl<T: Default> Default for EventWrapper<T> {
    fn default() -> Self {
        Self {
            timestamp: 0,
            data: T::default(),
            slot: None,
            signature: None,
        }
    }
}
"#
        .to_string()
    }

    fn generate_entity_rs(&self) -> String {
        let entity_name = &self.entity_name;

        format!(
            r#"use hyperstack_sdk::Entity;
use crate::types::{entity_name};

pub struct {entity_name}Entity;

impl Entity for {entity_name}Entity {{
    type Data = {entity_name};
    
    const NAME: &'static str = "{entity_name}";
    
    fn state_view() -> &'static str {{
        "{entity_name}/state"
    }}
    
    fn list_view() -> &'static str {{
        "{entity_name}/list"
    }}
}}
"#,
            entity_name = entity_name
        )
    }

    /// Generate Rust type for a field.
    ///
    /// All fields are wrapped in Option<T> because we receive partial patches,
    /// so any field may not yet be present.
    ///
    /// - Non-optional spec fields become `Option<T>`:
    ///   - `None` = not yet received in any patch
    ///   - `Some(value)` = has value
    ///
    /// - Optional spec fields become `Option<Option<T>>`:
    ///   - `None` = not yet received in any patch
    ///   - `Some(None)` = explicitly set to null
    ///   - `Some(Some(value))` = has value
    fn field_type_to_rust(&self, field: &FieldTypeInfo) -> String {
        let base = self.base_type_to_rust(&field.base_type, &field.rust_type_name);

        let typed = if field.is_array && !matches!(field.base_type, BaseType::Array) {
            format!("Vec<{}>", base)
        } else {
            base
        };

        // All fields wrapped in Option since we receive patches
        // Optional spec fields get Option<Option<T>> to distinguish "not received" from "explicitly null"
        if field.is_optional {
            format!("Option<Option<{}>>", typed)
        } else {
            format!("Option<{}>", typed)
        }
    }

    fn base_type_to_rust(&self, base_type: &BaseType, rust_type_name: &str) -> String {
        match base_type {
            BaseType::Integer => {
                if rust_type_name.contains("u64") {
                    "u64".to_string()
                } else if rust_type_name.contains("i64") {
                    "i64".to_string()
                } else if rust_type_name.contains("u32") {
                    "u32".to_string()
                } else if rust_type_name.contains("i32") {
                    "i32".to_string()
                } else {
                    "i64".to_string()
                }
            }
            BaseType::Float => "f64".to_string(),
            BaseType::String => "String".to_string(),
            BaseType::Boolean => "bool".to_string(),
            BaseType::Timestamp => "i64".to_string(),
            BaseType::Binary => "Vec<u8>".to_string(),
            BaseType::Pubkey => "String".to_string(),
            BaseType::Array => "Vec<serde_json::Value>".to_string(),
            BaseType::Object => "serde_json::Value".to_string(),
            BaseType::Any => "serde_json::Value".to_string(),
        }
    }

    fn resolved_field_to_rust(&self, field: &ResolvedField) -> String {
        let base = self.base_type_to_rust(&field.base_type, &field.field_type);

        let typed = if field.is_array {
            format!("Vec<{}>", base)
        } else {
            base
        };

        if field.is_optional {
            format!("Option<Option<{}>>", typed)
        } else {
            format!("Option<{}>", typed)
        }
    }
}

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

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    let mut chars = pascal.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
    }
}
