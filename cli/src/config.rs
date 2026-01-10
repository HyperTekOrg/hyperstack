use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration structure for hyperstack.toml
/// This is now OPTIONAL - the CLI can work without it for single-spec projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperstackConfig {
    pub project: ProjectConfig,

    #[serde(default)]
    pub specs: Vec<SpecConfig>,

    #[serde(default)]
    pub sdk: Option<SdkConfig>,

    #[serde(default)]
    pub build: Option<BuildConfig>,
}

/// Project-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SdkConfig {
    #[serde(default = "default_output_dir")]
    pub output_dir: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub typescript_package: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_crate_prefix: Option<String>,
}

fn default_output_dir() -> String {
    "./generated".to_string()
}

/// Build preferences
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    /// Stream build progress by default (default: true)
    #[serde(default = "default_watch")]
    pub watch_by_default: bool,
}

fn default_watch() -> bool {
    true
}

/// Configuration for a single spec - now much simpler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecConfig {
    /// User-friendly name for the spec (used in CLI)
    /// If not provided, derived from AST entity name (kebab-case)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Entity name OR path to .ast.json file
    /// Examples: "SettlementGame" or "./path/to/SettlementGame.ast.json"
    pub ast: String,

    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl HyperstackConfig {
    /// Load configuration from a TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: HyperstackConfig = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        config.validate()?;
        Ok(config)
    }

    /// Try to load config, returning None if file doesn't exist
    pub fn load_optional<P: AsRef<Path>>(path: P) -> Result<Option<Self>> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(None);
        }
        Self::load(path).map(Some)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.project.name.is_empty() {
            anyhow::bail!("Project name cannot be empty");
        }

        // Check for duplicate spec names
        let mut names = HashSet::new();
        for spec in &self.specs {
            if let Some(name) = &spec.name {
                if !names.insert(name.clone()) {
                    anyhow::bail!("Duplicate spec name: {}", name);
                }
            }
        }

        Ok(())
    }

    /// Find a spec by name
    pub fn find_spec(&self, name: &str) -> Option<&SpecConfig> {
        self.specs
            .iter()
            .find(|s| s.name.as_deref() == Some(name) || s.ast == name)
    }

    /// Get the output directory for SDK generation
    pub fn get_output_dir(&self) -> &str {
        self.sdk
            .as_ref()
            .map(|s| s.output_dir.as_str())
            .unwrap_or("./generated")
    }

    /// Get output path for a spec
    pub fn get_output_path(&self, spec_name: &str, override_path: Option<String>) -> PathBuf {
        if let Some(path) = override_path {
            return PathBuf::from(path);
        }

        PathBuf::from(self.get_output_dir()).join(format!("{}-stack.ts", spec_name))
    }
}

/// Represents a discovered AST file with its metadata
#[derive(Debug, Clone)]
pub struct DiscoveredAst {
    /// Path to the AST file
    pub path: PathBuf,
    /// Entity name extracted from AST (e.g., "SettlementGame")
    pub entity_name: String,
    /// Program ID from AST (if present)
    pub program_id: Option<String>,
    /// Derived spec name (kebab-case of entity name)
    pub spec_name: String,
}

impl DiscoveredAst {
    /// Load AST metadata from a file
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

        let spec_name = to_kebab_case(&entity_name);

        Ok(Self {
            path,
            entity_name,
            program_id,
            spec_name,
        })
    }

    /// Load the full AST content as JSON
    pub fn load_ast(&self) -> Result<serde_json::Value> {
        let contents = fs::read_to_string(&self.path)
            .with_context(|| format!("Failed to read AST file: {}", self.path.display()))?;

        serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse AST JSON: {}", self.path.display()))
    }
}

/// Convert PascalCase to kebab-case
/// e.g., "SettlementGame" -> "settlement-game"
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

/// Discover AST files in the current directory and subdirectories
pub fn discover_ast_files(base_path: Option<&Path>) -> Result<Vec<DiscoveredAst>> {
    let base = base_path.unwrap_or_else(|| Path::new("."));
    let mut discovered = Vec::new();

    // First check .hyperstack/ in the base directory
    let local_hyperstack = base.join(".hyperstack");
    if local_hyperstack.is_dir() {
        discover_in_dir(&local_hyperstack, &mut discovered)?;
    }

    // Then search subdirectories (max depth 3 to avoid excessive searching)
    discover_recursive(base, &mut discovered, 0, 3)?;

    // Deduplicate by entity name (prefer closer paths)
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
                            // Log but don't fail - might be an invalid file
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
                // Skip hidden dirs (except .hyperstack), node_modules, target, etc.
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

/// Find a specific AST file by entity name or spec name
pub fn find_ast_file(name: &str, base_path: Option<&Path>) -> Result<Option<DiscoveredAst>> {
    // First check if it's a direct path
    let as_path = Path::new(name);
    if as_path.exists() && as_path.is_file() {
        return DiscoveredAst::from_path(as_path.to_path_buf()).map(Some);
    }

    // Search for matching AST
    let discovered = discover_ast_files(base_path)?;

    // Match by entity name or spec name
    let name_lower = name.to_lowercase();
    let name_kebab = to_kebab_case(name);

    Ok(discovered.into_iter().find(|ast| {
        ast.entity_name.to_lowercase() == name_lower
            || ast.spec_name == name_kebab
            || ast.spec_name == name_lower
    }))
}

/// Resolve specs to push - either from config or auto-discovered
pub fn resolve_specs_to_push(
    config: Option<&HyperstackConfig>,
    spec_name: Option<&str>,
) -> Result<Vec<DiscoveredAst>> {
    // If a specific spec name is given, find just that one
    if let Some(name) = spec_name {
        let ast = find_ast_file(name, None)?.ok_or_else(|| {
            anyhow::anyhow!(
                "AST file not found for '{}'\n\nSearched in .hyperstack/ directories.\n\
                 Make sure your spec is compiled (cargo build) and the AST file exists.",
                name
            )
        })?;
        return Ok(vec![ast]);
    }

    // If we have a config with specs defined, use those
    if let Some(config) = config {
        if !config.specs.is_empty() {
            let mut result = Vec::new();
            for spec in &config.specs {
                let ast = find_ast_file(&spec.ast, None)?.ok_or_else(|| {
                    anyhow::anyhow!(
                        "AST file not found for spec '{}' (ast: '{}')\n\
                         Make sure your spec is compiled and the AST file exists.",
                        spec.name.as_deref().unwrap_or(&spec.ast),
                        spec.ast
                    )
                })?;

                // Override spec name if provided in config
                let mut ast = ast;
                if let Some(name) = &spec.name {
                    ast.spec_name = name.clone();
                }
                result.push(ast);
            }
            return Ok(result);
        }
    }

    // Auto-discover all AST files
    let discovered = discover_ast_files(None)?;

    if discovered.is_empty() {
        anyhow::bail!(
            "No AST files found.\n\n\
             Make sure you have compiled your spec (cargo build) and have a\n\
             .hyperstack/*.ast.json file in your project.\n\n\
             Alternatively, create a hyperstack.toml to configure your specs."
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
