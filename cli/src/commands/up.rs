use anyhow::Result;
use colored::Colorize;
use std::thread;
use std::time::Duration;

use crate::api_client::{ApiClient, BuildStatus, CreateBuildRequest};
use crate::config::{resolve_stacks_to_push, HyperstackConfig};

fn generate_short_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:08x}", (timestamp & 0xFFFFFFFF) as u32)
}

pub fn up(
    config_path: &str,
    stack_name: Option<&str>,
    branch: Option<String>,
    preview: bool,
) -> Result<()> {
    let config = HyperstackConfig::load_optional(config_path)?;
    let client = ApiClient::new()?;

    let branch = if preview {
        Some(format!("preview-{}", generate_short_uuid()))
    } else {
        branch
    };

    let stacks = resolve_stacks_to_push(config.as_ref(), stack_name)?;

    if stacks.is_empty() {
        anyhow::bail!("No stacks found to deploy");
    }

    if stacks.len() > 1 && stack_name.is_none() {
        println!(
            "{} Found {} stacks. Deploying all...\n",
            "→".blue().bold(),
            stacks.len()
        );
    }

    for ast in stacks {
        deploy_single_stack(&client, &ast, branch.as_deref())?;
        println!();
    }

    Ok(())
}

fn deploy_single_stack(
    client: &ApiClient,
    ast: &crate::config::DiscoveredAst,
    branch: Option<&str>,
) -> Result<()> {
    println!("{}", "━".repeat(50).dimmed());
    if let Some(branch_name) = branch {
        println!(
            "{} Deploying {} (branch: {})",
            "→".blue().bold(),
            ast.stack_name.bold(),
            branch_name.cyan()
        );
    } else {
        println!("{} Deploying {}", "→".blue().bold(), ast.stack_name.bold());
    }
    println!("{}", "━".repeat(50).dimmed());

    println!("\n{} Pushing stack...", "1".blue().bold());

    let remote_spec = client.get_spec_by_name(&ast.stack_name)?;

    let spec_id = if let Some(spec) = remote_spec {
        println!("  {} Stack exists (id={})", "✓".green(), spec.id);
        spec.id
    } else {
        print!("  Creating stack... ");
        let req = crate::api_client::CreateSpecRequest {
            name: ast.stack_name.clone(),
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

        spinner_idx = (spinner_idx + 1) % spinner_chars.len();
        let spinner = spinner_chars[spinner_idx];

        if last_phase != build.phase {
            if let Some(phase) = &build.phase {
                let phase_display = humanize_phase(phase);
                println!("  {} {}", "→".blue(), phase_display);
            }
            last_phase = build.phase.clone();
        }

        if let Some(progress) = build.progress {
            if last_progress != Some(progress) {
                let bar = render_progress_bar(progress, 30);
                print!("\r  {} {} {}%  ", spinner.to_string().blue(), bar, progress);
                std::io::Write::flush(&mut std::io::stdout())?;
                last_progress = Some(progress);
            }
        }

        if build.status.is_terminal() {
            println!();
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
