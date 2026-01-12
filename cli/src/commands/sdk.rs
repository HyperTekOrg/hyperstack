use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::config::{discover_ast_files, find_ast_file, DiscoveredAst, HyperstackConfig};

/// List all available specs from the configuration or auto-discovered ASTs
pub fn list(config_path: &str) -> Result<()> {
    // Try to load config
    let config = HyperstackConfig::load_optional(config_path)?;

    // Also discover local AST files
    let discovered = discover_ast_files(None)?;

    let has_config_specs = config
        .as_ref()
        .map(|c| !c.specs.is_empty())
        .unwrap_or(false);

    if !has_config_specs && discovered.is_empty() {
        println!("{}", "No specs found.".yellow());
        println!();
        println!("To add specs:");
        println!("  1. Build your spec crate to generate .hyperstack/*.ast.json files");
        println!(
            "  2. Run {} to create a configuration",
            "hyperstack init".cyan()
        );
        return Ok(());
    }

    println!("{} Available specs:\n", "→".blue().bold());

    // Show config-based specs
    if let Some(ref cfg) = config {
        for spec in &cfg.specs {
            let name = spec.name.as_deref().unwrap_or(&spec.ast);
            println!("  {}", name.green().bold());
            println!("    AST: {}", spec.ast);

            if let Some(desc) = &spec.description {
                println!("    Description: {}", desc);
            }

            let ts_output = cfg.get_typescript_output_path(name, None);
            let rust_output = cfg.get_rust_output_path(name, None);
            println!("    TypeScript: {}", ts_output.display());
            println!("    Rust: {}", rust_output.display());
            println!();
        }
    }

    // Show discovered ASTs not in config
    let config_asts: std::collections::HashSet<_> = config
        .as_ref()
        .map(|c| c.specs.iter().map(|s| s.ast.clone()).collect())
        .unwrap_or_default();

    for ast in discovered {
        if !config_asts.contains(&ast.entity_name) {
            println!("  {} {}", "•".dimmed(), ast.spec_name.green().bold());
            println!("    Entity: {}", ast.entity_name);
            println!("    Path: {}", ast.path.display());
            if let Some(pid) = &ast.program_id {
                println!("    Program ID: {}", pid);
            }
            println!("    {}", "(auto-discovered, not in config)".dimmed());
            println!();
        }
    }

    println!(
        "Use {} <spec-name> to generate SDK",
        "hyperstack sdk create typescript".cyan()
    );

    Ok(())
}

/// Create TypeScript SDK from a spec
pub fn create_typescript(
    config_path: &str,
    spec_name: &str,
    output_override: Option<String>,
    package_name_override: Option<String>,
) -> Result<()> {
    println!("{} Looking for spec '{}'...", "→".blue().bold(), spec_name);

    // Try to load config
    let config = HyperstackConfig::load_optional(config_path)?;

    // Try to find the AST - either via config or auto-discovery
    let (ast, output_path, package_name) = if let Some(ref cfg) = config {
        if let Some(spec_config) = cfg.find_spec(spec_name) {
            // Found in config - use config settings
            let ast = find_ast_file(&spec_config.ast, None)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "AST file not found for '{}'. Build your spec crate first.",
                    spec_config.ast
                )
            })?;

            let name = spec_config.name.as_deref().unwrap_or(&spec_config.ast);
            let output = output_override
                .map(|p| p.into())
                .unwrap_or_else(|| cfg.get_typescript_output_path(name, None));

            let pkg = package_name_override
                .or_else(|| cfg.sdk.as_ref().and_then(|s| s.typescript_package.clone()))
                .unwrap_or_else(|| "hyperstack-react".to_string());

            (ast, output, pkg)
        } else {
            // Not in config - try auto-discovery
            find_spec_by_name(spec_name, output_override, package_name_override)?
        }
    } else {
        // No config - use auto-discovery
        find_spec_by_name(spec_name, output_override, package_name_override)?
    };

    println!(
        "{} Found spec: {}",
        "✓".green().bold(),
        ast.entity_name.bold()
    );
    println!("  Path: {}", ast.path.display());
    if let Some(pid) = &ast.program_id {
        println!("  Program ID: {}", pid);
    }
    println!("  Output: {}", output_path.display());

    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {}", parent.display()))?;
    }

    println!("\n{} Generating TypeScript SDK...", "→".blue().bold());

    // Generate SDK from AST
    generate_sdk_from_ast(&ast, &output_path, &package_name)?;

    println!(
        "{} Successfully generated TypeScript SDK!",
        "✓".green().bold()
    );
    println!("  File: {}", output_path.display().to_string().bold());

    Ok(())
}

/// Find a spec by name using auto-discovery
fn find_spec_by_name(
    spec_name: &str,
    output_override: Option<String>,
    package_name_override: Option<String>,
) -> Result<(DiscoveredAst, std::path::PathBuf, String)> {
    let ast = find_ast_file(spec_name, None)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Spec '{}' not found.\n\
             Make sure you've built your spec crate to generate .hyperstack/*.ast.json files.",
            spec_name
        )
    })?;

    let output = output_override.map(|p| p.into()).unwrap_or_else(|| {
        std::path::PathBuf::from(format!("./generated/{}-stack.ts", ast.spec_name))
    });

    let pkg = package_name_override.unwrap_or_else(|| "hyperstack-react".to_string());

    Ok((ast, output, pkg))
}

fn generate_sdk_from_ast(
    ast: &DiscoveredAst,
    output_path: &Path,
    package_name: &str,
) -> Result<()> {
    println!(
        "{} Reading AST from {}...",
        "→".blue().bold(),
        ast.path.display()
    );

    let ast_json = fs::read_to_string(&ast.path)
        .with_context(|| format!("Failed to read AST file: {}", ast.path.display()))?;

    let spec: hyperstack_interpreter::ast::SerializableStreamSpec = serde_json::from_str(&ast_json)
        .with_context(|| format!("Failed to deserialize AST from {}", ast.path.display()))?;

    println!("{} Compiling TypeScript from AST...", "→".blue().bold());

    let config = hyperstack_interpreter::typescript::TypeScriptConfig {
        package_name: package_name.to_string(),
        generate_helpers: true,
        interface_prefix: String::new(),
        export_const_name: "STACK".to_string(),
    };

    let output = hyperstack_interpreter::typescript::compile_serializable_spec(
        spec,
        ast.entity_name.clone(),
        Some(config),
    )
    .map_err(|e| anyhow::anyhow!("Failed to compile TypeScript: {}", e))?;

    hyperstack_interpreter::typescript::write_typescript_to_file(&output, output_path)
        .with_context(|| format!("Failed to write TypeScript to {}", output_path.display()))?;

    Ok(())
}

pub fn create_rust(
    config_path: &str,
    spec_name: &str,
    output_override: Option<String>,
    crate_name_override: Option<String>,
) -> Result<()> {
    println!("{} Looking for spec '{}'...", "→".blue().bold(), spec_name);

    let config = HyperstackConfig::load_optional(config_path)?;

    let (ast, output_dir, crate_name) = find_spec_for_rust(
        spec_name,
        config.as_ref(),
        output_override,
        crate_name_override,
    )?;

    println!(
        "{} Found spec: {}",
        "✓".green().bold(),
        ast.entity_name.bold()
    );
    println!("  Path: {}", ast.path.display());
    if let Some(pid) = &ast.program_id {
        println!("  Program ID: {}", pid);
    }

    let crate_dir = output_dir.join(&crate_name);
    println!("  Output: {}", crate_dir.display());

    println!("\n{} Generating Rust SDK...", "→".blue().bold());

    let ast_json = fs::read_to_string(&ast.path)
        .with_context(|| format!("Failed to read AST file: {}", ast.path.display()))?;

    let spec: hyperstack_interpreter::ast::SerializableStreamSpec = serde_json::from_str(&ast_json)
        .with_context(|| format!("Failed to deserialize AST from {}", ast.path.display()))?;

    let rust_config = hyperstack_interpreter::rust::RustConfig {
        crate_name: crate_name.clone(),
        sdk_version: "0.1".to_string(),
    };

    let output = hyperstack_interpreter::rust::compile_serializable_spec(
        spec,
        ast.entity_name.clone(),
        Some(rust_config),
    )
    .map_err(|e| anyhow::anyhow!("Failed to compile Rust: {}", e))?;

    hyperstack_interpreter::rust::write_rust_crate(&output, &crate_dir)
        .with_context(|| format!("Failed to write Rust crate to {}", crate_dir.display()))?;

    println!("{} Successfully generated Rust SDK!", "✓".green().bold());
    println!("  Crate: {}", crate_dir.display().to_string().bold());
    println!("\n  Add to your Cargo.toml:");
    println!(
        "    {} = {{ path = \"{}\" }}",
        crate_name.cyan(),
        crate_dir.display()
    );

    Ok(())
}

fn find_spec_for_rust(
    spec_name: &str,
    config: Option<&HyperstackConfig>,
    output_override: Option<String>,
    crate_name_override: Option<String>,
) -> Result<(DiscoveredAst, std::path::PathBuf, String)> {
    let ast = if let Some(cfg) = config {
        if let Some(spec_config) = cfg.find_spec(spec_name) {
            find_ast_file(&spec_config.ast, None)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "AST file not found for '{}'. Build your spec crate first.",
                    spec_config.ast
                )
            })?
        } else {
            find_ast_file(spec_name, None)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Spec '{}' not found.\n\
                     Make sure you've built your spec crate to generate .hyperstack/*.ast.json files.",
                    spec_name
                )
            })?
        }
    } else {
        find_ast_file(spec_name, None)?.ok_or_else(|| {
            anyhow::anyhow!(
                "Spec '{}' not found.\n\
                 Make sure you've built your spec crate to generate .hyperstack/*.ast.json files.",
                spec_name
            )
        })?
    };

    let output_dir = output_override
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            config
                .map(|c| std::path::PathBuf::from(c.get_rust_output_dir()))
                .unwrap_or_else(|| std::path::PathBuf::from("./generated"))
        });

    let crate_name = crate_name_override.unwrap_or_else(|| format!("{}-stack", ast.spec_name));

    Ok((ast, output_dir, crate_name))
}
