use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::config::{discover_ast_files, find_ast_file, DiscoveredAst, HyperstackConfig};
use crate::telemetry;

pub fn list(config_path: &str) -> Result<()> {
    let config = HyperstackConfig::load_optional(config_path)?;

    let discovered = discover_ast_files(None)?;

    let has_config_stacks = config
        .as_ref()
        .map(|c| !c.stacks.is_empty())
        .unwrap_or(false);

    if !has_config_stacks && discovered.is_empty() {
        println!("{}", "No stacks found.".yellow());
        println!();
        println!("To add stacks:");
        println!("  1. Build your stack crate to generate .hyperstack/*.ast.json files");
        println!("  2. Run {} to create a configuration", "hs init".cyan());
        return Ok(());
    }

    println!("{} Available stacks:\n", "→".blue().bold());

    if let Some(ref cfg) = config {
        for stack in &cfg.stacks {
            let name = stack.name.as_deref().unwrap_or(&stack.ast);
            println!("  {}", name.green().bold());
            println!("    AST: {}", stack.ast);

            if let Some(desc) = &stack.description {
                println!("    Description: {}", desc);
            }
            
            if let Some(url) = &stack.url {
                println!("    URL: {}", url.cyan());
            }

            let ts_output = cfg.get_typescript_output_path(name, Some(stack), None);
            let rust_output = cfg.get_rust_output_path(name, Some(stack), None);
            println!("    TypeScript: {}", ts_output.display());
            println!("    Rust: {}", rust_output.display());
            println!();
        }
    }

    let config_asts: std::collections::HashSet<_> = config
        .as_ref()
        .map(|c| c.stacks.iter().map(|s| s.ast.clone()).collect())
        .unwrap_or_default();

    for ast in discovered {
        if !config_asts.contains(&ast.entity_name) {
            println!("  {} {}", "•".dimmed(), ast.stack_name.green().bold());
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
        "Use {} to generate SDK",
        "hs sdk create typescript <stack-name>".cyan()
    );

    Ok(())
}

pub fn create_typescript(
    config_path: &str,
    stack_name: &str,
    output_override: Option<String>,
    package_name_override: Option<String>,
    url_override: Option<String>,
) -> Result<()> {
    println!(
        "{} Looking for stack '{}'...",
        "→".blue().bold(),
        stack_name
    );

    let config = HyperstackConfig::load_optional(config_path)?;

    // Get the config file's directory for resolving relative paths
    let config_dir = Path::new(config_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let (ast, output_path, package_name, stack_url) = if let Some(ref cfg) = config {
        if let Some(stack_config) = cfg.find_stack(stack_name) {
            let ast = find_ast_file(&stack_config.ast, None)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "AST file not found for '{}'. Build your stack crate first.",
                    stack_config.ast
                )
            })?;

            let name = stack_config.name.as_deref().unwrap_or(&stack_config.ast);
            let raw_output =
                cfg.get_typescript_output_path(name, Some(stack_config), output_override.clone());

            // Resolve relative paths relative to the config file's directory
            let output = if raw_output.is_relative() {
                config_dir.join(&raw_output)
            } else {
                raw_output
            };

            let pkg = package_name_override
                .or_else(|| cfg.sdk.as_ref().and_then(|s| s.typescript_package.clone()))
                .unwrap_or_else(|| "hyperstack-react".to_string());

            // URL priority: override > config > None
            let url = url_override.or_else(|| stack_config.url.clone());

            (ast, output, pkg, url)
        } else {
            let (ast, output, pkg) = find_stack_by_name(stack_name, output_override, package_name_override)?;
            (ast, output, pkg, url_override)
        }
    } else {
        let (ast, output, pkg) = find_stack_by_name(stack_name, output_override, package_name_override)?;
        (ast, output, pkg, url_override)
    };

    println!(
        "{} Found stack: {}",
        "✓".green().bold(),
        ast.entity_name.bold()
    );
    println!("  Path: {}", ast.path.display());
    if let Some(pid) = &ast.program_id {
        println!("  Program ID: {}", pid);
    }
    println!("  Output: {}", output_path.display());
    if let Some(url) = &stack_url {
        println!("  URL: {}", url.cyan());
    } else {
        println!("  URL: {}", "(not configured - placeholder will be generated)".dimmed());
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {}", parent.display()))?;
    }

    println!("\n{} Generating TypeScript SDK...", "→".blue().bold());

    generate_typescript_sdk_from_ast(&ast, &output_path, &package_name, stack_url)?;

    println!(
        "{} Successfully generated TypeScript SDK!",
        "✓".green().bold()
    );
    println!("  File: {}", output_path.display().to_string().bold());

    telemetry::record_sdk_generated("typescript");

    Ok(())
}

fn find_stack_by_name(
    stack_name: &str,
    output_override: Option<String>,
    package_name_override: Option<String>,
) -> Result<(DiscoveredAst, std::path::PathBuf, String)> {
    let ast = find_ast_file(stack_name, None)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Stack '{}' not found.\n\
             Make sure you've built your stack crate to generate .hyperstack/*.ast.json files.",
            stack_name
        )
    })?;

    let output = output_override.map(|p| p.into()).unwrap_or_else(|| {
        std::path::PathBuf::from(format!("./generated/{}-stack.ts", ast.stack_name))
    });

    let pkg = package_name_override.unwrap_or_else(|| "hyperstack-react".to_string());

    Ok((ast, output, pkg))
}

fn generate_typescript_sdk_from_ast(
    ast: &DiscoveredAst,
    output_path: &Path,
    package_name: &str,
    url: Option<String>,
) -> Result<()> {
    println!(
        "{} Reading AST from {}...",
        "→".blue().bold(),
        ast.path.display()
    );

    let ast_json = fs::read_to_string(&ast.path)
        .with_context(|| format!("Failed to read AST file: {}", ast.path.display()))?;

    let spec: hyperstack_interpreter::ast::SerializableStreamSpec =
        match serde_json::from_str(&ast_json) {
            Ok(s) => s,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to deserialize AST from {}: {}",
                    ast.path.display(),
                    e
                ));
            }
        };

    println!(
        "{} Deserialized {} views from AST",
        "→".blue().bold(),
        spec.views.len()
    );
    for view in &spec.views {
        println!("   View: {}", view.id);
    }

    println!("{} Compiling TypeScript from AST...", "→".blue().bold());

    let config = hyperstack_interpreter::typescript::TypeScriptConfig {
        package_name: package_name.to_string(),
        generate_helpers: true,
        interface_prefix: String::new(),
        export_const_name: "STACK".to_string(),
        url,
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
    stack_name: &str,
    output_override: Option<String>,
    crate_name_override: Option<String>,
    module_flag: bool,
    url_override: Option<String>,
) -> Result<()> {
    println!(
        "{} Looking for stack '{}'...",
        "→".blue().bold(),
        stack_name
    );

    let config = HyperstackConfig::load_optional(config_path)?;

    let config_dir = Path::new(config_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

    let stack_config = config.as_ref().and_then(|c| c.find_stack(stack_name));

    let as_module = module_flag
        || stack_config.and_then(|s| s.rust_module).unwrap_or_else(|| {
            config
                .as_ref()
                .and_then(|c| c.sdk.as_ref())
                .map(|s| s.rust_module_mode)
                .unwrap_or(false)
        });

    // URL priority: override > config > None
    let stack_url = url_override.or_else(|| stack_config.and_then(|s| s.url.clone()));

    let (ast, raw_output_dir, crate_name) = find_stack_for_rust(
        stack_name,
        config.as_ref(),
        output_override,
        crate_name_override,
    )?;

    let output_dir = if raw_output_dir.is_relative() {
        config_dir.join(&raw_output_dir)
    } else {
        raw_output_dir
    };

    println!(
        "{} Found stack: {}",
        "✓".green().bold(),
        ast.entity_name.bold()
    );
    println!("  Path: {}", ast.path.display());
    if let Some(pid) = &ast.program_id {
        println!("  Program ID: {}", pid);
    }
    println!("  Output: {}", output_dir.display());
    if as_module {
        println!("  Mode: module (mod.rs)");
    }
    if let Some(url) = &stack_url {
        println!("  URL: {}", url.cyan());
    } else {
        println!("  URL: {}", "(not configured - placeholder will be generated)".dimmed());
    }

    println!("\n{} Generating Rust SDK...", "→".blue().bold());

    let ast_json = fs::read_to_string(&ast.path)
        .with_context(|| format!("Failed to read AST file: {}", ast.path.display()))?;

    let spec: hyperstack_interpreter::ast::SerializableStreamSpec = serde_json::from_str(&ast_json)
        .with_context(|| format!("Failed to deserialize AST from {}", ast.path.display()))?;

    let rust_config = hyperstack_interpreter::rust::RustConfig {
        crate_name: crate_name.clone(),
        sdk_version: "0.2".to_string(),
        module_mode: as_module,
        url: stack_url,
    };

    let output = hyperstack_interpreter::rust::compile_serializable_spec(
        spec,
        ast.entity_name.clone(),
        Some(rust_config),
    )
    .map_err(|e| anyhow::anyhow!("Failed to compile Rust: {}", e))?;

    if as_module {
        hyperstack_interpreter::rust::write_rust_module(&output, &output_dir)
            .with_context(|| format!("Failed to write Rust module to {}", output_dir.display()))?;

        println!("{} Successfully generated Rust module!", "✓".green().bold());
        println!("  Module: {}", output_dir.display().to_string().bold());
        println!("\n  Add to your lib.rs:");
        let module_name = output_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("module");
        println!("    pub mod {};", module_name.cyan());
    } else {
        hyperstack_interpreter::rust::write_rust_crate(&output, &output_dir)
            .with_context(|| format!("Failed to write Rust crate to {}", output_dir.display()))?;

        println!("{} Successfully generated Rust SDK!", "✓".green().bold());
        println!("  Crate: {}", output_dir.display().to_string().bold());
        println!("\n  Add to your Cargo.toml:");
        println!(
            "    {} = {{ path = \"{}\" }}",
            crate_name.cyan(),
            output_dir.display()
        );
    }

    telemetry::record_sdk_generated("rust");

    Ok(())
}

fn find_stack_for_rust(
    stack_name: &str,
    config: Option<&HyperstackConfig>,
    output_override: Option<String>,
    crate_name_override: Option<String>,
) -> Result<(DiscoveredAst, std::path::PathBuf, String)> {
    let (ast, stack_config) = if let Some(cfg) = config {
        if let Some(stack_config) = cfg.find_stack(stack_name) {
            let ast = find_ast_file(&stack_config.ast, None)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "AST file not found for '{}'. Build your stack crate first.",
                    stack_config.ast
                )
            })?;
            (ast, Some(stack_config))
        } else {
            let ast = find_ast_file(stack_name, None)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "Stack '{}' not found.\n\
                     Make sure you've built your stack crate to generate .hyperstack/*.ast.json files.",
                    stack_name
                )
            })?;
            (ast, None)
        }
    } else {
        let ast = find_ast_file(stack_name, None)?.ok_or_else(|| {
            anyhow::anyhow!(
                "Stack '{}' not found.\n\
                 Make sure you've built your stack crate to generate .hyperstack/*.ast.json files.",
                stack_name
            )
        })?;
        (ast, None)
    };

    let crate_name = crate_name_override.unwrap_or_else(|| format!("{}-stack", ast.stack_name));

    let crate_dir = if let Some(cfg) = config {
        cfg.get_rust_output_path(&ast.stack_name, stack_config, output_override)
    } else {
        output_override
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(format!("./generated/{}-stack", ast.stack_name))
            })
    };

    Ok((ast, crate_dir, crate_name))
}
