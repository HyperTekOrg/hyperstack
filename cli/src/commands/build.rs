use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::fs;
use std::thread;
use std::time::Duration;

use crate::api_client::{ApiClient, BuildStatus, CreateBuildRequest};
use crate::config::find_ast_file;

/// Create a new build from a spec
///
/// Modes:
/// 1. From spec name: Looks up spec by name, uses latest version
/// 2. From spec name + version: Uses specific version
/// 3. From AST file: Uploads AST directly
///
/// Config is now OPTIONAL - will auto-discover ASTs if not present
/// By default, watches build progress until completion.
pub fn create(
    _config_path: &str,
    spec_name: &str,
    version: Option<i32>,
    ast_file: Option<&str>,
    watch: bool,
) -> Result<()> {
    let client = ApiClient::new()?;

    if let Some(ast_path) = ast_file {
        // Mode 3: Direct AST upload
        println!("{} Loading AST from file...", "→".blue().bold());
        let ast_json = fs::read_to_string(ast_path)
            .with_context(|| format!("Failed to read AST file: {}", ast_path))?;

        let ast_payload: serde_json::Value = serde_json::from_str(&ast_json)
            .with_context(|| format!("Failed to parse AST JSON: {}", ast_path))?;

        // Try to get spec ID from name for association
        let spec_id = client.get_spec_by_name(spec_name)?.map(|s| s.id);

        println!("{} Creating build from AST file...", "→".blue().bold());
        let req = CreateBuildRequest {
            spec_id,
            spec_version_id: None,
            ast_payload: Some(ast_payload),
            branch: None,
        };

        let response = client.create_build(req)?;
        println!(
            "{} Build created (ID: {})",
            "✓".green().bold(),
            response.build_id
        );

        if watch {
            println!();
            return watch_build(&client, response.build_id);
        }

        println!("  Status: {}", format_status(response.status));
        println!();
        println!(
            "Track progress with: {}",
            format!("hyperstack build status {} --watch", response.build_id).cyan()
        );

        return Ok(());
    }

    // Mode 1/2: From spec name (optionally with version)
    println!("{} Looking up spec '{}'...", "→".blue().bold(), spec_name);

    // Get remote spec first
    let remote_spec = client.get_spec_by_name(spec_name)?;

    let (spec_id, spec_version_id) = match (&remote_spec, version) {
        (Some(spec), Some(v)) => {
            // Specific version requested - we need to find the version ID
            println!(
                "{} Found spec (id={}), looking up version {}...",
                "✓".green().bold(),
                spec.id,
                v
            );

            let versions = client.list_spec_versions(spec.id)?;
            let ver = versions
                .iter()
                .find(|ver| ver.version_number == v)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Version {} not found for spec '{}'. Available versions: {:?}",
                        v,
                        spec_name,
                        versions
                            .iter()
                            .map(|v| v.version_number)
                            .collect::<Vec<_>>()
                    )
                })?;

            println!("  Version {} found (hash: {})", v, &ver.content_hash[..12]);
            (Some(spec.id), Some(ver.id))
        }
        (Some(spec), None) => {
            // Latest version
            println!(
                "{} Found spec (id={}), using latest version...",
                "✓".green().bold(),
                spec.id
            );

            let spec_with_version = client.get_spec_with_latest_version(spec.id)?;

            let version_id = spec_with_version
                .latest_version
                .as_ref()
                .map(|v| v.id)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                    "Spec '{}' has no versions. Push a version first with: hyperstack spec push {}",
                    spec_name, spec_name
                )
                })?;

            if let Some(ver) = &spec_with_version.latest_version {
                println!(
                    "  Using version {} (hash: {})",
                    ver.version_number,
                    &ver.content_hash[..12]
                );
            }

            (Some(spec.id), Some(version_id))
        }
        (None, _) => {
            // Spec not found remotely - try to use local AST via auto-discovery
            println!(
                "{} Spec not found remotely, searching for local AST...",
                "!".yellow().bold()
            );

            // Try to find AST by spec name
            if let Some(ast) = find_ast_file(spec_name, None)? {
                println!(
                    "{} Found local AST: {}",
                    "✓".green().bold(),
                    ast.path.display()
                );

                let ast_payload = ast.load_ast()?;

                println!("{} Creating build from local AST...", "→".blue().bold());
                let req = CreateBuildRequest {
                    spec_id: None,
                    spec_version_id: None,
                    ast_payload: Some(ast_payload),
                    branch: None,
                };

                let response = client.create_build(req)?;
                println!(
                    "{} Build created (ID: {})",
                    "✓".green().bold(),
                    response.build_id
                );

                if watch {
                    println!();
                    return watch_build(&client, response.build_id);
                }

                println!("  Status: {}", format_status(response.status));
                println!();
                println!(
                    "Track progress with: {}",
                    format!("hyperstack build status {} --watch", response.build_id).cyan()
                );

                return Ok(());
            }

            bail!(
                "Spec '{}' not found remotely and no local AST file found.\n\n\
                 To fix this:\n\
                 1. Build your spec crate to generate the AST file: cargo build\n\
                 2. Push your spec: hyperstack spec push {}\n\
                 3. Then create a build: hyperstack build create {}",
                spec_name,
                spec_name,
                spec_name
            );
        }
    };

    println!("{} Creating build...", "→".blue().bold());
    let req = CreateBuildRequest {
        spec_id,
        spec_version_id,
        ast_payload: None,
        branch: None,
    };

    let response = client.create_build(req)?;
    println!(
        "{} Build created (ID: {})",
        "✓".green().bold(),
        response.build_id
    );

    if watch {
        println!();
        return watch_build(&client, response.build_id);
    }

    println!("  Status: {}", format_status(response.status));
    println!();
    println!(
        "Track progress with: {}",
        format!("hyperstack build status {} --watch", response.build_id).cyan()
    );

    Ok(())
}

/// List builds for the authenticated user
pub fn list(limit: i64, status_filter: Option<&str>, json: bool) -> Result<()> {
    let client = ApiClient::new()?;

    if !json {
        println!("{} Fetching builds...", "→".blue().bold());
    }
    let builds = client.list_builds(Some(limit), None)?;

    // Filter by status if requested
    let filtered_builds: Vec<_> = if let Some(filter) = status_filter {
        let filter_lower = filter.to_lowercase();
        builds
            .into_iter()
            .filter(|b| b.status.to_string() == filter_lower)
            .collect()
    } else {
        builds
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&filtered_builds)?);
        return Ok(());
    }

    if filtered_builds.is_empty() {
        if status_filter.is_some() {
            println!(
                "{}",
                format!(
                    "No builds with status '{}' found.",
                    status_filter.unwrap_or("")
                )
                .yellow()
            );
        } else {
            println!("{}", "No builds found.".yellow());
            println!(
                "Create a build with: {}",
                "hyperstack build create <spec-name>".cyan()
            );
        }
        return Ok(());
    }

    println!("{} Builds:\n", "→".blue().bold());

    for build in &filtered_builds {
        let status_str = format_status(build.status);
        let id_str = format!("#{}", build.id).bold();

        println!("  {} {}", id_str, status_str);

        if let Some(msg) = &build.status_message {
            println!("    {}", msg.dimmed());
        }

        if let Some(phase) = &build.phase {
            println!("    Phase: {}", phase);
        }

        if let Some(ws_url) = &build.websocket_url {
            println!("    WebSocket: {}", ws_url.cyan());
        }

        println!("    Created: {}", build.created_at);

        if let Some(completed) = &build.completed_at {
            println!("    Completed: {}", completed);
        }

        println!();
    }

    println!("Total: {} build(s)", filtered_builds.len());

    Ok(())
}

/// Get detailed status for a specific build
pub fn status(build_id: i32, watch: bool, json_output: bool) -> Result<()> {
    let client = ApiClient::new()?;

    if watch {
        return watch_build(&client, build_id);
    }

    let response = client.get_build(build_id)?;
    let build = &response.build;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }

    println!("{} Build #{}\n", "→".blue().bold(), build_id);

    println!("  Status: {}", format_status(build.status));

    if let Some(msg) = &build.status_message {
        println!("  Message: {}", msg);
    }

    if let Some(phase) = &build.phase {
        println!("  Current Phase: {}", phase);
    }

    if let Some(progress) = build.progress {
        println!("  Progress: {}%", progress);
    }

    println!();
    println!("  {} Metadata", "•".dimmed());

    if let Some(spec_id) = build.spec_id {
        println!("    Spec ID: {}", spec_id);
    }

    if let Some(ver_id) = build.spec_version_id {
        println!("    Spec Version ID: {}", ver_id);
    }

    println!("    Created: {}", build.created_at);

    if let Some(started) = &build.started_at {
        println!("    Started: {}", started);
    }

    if let Some(completed) = &build.completed_at {
        println!("    Completed: {}", completed);
    }

    // Deployment info (only show if we have a WebSocket URL)
    if let Some(ws_url) = &build.websocket_url {
        println!();
        println!("  {} Deployment", "•".dimmed());
        println!("    WebSocket: {}", ws_url.cyan().bold());
    }

    // Events
    if !response.events.is_empty() {
        println!();
        println!("  {} Recent Events", "•".dimmed());

        for event in response.events.iter().take(10) {
            let status_change = match (&event.previous_status, &event.new_status) {
                (Some(prev), Some(new)) => format!("{} -> {}", prev, new),
                (None, Some(new)) => format!("-> {}", new),
                _ => String::new(),
            };

            println!(
                "    {} {} {}",
                event.created_at.dimmed(),
                event.event_type,
                status_change.dimmed()
            );
        }
    }

    Ok(())
}

/// Open or display logs URL for a build
/// Note: logs_url is no longer exposed in the sanitized API response
/// This function now displays a message directing users to AWS console
pub fn logs(build_id: i32) -> Result<()> {
    let client = ApiClient::new()?;
    let response = client.get_build(build_id)?;
    let build = &response.build;

    println!("{} Build #{} logs:", "→".blue().bold(), build_id);
    println!();
    println!("  Build status: {}", format_status(build.status));

    if let Some(ws_url) = &build.websocket_url {
        println!("  WebSocket URL: {}", ws_url.cyan());
    }

    println!();
    println!(
        "  {} Logs are available in the AWS Console.",
        "!".yellow().bold()
    );
    println!("  Check CodeBuild logs for detailed build output.");

    Ok(())
}

/// Watch a build until it reaches a terminal state
fn watch_build(client: &ApiClient, build_id: i32) -> Result<()> {
    println!("{} Watching build #{}...\n", "→".blue().bold(), build_id);

    let mut last_status: Option<BuildStatus> = None;
    let mut last_phase: Option<String> = None;

    loop {
        let response = client.get_build(build_id)?;
        let build = &response.build;

        // Print status change
        if last_status != Some(build.status) {
            println!(
                "  {} Status: {}",
                chrono_now().dimmed(),
                format_status(build.status)
            );
            last_status = Some(build.status);
        }

        // Print phase change
        if last_phase != build.phase {
            if let Some(phase) = &build.phase {
                println!("  {} Phase: {}", chrono_now().dimmed(), phase);
            }
            last_phase = build.phase.clone();
        }

        // Print status message if changed
        if let Some(msg) = &build.status_message {
            if !msg.is_empty() {
                println!("  {} {}", chrono_now().dimmed(), msg.dimmed());
            }
        }

        // Check for terminal state
        if build.status.is_terminal() {
            println!();

            match build.status {
                BuildStatus::Completed => {
                    println!("{} Build completed successfully!", "✓".green().bold());

                    if let Some(ws_url) = &build.websocket_url {
                        println!();
                        println!("  WebSocket URL: {}", ws_url.cyan().bold());
                    }
                }
                BuildStatus::Failed => {
                    println!("{} Build failed!", "✗".red().bold());

                    if let Some(msg) = &build.status_message {
                        println!("  {}", msg);
                    }
                }
                BuildStatus::Cancelled => {
                    println!("{} Build was cancelled.", "!".yellow().bold());
                }
                _ => {}
            }

            break;
        }

        // Poll interval
        thread::sleep(Duration::from_secs(3));
    }

    Ok(())
}

// Helper functions

fn format_status(status: BuildStatus) -> String {
    match status {
        BuildStatus::Pending => "pending".yellow().to_string(),
        BuildStatus::Uploading => "uploading".yellow().to_string(),
        BuildStatus::Queued => "queued".yellow().to_string(),
        BuildStatus::Building => "building".blue().to_string(),
        BuildStatus::Pushing => "pushing".blue().to_string(),
        BuildStatus::Deploying => "deploying".blue().to_string(),
        BuildStatus::Completed => "completed".green().bold().to_string(),
        BuildStatus::Failed => "failed".red().bold().to_string(),
        BuildStatus::Cancelled => "cancelled".dimmed().to_string(),
    }
}

fn chrono_now() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}
