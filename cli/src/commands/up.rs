//! The `hyperstack up` command - the happy path for deploying specs.
//!
//! This command combines: push + build + watch into a single workflow.

use anyhow::Result;
use colored::Colorize;
use std::thread;
use std::time::Duration;

use crate::api_client::{ApiClient, BuildStatus, CreateBuildRequest};
use crate::config::{resolve_specs_to_push, HyperstackConfig};

/// Generate a short UUID for preview branches
fn generate_short_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    // Take last 8 hex chars of timestamp for uniqueness
    format!("{:08x}", (timestamp & 0xFFFFFFFF) as u32)
}

/// Deploy a spec: push AST, create build, watch until completion
pub fn up(
    config_path: &str,
    spec_name: Option<&str>,
    branch: Option<String>,
    preview: bool,
) -> Result<()> {
    let config = HyperstackConfig::load_optional(config_path)?;
    let client = ApiClient::new()?;

    // Determine the branch to use
    let branch = if preview {
        Some(format!("preview-{}", generate_short_uuid()))
    } else {
        branch
    };

    // Resolve which spec to deploy
    let specs = resolve_specs_to_push(config.as_ref(), spec_name)?;

    if specs.is_empty() {
        anyhow::bail!("No specs found to deploy");
    }

    if specs.len() > 1 && spec_name.is_none() {
        println!(
            "{} Found {} specs. Deploying all...\n",
            "→".blue().bold(),
            specs.len()
        );
    }

    for ast in specs {
        deploy_single_spec(&client, &ast, branch.as_deref())?;
        println!();
    }

    Ok(())
}

fn deploy_single_spec(
    client: &ApiClient,
    ast: &crate::config::DiscoveredAst,
    branch: Option<&str>,
) -> Result<()> {
    println!("{}", "━".repeat(50).dimmed());
    if let Some(branch_name) = branch {
        println!(
            "{} Deploying {} (branch: {})",
            "→".blue().bold(),
            ast.spec_name.bold(),
            branch_name.cyan()
        );
    } else {
        println!("{} Deploying {}", "→".blue().bold(), ast.spec_name.bold());
    }
    println!("{}", "━".repeat(50).dimmed());

    // Step 1: Push the spec
    println!("\n{} Pushing spec...", "1".blue().bold());

    // Check if spec exists remotely
    let remote_spec = client.get_spec_by_name(&ast.spec_name)?;

    let spec_id = if let Some(spec) = remote_spec {
        println!("  {} Spec exists (id={})", "✓".green(), spec.id);
        spec.id
    } else {
        // Create the spec
        print!("  Creating spec... ");
        let req = crate::api_client::CreateSpecRequest {
            name: ast.spec_name.clone(),
            entity_name: ast.entity_name.clone(),
            crate_name: String::new(),
            module_path: String::new(),
            description: None,
            package_name: None,
            output_path: None,
        };
        let new_spec = client.create_spec(req)?;
        println!("{}", "✓".green());
        new_spec.id
    };

    // Upload AST version
    print!("  Uploading AST... ");
    let ast_payload = ast.load_ast()?;
    let version_response = client.create_spec_version(spec_id, ast_payload)?;

    let hash_short = &version_response.version.content_hash[..12];
    if version_response.version_is_new {
        println!(
            "{} v{} ({})",
            "✓".green(),
            version_response.version.version_number,
            hash_short
        );
    } else {
        println!(
            "{} v{} (up to date)",
            "=".blue(),
            version_response.version.version_number
        );
    }

    // Step 2: Create build
    println!("\n{} Creating build...", "2".blue().bold());

    let req = CreateBuildRequest {
        spec_id: Some(spec_id),
        spec_version_id: Some(version_response.version.id),
        ast_payload: None,
        branch: branch.map(|s| s.to_string()),
    };

    let build_response = client.create_build(req)?;
    println!("  Build ID: {}", build_response.build_id.to_string().bold());
    if let Some(branch_name) = branch {
        println!("  Branch: {}", branch_name.cyan());
    }

    // Step 3: Watch build
    println!("\n{} Building & deploying...\n", "3".blue().bold());

    watch_build_progress(client, build_response.build_id)?;

    Ok(())
}

fn watch_build_progress(client: &ApiClient, build_id: i32) -> Result<()> {
    let mut last_phase: Option<String> = None;
    let mut last_progress: Option<i32> = None;
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut spinner_idx = 0;

    loop {
        let response = client.get_build(build_id)?;
        let build = &response.build;

        // Update spinner
        spinner_idx = (spinner_idx + 1) % spinner_chars.len();
        let spinner = spinner_chars[spinner_idx];

        // Show phase changes
        if last_phase != build.phase {
            if let Some(phase) = &build.phase {
                let phase_display = humanize_phase(phase);
                println!("  {} {}", "→".blue(), phase_display);
            }
            last_phase = build.phase.clone();
        }

        // Show progress bar if we have progress
        if let Some(progress) = build.progress {
            if last_progress != Some(progress) {
                let bar = render_progress_bar(progress, 30);
                print!("\r  {} {} {}%  ", spinner.to_string().blue(), bar, progress);
                std::io::Write::flush(&mut std::io::stdout())?;
                last_progress = Some(progress);
            }
        }

        // Check for terminal state
        if build.status.is_terminal() {
            println!(); // Clear progress line
            println!();

            match build.status {
                BuildStatus::Completed => {
                    println!("{} Deployed successfully!", "✓".green().bold());

                    if let Some(ws_url) = &build.websocket_url {
                        println!();
                        println!("  {} {}", "WebSocket:".bold(), ws_url.cyan().bold());
                    }
                }
                BuildStatus::Failed => {
                    println!("{} Build failed!", "✗".red().bold());

                    if let Some(msg) = &build.status_message {
                        println!("  {}", msg);
                    }

                    anyhow::bail!("Deployment failed");
                }
                BuildStatus::Cancelled => {
                    println!("{} Build was cancelled.", "!".yellow().bold());
                    anyhow::bail!("Deployment cancelled");
                }
                _ => {}
            }

            break;
        }

        thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}

fn render_progress_bar(progress: i32, width: usize) -> String {
    let filled = (progress as usize * width) / 100;
    let empty = width - filled;
    format!(
        "[{}{}]",
        "█".repeat(filled).green(),
        "░".repeat(empty).dimmed()
    )
}

fn humanize_phase(phase: &str) -> &str {
    match phase.to_uppercase().as_str() {
        "SUBMITTED" => "Queued",
        "PROVISIONING" => "Starting build environment",
        "DOWNLOAD_SOURCE" => "Preparing",
        "INSTALL" => "Installing dependencies",
        "PRE_BUILD" => "Preparing build",
        "BUILD" => "Building",
        "POST_BUILD" => "Finalizing build",
        "UPLOAD_ARTIFACTS" => "Publishing image",
        "FINALIZING" => "Deploying",
        "COMPLETED" => "Completed",
        _ => phase,
    }
}
