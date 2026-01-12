use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::config::{discover_ast_files, HyperstackConfig, ProjectConfig, SdkConfig, SpecConfig};

/// Initialize a new hyperstack.toml configuration file
///
/// Now smarter - auto-detects AST files and prompts for project name
pub fn init(config_path: &str) -> Result<()> {
    let path = Path::new(config_path);

    if path.exists() {
        anyhow::bail!(
            "Configuration file already exists: {}\nUse a different path or remove the existing file.",
            path.display()
        );
    }

    println!("{} Initializing Hyperstack project...\n", "→".blue().bold());

    // Auto-discover AST files
    println!("{} Scanning for AST files...", "→".blue().bold());
    let discovered = discover_ast_files(None)?;

    if discovered.is_empty() {
        println!("  {}", "No AST files found.".yellow());
        println!("  Build your spec crate first to generate .hyperstack/*.ast.json files.\n");
    } else {
        println!("  {} Found {} AST file(s):", "✓".green(), discovered.len());
        for ast in &discovered {
            println!(
                "    {} {} ({})",
                "•".dimmed(),
                ast.entity_name,
                ast.path.display()
            );
        }
        println!();
    }

    // Prompt for project name
    let project_name = prompt_project_name()?;

    // Build config
    let specs: Vec<SpecConfig> = discovered
        .iter()
        .map(|ast| SpecConfig {
            name: Some(ast.spec_name.clone()),
            ast: ast.entity_name.clone(),
            description: None,
        })
        .collect();

    let config = HyperstackConfig {
        project: ProjectConfig {
            name: project_name.clone(),
        },
        specs,
        sdk: Some(SdkConfig {
            output_dir: "./generated".to_string(),
            typescript_output_dir: None,
            rust_output_dir: None,
            typescript_package: None,
            rust_crate_prefix: None,
        }),
        build: None,
    };

    // Write config
    let config_toml = toml::to_string_pretty(&config)?;
    fs::write(path, &config_toml)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    println!("{} Created {}", "✓".green().bold(), path.display());
    println!();

    if config.specs.is_empty() {
        println!("{}", "Next steps:".bold());
        println!("  1. Build your spec crate: {}", "cargo build".cyan());
        println!("  2. Run init again or manually add specs to hyperstack.toml");
        println!("  3. Push your spec: {}", "hyperstack spec push".cyan());
    } else {
        println!("{}", "Next steps:".bold());
        println!(
            "  {} to verify your configuration",
            "hyperstack config validate".cyan()
        );
        println!(
            "  {} to push your specs to remote",
            "hyperstack spec push".cyan()
        );
        println!("  {} to deploy (push + build)", "hyperstack up".cyan());
    }

    Ok(())
}

/// Prompt user for project name
fn prompt_project_name() -> Result<String> {
    // Try to derive from current directory
    let default_name = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "my-project".to_string());

    print!("Project name [{}]: ", default_name.dimmed());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(default_name)
    } else {
        Ok(input.to_string())
    }
}

/// Validate the hyperstack.toml configuration file
pub fn validate(config_path: &str) -> Result<()> {
    println!("{} Validating configuration...", "→".blue().bold());

    let config = HyperstackConfig::load(config_path).context(
        "Failed to load configuration. Run `hyperstack init` to create a configuration file.",
    )?;

    println!("{} Configuration is valid!", "✓".green().bold());
    println!();
    println!("  Project: {}", config.project.name.bold());

    if let Some(sdk) = &config.sdk {
        println!("  SDK output: {}", sdk.output_dir);
        if let Some(pkg) = &sdk.typescript_package {
            println!("  TypeScript package: {}", pkg);
        }
    }

    println!();

    if config.specs.is_empty() {
        println!("  {} No specs defined", "!".yellow());
        println!(
            "  Add specs to hyperstack.toml or run {} to auto-detect",
            "hyperstack init".cyan()
        );
    } else {
        println!("  {} Specs ({}):", "•".dimmed(), config.specs.len());
        for spec in &config.specs {
            let name = spec.name.as_deref().unwrap_or(&spec.ast);
            println!("    {} {} (ast: {})", "•".dimmed(), name.bold(), spec.ast);
        }
    }

    Ok(())
}
