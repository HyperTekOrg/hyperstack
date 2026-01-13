//! # hyperstack-cli
//!
//! Command-line tool for building, deploying, and managing HyperStack
//! stream specifications.
//!
//! ## Installation
//!
//! ```bash
//! cargo install hyperstack-cli
//! ```
//!
//! ## Commands
//!
//! - `hs config init` - Initialize configuration
//! - `hs auth login` - Authenticate with HyperStack
//! - `hs spec push` - Push specs to cloud
//! - `hs build create` - Create a build from a spec
//! - `hs sdk create` - Generate TypeScript SDK
//!
//! See `hs --help` for the full command reference.

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::process;

mod api_client;
mod commands;
mod config;

#[derive(Parser)]
#[command(name = "hyperstack")]
#[command(about = "Hyperstack CLI - Build, deploy, and manage stream specifications", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to hyperstack.toml configuration file
    #[arg(short, long, global = true, default_value = "hyperstack.toml")]
    config: String,

    /// Output as JSON (machine-readable format)
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Hyperstack project (auto-detects AST files)
    Init,

    /// Deploy a spec: push, build, and watch until completion
    Up {
        /// Name of specific spec to deploy (deploys all if not specified)
        spec_name: Option<String>,

        /// Deploy to a specific branch (creates {spec-name}-{branch}.stack.usehyperstack.com)
        #[arg(short, long)]
        branch: Option<String>,

        /// Create a preview deployment with auto-generated branch name
        #[arg(long, conflicts_with = "branch")]
        preview: bool,
    },

    /// Show overview of specs, builds, and deployments
    Status,

    /// Push local specs to remote (alias for 'spec push')
    Push {
        /// Name of specific spec to push (pushes all if not specified)
        spec_name: Option<String>,
    },

    /// View build logs (alias for 'build logs')
    Logs {
        /// Build ID to show logs for (uses last build if not specified)
        build_id: Option<i32>,
    },

    /// SDK generation commands
    #[command(subcommand)]
    Sdk(SdkCommands),

    /// Configuration management commands
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Authentication commands
    #[command(subcommand)]
    Auth(AuthCommands),

    /// Spec sync commands
    #[command(subcommand)]
    Spec(SpecCommands),

    /// Build commands - compile and deploy specs
    #[command(subcommand)]
    Build(BuildCommands),

    /// Deployment commands - view and manage deployments
    #[command(subcommand)]
    Deploy(DeployCommands),
}

#[derive(Subcommand)]
enum SdkCommands {
    /// Create SDK from a spec
    #[command(subcommand)]
    Create(CreateCommands),

    /// List all available specs from hyperstack.toml
    List,
}

#[derive(Subcommand)]
enum CreateCommands {
    /// Generate TypeScript SDK
    Typescript {
        /// Name of the spec to generate SDK for
        spec_name: String,

        /// Output file path (overrides config)
        #[arg(short, long)]
        output: Option<String>,

        /// Package name for TypeScript
        #[arg(short, long)]
        package_name: Option<String>,
    },

    /// Generate Rust SDK crate
    Rust {
        /// Name of the spec to generate SDK for
        spec_name: String,

        /// Output directory path (overrides config)
        #[arg(short, long)]
        output: Option<String>,

        /// Crate name for generated Rust crate
        #[arg(long)]
        crate_name: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Initialize a new hyperstack.toml configuration file
    Init,

    /// Validate the configuration file
    Validate,
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Register a new account
    Register,

    /// Login to your account
    Login,

    /// Logout (remove stored credentials)
    Logout,

    /// Check authentication status (local only)
    Status,

    /// Verify authentication and show user info
    Whoami,
}

#[derive(Subcommand)]
enum SpecCommands {
    /// Push local specs with their AST to remote.
    /// Reads specs from `hyperstack.toml` and uploads AST from `.hyperstack/<entity>.ast.json`.
    Push {
        /// Name of specific spec to push (pushes all if not specified)
        spec_name: Option<String>,
    },

    /// Pull remote specs to local config
    Pull,

    /// List remote specs
    List,

    /// Show spec versions history
    Versions {
        /// Name of the spec
        spec_name: String,

        /// Maximum number of versions to show
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },

    /// Show detailed spec information
    Show {
        /// Name of the spec
        spec_name: String,

        /// Show specific version details
        #[arg(short, long)]
        version: Option<i32>,
    },

    /// Delete a spec from remote
    Delete {
        /// Name of the spec to delete
        spec_name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum BuildCommands {
    /// Create a new build from a spec (watches progress by default)
    Create {
        /// Name of the spec to build
        spec_name: String,

        /// Use specific version (default: latest)
        #[arg(short, long)]
        version: Option<i32>,

        /// Use local AST file directly instead of spec version
        #[arg(long)]
        ast_file: Option<String>,

        /// Don't wait for build to complete (return immediately)
        #[arg(long)]
        no_wait: bool,
    },

    /// List builds
    List {
        /// Maximum number of builds to show
        #[arg(short, long, default_value = "20")]
        limit: i64,

        /// Filter by status (pending, building, completed, failed, etc.)
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Get detailed build status
    Status {
        /// Build ID
        build_id: i32,

        /// Watch build progress until completion
        #[arg(short, long)]
        watch: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// View build logs
    Logs {
        /// Build ID
        build_id: i32,
    },
}

#[derive(Subcommand)]
enum DeployCommands {
    /// Show deployment info for a build
    Info {
        /// Build ID to show deployment info for
        build_id: i32,
    },

    /// List all active deployments
    List {
        /// Maximum number of deployments to show
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },

    /// Stop a deployment
    Stop {
        /// Deployment ID to stop
        deployment_id: i32,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Rollback to a previous deployment
    Rollback {
        /// Name of the spec to rollback
        spec_name: String,

        /// Rollback to specific version number (uses previous successful if not specified)
        #[arg(long)]
        to: Option<i32>,

        /// Rollback to specific build ID
        #[arg(long)]
        build: Option<i32>,

        /// Branch deployment to rollback (default: production)
        #[arg(long, default_value = "production")]
        branch: String,

        /// Force full rebuild instead of using existing image
        #[arg(long)]
        rebuild: bool,

        /// Don't watch the rollback progress
        #[arg(long)]
        no_wait: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init => commands::config::init(&cli.config),
        Commands::Up {
            spec_name,
            branch,
            preview,
        } => commands::up::up(&cli.config, spec_name.as_deref(), branch, preview),
        Commands::Status => commands::status::status(cli.json),
        Commands::Push { spec_name } => commands::spec::push(&cli.config, spec_name.as_deref()),
        Commands::Logs { build_id } => {
            let id = match build_id {
                Some(id) => id,
                None => {
                    // Fetch most recent build scoped to this project's specs
                    let client = api_client::ApiClient::new()?;

                    // Try to scope to local specs
                    let config = config::HyperstackConfig::load_optional(&cli.config)?;
                    let local_specs =
                        config::resolve_specs_to_push(config.as_ref(), None).unwrap_or_default();

                    let builds = if local_specs.is_empty() {
                        // No local specs found, fall back to all builds
                        client.list_builds(Some(1), None)?
                    } else {
                        // Find spec IDs for local specs
                        let mut all_builds = Vec::new();
                        for ast in &local_specs {
                            if let Ok(Some(spec)) = client.get_spec_by_name(&ast.spec_name) {
                                if let Ok(builds) =
                                    client.list_builds_filtered(Some(1), None, Some(spec.id))
                                {
                                    all_builds.extend(builds);
                                }
                            }
                        }
                        // Sort by created_at descending to get most recent
                        all_builds.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                        all_builds.truncate(1);
                        all_builds
                    };

                    builds.first().map(|b| b.id).ok_or_else(|| {
                        anyhow::anyhow!(
                            "No build ID specified and no builds found for this project.\n\
                             Usage: hyperstack logs <build-id>"
                        )
                    })?
                }
            };
            commands::build::logs(id)
        }
        Commands::Sdk(sdk_cmd) => match sdk_cmd {
            SdkCommands::Create(create_cmd) => match create_cmd {
                CreateCommands::Typescript {
                    spec_name,
                    output,
                    package_name,
                } => {
                    commands::sdk::create_typescript(&cli.config, &spec_name, output, package_name)
                }
                CreateCommands::Rust {
                    spec_name,
                    output,
                    crate_name,
                } => commands::sdk::create_rust(&cli.config, &spec_name, output, crate_name),
            },
            SdkCommands::List => commands::sdk::list(&cli.config),
        },
        Commands::Config(config_cmd) => match config_cmd {
            ConfigCommands::Init => commands::config::init(&cli.config),
            ConfigCommands::Validate => commands::config::validate(&cli.config),
        },
        Commands::Auth(auth_cmd) => match auth_cmd {
            AuthCommands::Register => commands::auth::register(),
            AuthCommands::Login => commands::auth::login(),
            AuthCommands::Logout => commands::auth::logout(),
            AuthCommands::Status => commands::auth::status(),
            AuthCommands::Whoami => commands::auth::whoami(),
        },
        Commands::Spec(spec_cmd) => match spec_cmd {
            SpecCommands::Push { spec_name } => {
                commands::spec::push(&cli.config, spec_name.as_deref())
            }
            SpecCommands::Pull => commands::spec::pull(&cli.config),
            SpecCommands::List => commands::spec::list_remote(cli.json),
            SpecCommands::Versions { spec_name, limit } => {
                commands::spec::versions(&spec_name, limit, cli.json)
            }
            SpecCommands::Show { spec_name, version } => commands::spec::show(&spec_name, version),
            SpecCommands::Delete { spec_name, force } => commands::spec::delete(&spec_name, force),
        },
        Commands::Build(build_cmd) => match build_cmd {
            BuildCommands::Create {
                spec_name,
                version,
                ast_file,
                no_wait,
            } => commands::build::create(
                &cli.config,
                &spec_name,
                version,
                ast_file.as_deref(),
                !no_wait,
            ),
            BuildCommands::List { limit, status } => {
                commands::build::list(limit, status.as_deref(), cli.json)
            }
            BuildCommands::Status {
                build_id,
                watch,
                json,
            } => commands::build::status(build_id, watch, json || cli.json),
            BuildCommands::Logs { build_id } => commands::build::logs(build_id),
        },
        Commands::Deploy(deploy_cmd) => match deploy_cmd {
            DeployCommands::Info { build_id } => commands::deploy::info(build_id, cli.json),
            DeployCommands::List { limit } => commands::deploy::list(limit, cli.json),
            DeployCommands::Stop {
                deployment_id,
                force,
            } => commands::deploy::stop(deployment_id, force),
            DeployCommands::Rollback {
                spec_name,
                to,
                build,
                branch,
                rebuild,
                no_wait,
            } => commands::deploy::rollback(&spec_name, to, build, &branch, rebuild, !no_wait),
        },
    }
}
