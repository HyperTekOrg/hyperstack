use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperstackConfig {
    pub project: ProjectConfig,

    #[serde(default)]
    pub stacks: Vec<StackConfig>,

    #[serde(default)]
    pub sdk: Option<SdkConfig>,

    #[serde(default)]
    pub build: Option<BuildConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SdkConfig {
    #[serde(default = "default_output_dir")]
    pub output_dir: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub typescript_output_dir: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_output_dir: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub typescript_package: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_crate_prefix: Option<String>,

    #[serde(default)]
    pub rust_module_mode: bool,
}

fn default_output_dir() -> String {
    "./generated".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    #[serde(default = "default_watch")]
    pub watch_by_default: bool,
}

fn default_watch() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    pub ast: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub typescript_output_file: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_output_crate: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_module: Option<bool>,
}

impl HyperstackConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: HyperstackConfig = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        config.validate()?;
        Ok(config)
    }

    pub fn load_optional<P: AsRef<Path>>(path: P) -> Result<Option<Self>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(None);
        }
        Self::load(path).map(Some)
    }

    pub fn validate(&self) -> Result<()> {
        if self.project.name.is_empty() {
            anyhow::bail!("Project name cannot be empty");
        }

        let mut names = HashSet::new();
        for stack in &self.stacks {
            if let Some(name) = &stack.name {
                if !names.insert(name.clone()) {
                    anyhow::bail!("Duplicate stack name: {}", name);
                }
            }
        }

        Ok(())
    }

    pub fn find_stack(&self, name: &str) -> Option<&StackConfig> {
        self.stacks
            .iter()
            .find(|s| s.name.as_deref() == Some(name) || s.ast == name)
    }

    pub fn get_output_dir(&self) -> &str {
        self.sdk
            .as_ref()
            .map(|s| s.output_dir.as_str())
            .unwrap_or("./generated")
    }

    pub fn get_typescript_output_dir(&self) -> &str {
        self.sdk
            .as_ref()
            .and_then(|s| s.typescript_output_dir.as_deref())
            .unwrap_or_else(|| self.get_output_dir())
    }

    pub fn get_rust_output_dir(&self) -> &str {
        self.sdk
            .as_ref()
            .and_then(|s| s.rust_output_dir.as_deref())
            .unwrap_or_else(|| self.get_output_dir())
    }

    pub fn get_typescript_output_path(
        &self,
        stack_name: &str,
        stack_config: Option<&StackConfig>,
        override_path: Option<String>,
    ) -> PathBuf {
        if let Some(path) = override_path {
            return PathBuf::from(path);
        }

        if let Some(stack) = stack_config {
            if let Some(ref file_path) = stack.typescript_output_file {
                return PathBuf::from(file_path);
            }
        }

        PathBuf::from(self.get_typescript_output_dir()).join(format!("{}-stack.ts", stack_name))
    }

    pub fn get_rust_output_path(
        &self,
        stack_name: &str,
        stack_config: Option<&StackConfig>,
        override_path: Option<String>,
    ) -> PathBuf {
        if let Some(path) = override_path {
            return PathBuf::from(path);
        }

        if let Some(stack) = stack_config {
            if let Some(ref crate_path) = stack.rust_output_crate {
                return PathBuf::from(crate_path);
            }
        }

        PathBuf::from(self.get_rust_output_dir()).join(format!("{}-stack", stack_name))
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveredAst {
    pub path: PathBuf,
    pub entity_name: String,
    pub program_id: Option<String>,
    pub stack_name: String,
}

impl DiscoveredAst {
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read AST file: {}", path.display()))?;

        let ast: serde_json::Value = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse AST JSON: {}", path.display()))?;

        let entity_name = ast
            .get("state_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("AST missing 'state_name' field: {}", path.display()))?
            .to_string();

        let program_id = ast
            .get("program_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let stack_name = to_kebab_case(&entity_name);

        Ok(Self {
            path,
            entity_name,
            program_id,
            stack_name,
        })
    }

    pub fn load_ast(&self) -> Result<serde_json::Value> {
        let contents = fs::read_to_string(&self.path)
            .with_context(|| format!("Failed to read AST file: {}", self.path.display()))?;

        serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse AST JSON: {}", self.path.display()))
    }
}

pub fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

pub fn discover_ast_files(base_path: Option<&Path>) -> Result<Vec<DiscoveredAst>> {
    let base = base_path.unwrap_or_else(|| Path::new("."));
    let mut discovered = Vec::new();

    let local_hyperstack = base.join(".hyperstack");
    if local_hyperstack.is_dir() {
        discover_in_dir(&local_hyperstack, &mut discovered)?;
    }

    discover_recursive(base, &mut discovered, 0, 3)?;

    let mut seen_entities = HashSet::new();
    discovered.retain(|ast| seen_entities.insert(ast.entity_name.clone()));

    Ok(discovered)
}

fn discover_in_dir(dir: &Path, discovered: &mut Vec<DiscoveredAst>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".ast.json") {
                    match DiscoveredAst::from_path(path.clone()) {
                        Ok(ast) => discovered.push(ast),
                        Err(e) => {
                            eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn discover_recursive(
    dir: &Path,
    discovered: &mut Vec<DiscoveredAst>,
    depth: usize,
    max_depth: usize,
) -> Result<()> {
    if depth >= max_depth || !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == ".hyperstack" {
                    discover_in_dir(&path, discovered)?;
                } else if !name.starts_with('.') && name != "node_modules" && name != "target" {
                    discover_recursive(&path, discovered, depth + 1, max_depth)?;
                }
            }
        }
    }

    Ok(())
}

pub fn find_ast_file(name: &str, base_path: Option<&Path>) -> Result<Option<DiscoveredAst>> {
    let as_path = Path::new(name);
    if as_path.exists() && as_path.is_file() {
        return DiscoveredAst::from_path(as_path.to_path_buf()).map(Some);
    }

    let discovered = discover_ast_files(base_path)?;

    let name_lower = name.to_lowercase();
    let name_kebab = to_kebab_case(name);

    Ok(discovered.into_iter().find(|ast| {
        ast.entity_name.to_lowercase() == name_lower
            || ast.stack_name == name_kebab
            || ast.stack_name == name_lower
    }))
}

pub fn resolve_stacks_to_push(
    config: Option<&HyperstackConfig>,
    stack_name: Option<&str>,
) -> Result<Vec<DiscoveredAst>> {
    if let Some(name) = stack_name {
        let ast = find_ast_file(name, None)?.ok_or_else(|| {
            anyhow::anyhow!(
                "AST file not found for '{}'\n\nSearched in .hyperstack/ directories.\n\
                 Make sure your stack is compiled (cargo build) and the AST file exists.",
                name
            )
        })?;
        return Ok(vec![ast]);
    }

    if let Some(config) = config {
        if !config.stacks.is_empty() {
            let mut result = Vec::new();
            for stack in &config.stacks {
                let ast = find_ast_file(&stack.ast, None)?.ok_or_else(|| {
                    anyhow::anyhow!(
                        "AST file not found for stack '{}' (ast: '{}')\n\
                         Make sure your stack is compiled and the AST file exists.",
                        stack.name.as_deref().unwrap_or(&stack.ast),
                        stack.ast
                    )
                })?;

                let mut ast = ast;
                if let Some(name) = &stack.name {
                    ast.stack_name = name.clone();
                }
                result.push(ast);
            }
            return Ok(result);
        }
    }

    let discovered = discover_ast_files(None)?;

    if discovered.is_empty() {
        anyhow::bail!(
            "No AST files found.\n\n\
             Make sure you have compiled your stack (cargo build) and have a\n\
             .hyperstack/*.ast.json file in your project.\n\n\
             Alternatively, create a hyperstack.toml to configure your stacks."
        );
    }

    Ok(discovered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("SettlementGame"), "settlement-game");
        assert_eq!(to_kebab_case("OreRound"), "ore-round");
        assert_eq!(to_kebab_case("PumpfunToken"), "pumpfun-token");
        assert_eq!(to_kebab_case("simple"), "simple");
        assert_eq!(to_kebab_case("ABC"), "a-b-c");
    }
}
