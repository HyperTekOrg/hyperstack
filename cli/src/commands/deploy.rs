use anyhow::{bail, Result};
use colored::Colorize;
use serde::Serialize;
use std::thread;
use std::time::Duration;

use crate::api_client::{ApiClient, Build, BuildStatus, CreateBuildRequest, DeploymentStatus};

/// Show deployment info for a build
pub fn info(build_id: i32, json: bool) -> Result<()> {
    let client = ApiClient::new()?;

    if !json {
        println!(
            "{} Fetching deployment info for build #{}...",
            "->".blue().bold(),
            build_id
        );
    }

    let response = client.get_build(build_id)?;
    let build = &response.build;

    if json {
        #[derive(Serialize)]
        struct DeploymentInfo {
            build_id: i32,
            status: String,
            deployed: bool,
            atom_name: Option<String>,
            namespace: Option<String>,
            domain_suffix: Option<String>,
            websocket_url: Option<String>,
            image_uri: Option<String>,
            created_at: String,
            completed_at: Option<String>,
        }

        let info = DeploymentInfo {
            build_id: build.id,
            status: build.status.to_string(),
            deployed: build.status == BuildStatus::Completed,
            atom_name: None,     // No longer exposed in sanitized API
            namespace: None,     // No longer exposed in sanitized API
            domain_suffix: None, // No longer exposed in sanitized API
            websocket_url: build.websocket_url.clone(),
            image_uri: None, // No longer exposed in sanitized API
            created_at: build.created_at.clone(),
            completed_at: build.completed_at.clone(),
        };

        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    if build.status != BuildStatus::Completed {
        println!();
        println!(
            "{} Build #{} is not deployed yet.",
            "!".yellow().bold(),
            build_id
        );
        println!("  Status: {}", format_status(build.status));

        if let Some(msg) = &build.status_message {
            println!("  Message: {}", msg);
        }

        if !build.status.is_terminal() {
            println!();
            println!("Wait for completion or watch progress with:");
            println!(
                "  {}",
                format!("hyperstack build status {} --watch", build_id).cyan()
            );
        }

        return Ok(());
    }

    // Build is completed - show deployment info
    println!();
    println!("{} Deployment Info\n", "->".green().bold());

    println!("  Build ID: {}", build.id);
    println!("  Status: {}", format_status(build.status));

    if let Some(ws_url) = &build.websocket_url {
        println!();
        println!("  {} WebSocket URL:", ">>".dimmed());
        println!("     {}", ws_url.cyan().bold());
    }

    println!();
    println!("  Created: {}", build.created_at);

    if let Some(completed) = &build.completed_at {
        println!("  Deployed: {}", completed);
    }

    Ok(())
}

/// List all deployments
pub fn list(limit: i64, json: bool) -> Result<()> {
    let client = ApiClient::new()?;

    if !json {
        println!("{} Fetching deployments...", "->".blue().bold());
    }

    let deployments = client.list_deployments(limit)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&deployments)?);
        return Ok(());
    }

    if deployments.is_empty() {
        println!("{}", "No deployments found.".yellow());
        println!();
        println!("Deploy your spec with:");
        println!("  {}", "hyperstack up".cyan());
        return Ok(());
    }

    println!("{} Deployments:\n", "->".green().bold());

    for deployment in &deployments {
        let status_color = match deployment.status {
            DeploymentStatus::Active => "active".green(),
            DeploymentStatus::Updating => "updating".yellow(),
            DeploymentStatus::Stopped => "stopped".dimmed(),
            DeploymentStatus::Failed => "failed".red(),
        };

        let name_with_branch = if let Some(branch) = &deployment.branch {
            format!("{} ({})", deployment.spec_name, branch.dimmed())
        } else {
            deployment.spec_name.clone()
        };

        println!("  {} [{}]", name_with_branch.bold(), status_color);
        println!("    WebSocket: {}", deployment.websocket_url.cyan());

        if let Some(version) = deployment.current_version {
            println!("    Version: v{}", version);
        }

        if let Some(deployed_at) = &deployment.last_deployed_at {
            println!("    Deployed: {}", deployed_at);
        }

        println!();
    }

    println!("Total: {} deployment(s)", deployments.len());

    Ok(())
}

/// Stop a deployment (placeholder - not yet implemented)
pub fn stop(_deployment_id: i32, _force: bool) -> Result<()> {
    bail!(
        "Stop deployment is not yet implemented.\n\n\
         To stop a deployment, you can:\n\
         1. Delete the spec: hyperstack spec delete <spec-name>\n\
         2. Or manually scale down in Kubernetes"
    );
}

/// Rollback to a previous deployment
///
/// This function finds a previous successful deployment and redeploys it.
/// For now, it always triggers a rebuild from the spec version.
///
/// Args:
/// - spec_name: Name of the spec to rollback
/// - to_version: Optional specific version number to rollback to
/// - build_id: Optional specific build ID to rollback to
/// - branch: Branch deployment name (default: "production")
/// - rebuild: Force rebuild (currently always true)
/// - watch: Whether to watch build progress
pub fn rollback(
    spec_name: &str,
    to_version: Option<i32>,
    build_id: Option<i32>,
    branch: &str,
    _rebuild: bool, // Currently always rebuilds
    watch: bool,
) -> Result<()> {
    let client = ApiClient::new()?;

    println!(
        "{} Starting rollback for '{}'...",
        "->".blue().bold(),
        spec_name
    );

    // Step 1: Find the spec
    let spec = client.get_spec_by_name(spec_name)?.ok_or_else(|| {
        anyhow::anyhow!(
            "Spec '{}' not found. Use 'hyperstack spec list' to see available specs.",
            spec_name
        )
    })?;

    println!("  Found spec (id={})", spec.id);

    // Step 2: Determine the target for rollback
    let target_version_id = if let Some(bid) = build_id {
        // Rollback to specific build
        println!("{} Looking up build #{}...", "->".blue().bold(), bid);

        let build_response = client.get_build(bid)?;
        let build = &build_response.build;

        // Verify build belongs to this spec
        if build.spec_id != Some(spec.id) {
            bail!(
                "Build #{} does not belong to spec '{}'. It belongs to spec_id {:?}",
                bid,
                spec_name,
                build.spec_id
            );
        }

        // Verify build was successful
        if build.status != BuildStatus::Completed {
            bail!(
                "Build #{} is not a successful deployment (status: {}). \
                 Cannot rollback to a failed or incomplete build.",
                bid,
                build.status
            );
        }

        build.spec_version_id.ok_or_else(|| {
            anyhow::anyhow!(
                "Build #{} has no spec_version_id. Cannot rollback without a version reference.",
                bid
            )
        })?
    } else if let Some(version) = to_version {
        // Rollback to specific version number
        println!("{} Looking up version {}...", "->".blue().bold(), version);

        let versions = client.list_spec_versions(spec.id)?;
        let ver = versions
            .iter()
            .find(|v| v.version_number == version)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Version {} not found for spec '{}'. Available versions: {:?}",
                    version,
                    spec_name,
                    versions
                        .iter()
                        .map(|v| v.version_number)
                        .collect::<Vec<_>>()
                )
            })?;

        println!(
            "  Found version {} (hash: {})",
            ver.version_number,
            &ver.content_hash[..12]
        );
        ver.id
    } else {
        // Find previous successful deployment
        println!(
            "{} Finding previous successful deployment...",
            "->".blue().bold()
        );

        // Get deployment history for this spec
        let builds = client.list_builds_filtered(Some(50), None, Some(spec.id))?;

        // Filter for completed builds and sort by created_at descending
        let mut successful_builds: Vec<&Build> = builds
            .iter()
            .filter(|b| b.status == BuildStatus::Completed && b.spec_version_id.is_some())
            .collect();

        // Sort by created_at descending (most recent first)
        successful_builds.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if successful_builds.is_empty() {
            bail!(
                "No successful deployments found for spec '{}'. Nothing to rollback to.",
                spec_name
            );
        }

        // Skip the current deployment (first one), get the previous one
        if successful_builds.len() < 2 {
            bail!(
                "Only one successful deployment found for spec '{}'. \
                 Need at least two deployments to rollback.",
                spec_name
            );
        }

        let current = successful_builds[0];
        let previous = successful_builds[1];

        println!(
            "  Current deployment: build #{} (version_id: {:?})",
            current.id, current.spec_version_id
        );
        println!(
            "  Rolling back to: build #{} (version_id: {:?})",
            previous.id, previous.spec_version_id
        );

        previous.spec_version_id.unwrap()
    };

    // Step 3: Create a new build with the target spec_version_id
    println!();
    println!("{} Creating rollback build...", "->".blue().bold());

    let branch_opt = if branch == "production" {
        None
    } else {
        Some(branch.to_string())
    };

    let req = CreateBuildRequest {
        spec_id: Some(spec.id),
        spec_version_id: Some(target_version_id),
        ast_payload: None,
        branch: branch_opt,
    };

    let response = client.create_build(req)?;

    println!(
        "{} Rollback build created (ID: {})",
        "✓".green().bold(),
        response.build_id
    );
    println!("  Status: {}", format_status(response.status));

    if watch {
        println!();
        return watch_build(&client, response.build_id);
    }

    println!();
    println!("Track progress with:");
    println!(
        "  {}",
        format!("hyperstack build status {} --watch", response.build_id).cyan()
    );

    Ok(())
}

/// Watch a build until it reaches a terminal state
fn watch_build(client: &ApiClient, build_id: i32) -> Result<()> {
    println!(
        "{} Watching rollback build #{}...\n",
        "->".blue().bold(),
        build_id
    );

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

        // Print status message if present
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
                    println!("{} Rollback completed successfully!", "✓".green().bold());

                    if let Some(ws_url) = &build.websocket_url {
                        println!();
                        println!("  WebSocket URL: {}", ws_url.cyan().bold());
                    }
                }
                BuildStatus::Failed => {
                    println!("{} Rollback failed!", "✗".red().bold());

                    if let Some(msg) = &build.status_message {
                        println!("  {}", msg);
                    }
                }
                BuildStatus::Cancelled => {
                    println!("{} Rollback was cancelled.", "!".yellow().bold());
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

fn chrono_now() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

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
