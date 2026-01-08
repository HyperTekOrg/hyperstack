use anyhow::{bail, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;

use crate::api_client::{ApiClient, CreateSpecRequest, Spec as ApiSpec};
use crate::config::{resolve_specs_to_push, DiscoveredAst, HyperstackConfig, SpecConfig};

/// Push specs with their AST content to remote
/// 
/// - If `spec_name` is Some, only push that specific spec
/// - If `spec_name` is None, push all specs from config OR auto-discover ASTs
/// - Config is now OPTIONAL - will auto-discover ASTs if not present
pub fn push(config_path: &str, spec_name: Option<&str>) -> Result<()> {
    // Try to load config, but it's optional now
    let config = HyperstackConfig::load_optional(config_path)?;
    
    if config.is_none() && spec_name.is_none() {
        println!("{} No hyperstack.toml found, auto-discovering AST files...", "→".blue().bold());
    } else if config.is_some() {
        println!("{} Loading configuration...", "→".blue().bold());
    }

    // Resolve which specs to push
    let specs_to_push = resolve_specs_to_push(config.as_ref(), spec_name)?;

    if specs_to_push.is_empty() {
        println!("{}", "No specs found to push.".yellow());
        println!("\n{}", "To push specs, either:".dimmed());
        println!("  1. Build your spec crate to generate .hyperstack/*.ast.json files");
        println!("  2. Create a hyperstack.toml with your spec configuration");
        return Ok(());
    }

    let client = ApiClient::new()?;

    // Fetch remote specs to check what exists
    println!("{} Fetching remote specs...", "→".blue().bold());
    let remote_specs = client.list_specs()?;
    let remote_map: HashMap<String, ApiSpec> = remote_specs
        .into_iter()
        .map(|s| (s.name.clone(), s))
        .collect();

    println!("{} Pushing {} spec(s)...\n", "→".blue().bold(), specs_to_push.len());

    let mut created = 0;
    let mut updated = 0;
    let mut unchanged = 0;
    let mut errors = 0;

    for ast in specs_to_push {
        // Create spec config from discovered AST
        let _spec_config = SpecConfig {
            name: Some(ast.spec_name.clone()),
            ast: ast.entity_name.clone(),
            description: None,
        };

        // Ensure spec exists remotely (create or update if needed)
        let remote_spec = match remote_map.get(&ast.spec_name) {
            Some(existing) => {
                // Check if metadata needs update
                if spec_needs_update(&ast, existing) {
                    print!("  {} Updating {}... ", "↑".yellow(), ast.spec_name);
                    match update_remote_spec(&client, existing.id, &ast) {
                        Ok(updated_spec) => {
                            println!("{}", "metadata updated".dimmed());
                            updated_spec
                        }
                        Err(e) => {
                            println!("{} {}", "✗".red(), e);
                            errors += 1;
                            continue;
                        }
                    }
                } else {
                    existing.clone()
                }
            }
            None => {
                print!("  {} Creating {}... ", "+".green(), ast.spec_name);
                match create_remote_spec(&client, &ast) {
                    Ok(new_spec) => {
                        println!("{}", "✓".green());
                        created += 1;
                        new_spec
                    }
                    Err(e) => {
                        println!("{} {}", "✗".red(), e);
                        errors += 1;
                        continue;
                    }
                }
            }
        };

        // Upload AST
        print!("  {} {} ", "↑".blue(), ast.spec_name);
        match load_and_upload_ast(&client, remote_spec.id, &ast) {
            Ok(response) => {
                let hash_short = &response.version.content_hash[..12];
                if !response.version_is_new {
                    println!("{} (v{} up to date)", "=".blue(), response.version.version_number);
                    unchanged += 1;
                } else if !response.content_is_new {
                    println!("{} v{} (reused {})", "✓".green(), response.version.version_number, hash_short);
                    updated += 1;
                } else {
                    println!("{} v{} ({})", "✓".green(), response.version.version_number, hash_short);
                    updated += 1;
                }
            }
            Err(e) => {
                println!("{} {}", "✗".red(), e);
                errors += 1;
            }
        }
    }

    println!();
    if errors > 0 {
        println!("{} Push completed with errors", "!".yellow().bold());
    } else {
        println!("{} Push complete!", "✓".green().bold());
    }
    
    if created > 0 {
        println!("  New specs: {}", created);
    }
    if updated > 0 {
        println!("  Updated: {}", updated);
    }
    if unchanged > 0 {
        println!("  Unchanged: {}", unchanged);
    }
    if errors > 0 {
        println!("  {} Errors: {}", "✗".red(), errors);
        bail!("Push completed with {} error(s)", errors);
    }

    Ok(())
}

/// Load AST and upload to create a version
fn load_and_upload_ast(
    client: &ApiClient,
    spec_id: i32,
    ast: &DiscoveredAst,
) -> Result<crate::api_client::CreateSpecVersionResponse> {
    let ast_payload = ast.load_ast()?;
    client.create_spec_version(spec_id, ast_payload)
}

pub fn pull(config_path: &str) -> Result<()> {
    println!("{} Fetching remote specs...", "→".blue().bold());

    let client = ApiClient::new()?;
    let remote_specs = client.list_specs()?;

    if remote_specs.is_empty() {
        println!("{}", "No specs found on remote.".yellow());
        println!("  Run {} to upload your local specs.", "hyperstack spec push".cyan());
        return Ok(());
    }

    // Load existing config or create new one
    let mut config = HyperstackConfig::load_optional(config_path)?
        .unwrap_or_else(|| HyperstackConfig {
            project: crate::config::ProjectConfig {
                name: "my-hyperstack-project".to_string(),
            },
            specs: vec![],
            sdk: None,
            build: None,
        });

    println!("{} Syncing {} remote spec(s)...\n", "→".blue().bold(), remote_specs.len());

    let mut added = 0;
    let updated = 0;
    let mut skipped = 0;

    for remote_spec in &remote_specs {
        let local_spec = config.specs.iter().find(|s| {
            s.name.as_deref() == Some(&remote_spec.name) || s.ast == remote_spec.entity_name
        }).cloned();
        
        if let Some(_local_spec) = local_spec {
            // Spec exists locally - for now just skip
            println!("  {} {} (already in config)", "=".blue(), remote_spec.name);
            skipped += 1;
        } else {
            // Spec doesn't exist locally - add it
            print!("  {} Adding {}... ", "↓".green(), remote_spec.name);
            add_local_spec(&mut config, remote_spec);
            println!("{}", "✓".green());
            added += 1;
        }
    }

    // Save updated config
    let config_toml = toml::to_string_pretty(&config)?;
    fs::write(config_path, config_toml)?;

    println!();
    println!("{} Pull complete!", "✓".green().bold());
    println!("  Added: {}", added);
    println!("  Updated: {}", updated);
    println!("  Unchanged: {}", skipped);
    println!("  Config saved to: {}", config_path);

    Ok(())
}

pub fn list_remote(json: bool) -> Result<()> {
    let client = ApiClient::new()?;
    let specs = client.list_specs()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&specs)?);
        return Ok(());
    }

    if specs.is_empty() {
        println!("{}", "No specs found on remote.".yellow());
        println!("  Run {} to upload your local specs.", "hyperstack spec push".cyan());
        return Ok(());
    }

    println!("{} Remote specs:\n", "→".blue().bold());

    for spec in &specs {
        println!("  {}", spec.name.green().bold());
        println!("    Entity: {}", spec.entity_name);
        
        if let Some(desc) = &spec.description {
            println!("    Description: {}", desc);
        }
        
        println!();
    }

    println!("Total: {} spec(s)", specs.len());

    Ok(())
}

// Helper functions

fn spec_needs_update(local: &DiscoveredAst, remote: &ApiSpec) -> bool {
    local.entity_name != remote.entity_name
}

fn create_remote_spec(client: &ApiClient, ast: &DiscoveredAst) -> Result<ApiSpec> {
    let req = CreateSpecRequest {
        name: ast.spec_name.clone(),
        entity_name: ast.entity_name.clone(),
        // These fields are now optional/simplified
        crate_name: String::new(),
        module_path: String::new(),
        description: None,
        package_name: None,
        output_path: None,
    };

    client.create_spec(req)
}

fn update_remote_spec(client: &ApiClient, spec_id: i32, ast: &DiscoveredAst) -> Result<ApiSpec> {
    let req = crate::api_client::UpdateSpecRequest {
        name: Some(ast.spec_name.clone()),
        entity_name: Some(ast.entity_name.clone()),
        crate_name: None,
        module_path: None,
        description: None,
        package_name: None,
        output_path: None,
    };

    client.update_spec(spec_id, req)
}

fn add_local_spec(config: &mut HyperstackConfig, remote: &ApiSpec) {
    config.specs.push(SpecConfig {
        name: Some(remote.name.clone()),
        ast: remote.entity_name.clone(),
        description: remote.description.clone(),
    });
}

/// Show version history for a spec
pub fn versions(spec_name: &str, limit: i64, json: bool) -> Result<()> {
    let client = ApiClient::new()?;
    
    if !json {
        println!("{} Looking up spec '{}'...", "→".blue().bold(), spec_name);
    }
    
    let spec = client.get_spec_by_name(spec_name)?
        .ok_or_else(|| anyhow::anyhow!("Spec '{}' not found", spec_name))?;
    
    if !json {
        println!("{} Found spec (id={})", "✓".green().bold(), spec.id);
    }
    
    let versions = client.list_spec_versions_paginated(spec.id, Some(limit), None)?;
    
    if json {
        println!("{}", serde_json::to_string_pretty(&versions)?);
        return Ok(());
    }
    
    if versions.is_empty() {
        println!("\n{}", "No versions found for this spec.".yellow());
        println!("Push a version with: {}", format!("hyperstack spec push {}", spec_name).cyan());
        return Ok(());
    }
    
    println!("\n{} Version history for '{}':\n", "→".blue().bold(), spec_name);
    
    for version in &versions {
        let hash_short = &version.content_hash[..12];
        
        println!("  {} v{}", "•".dimmed(), version.version_number.to_string().bold());
        println!("    Hash: {}", hash_short);
        println!("    State: {}", version.state_name);
        println!("    Handlers: {}, Sections: {}", version.handler_count, version.section_count);
        
        if let Some(program_id) = &version.program_id {
            println!("    Program ID: {}", program_id);
        }
        
        println!("    Created: {}", version.version_created_at);
        println!();
    }
    
    println!("Total: {} version(s)", versions.len());
    
    Ok(())
}

/// Show detailed spec information
pub fn show(spec_name: &str, version: Option<i32>) -> Result<()> {
    let client = ApiClient::new()?;
    
    println!("{} Looking up spec '{}'...", "→".blue().bold(), spec_name);
    
    let spec = client.get_spec_by_name(spec_name)?
        .ok_or_else(|| anyhow::anyhow!("Spec '{}' not found", spec_name))?;
    
    let spec_with_version = client.get_spec_with_latest_version(spec.id)?;
    
    println!("\n{} Spec: {}\n", "→".blue().bold(), spec_name.green().bold());
    
    println!("  ID: {}", spec.id);
    println!("  Entity: {}", spec.entity_name);
    
    if let Some(desc) = &spec.description {
        println!("  Description: {}", desc);
    }
    
    println!("  Created: {}", spec.created_at);
    println!("  Updated: {}", spec.updated_at);
    
    // Show version info
    if let Some(ver) = &spec_with_version.latest_version {
        println!();
        println!("  {} Latest Version", "•".dimmed());
        println!("    Version: {}", ver.version_number);
        println!("    Hash: {}", &ver.content_hash[..12]);
        println!("    State: {}", ver.state_name);
        println!("    Handlers: {}, Sections: {}", ver.handler_count, ver.section_count);
        
        if let Some(program_id) = &ver.program_id {
            println!("    Program ID: {}", program_id);
        }
        
        println!("    Created: {}", ver.version_created_at);
    } else {
        println!();
        println!("  {}", "No versions pushed yet.".yellow());
        println!("  Push a version with: {}", format!("hyperstack spec push {}", spec_name).cyan());
    }
    
    // If specific version requested, show that version's details
    if let Some(v) = version {
        println!();
        println!("{} Looking up version {}...", "→".blue().bold(), v);
        
        let versions = client.list_spec_versions(spec.id)?;
        let ver = versions.iter().find(|ver| ver.version_number == v);
        
        if let Some(ver) = ver {
            println!();
            println!("  {} Version {}", "•".dimmed(), v);
            println!("    Hash: {}", ver.content_hash);
            println!("    State: {}", ver.state_name);
            println!("    Handlers: {}, Sections: {}", ver.handler_count, ver.section_count);
            
            if let Some(program_id) = &ver.program_id {
                println!("    Program ID: {}", program_id);
            }
            
            println!("    Created: {}", ver.version_created_at);
        } else {
            println!("{}", format!("Version {} not found.", v).yellow());
        }
    }
    
    Ok(())
}

/// Delete a spec from remote
pub fn delete(spec_name: &str, force: bool) -> Result<()> {
    let client = ApiClient::new()?;
    
    println!("{} Looking up spec '{}'...", "→".blue().bold(), spec_name);
    
    let spec = client.get_spec_by_name(spec_name)?
        .ok_or_else(|| anyhow::anyhow!("Spec '{}' not found", spec_name))?;
    
    if !force {
        println!();
        println!("{} You are about to delete spec '{}'", "!".yellow().bold(), spec_name);
        println!("  This will delete the spec and ALL its versions.");
        println!("  This action cannot be undone.");
        println!();
        
        print!("Type the spec name to confirm: ");
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        let mut confirmation = String::new();
        io::stdin().read_line(&mut confirmation)?;
        let confirmation = confirmation.trim();
        
        if confirmation != spec_name {
            println!();
            println!("{} Deletion cancelled.", "!".yellow().bold());
            return Ok(());
        }
    }
    
    println!("{} Deleting spec '{}'...", "→".blue().bold(), spec_name);
    
    client.delete_spec(spec.id)?;
    
    println!("{} Spec '{}' deleted successfully.", "✓".green().bold(), spec_name);
    
    Ok(())
}
