//! # hyperstack-cli
//!
//! Command-line tool for building, deploying, and managing HyperStack
//! stream stacks.
//!
//! ## Installation
//!
//! ```bash
//! cargo install hyperstack-cli
//! ```
//!
//! ## Commands
//!
//! - `hs init` - Initialize configuration
//! - `hs up [stack]` - Deploy a stack (push + build + deploy)
//! - `hs stack list` - List all stacks
//! - `hs stack show` - Show stack details
//! - `hs sdk create` - Generate TypeScript/Rust SDK
//!
//! See `hs --help` for the full command reference.

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use std::io;
use std::process;

mod api_client;
mod commands;
mod config;
mod ui;

#[derive(Parser)]
#[command(name = "hs")]
#[command(about = "Hyperstack CLI - Build, deploy, and manage stream stacks", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to hyperstack.toml configuration file
    #[arg(short, long, global = true, default_value = "hyperstack.toml")]
    config: String,

    /// Output as JSON (machine-readable format)
    #[arg(long, global = true)]
    json: bool,

    /// Enable verbose output
    #[arg(long, global = true)]
    verbose: bool,

    /// Generate shell completions
    #[arg(long, value_name = "SHELL")]
    completions: Option<Shell>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Hyperstack project (auto-detects AST files)
    Init,

    /// Deploy a stack: push, build, and watch until completion
    Up {
        /// Name of specific stack to deploy (deploys all if not specified)
        stack_name: Option<String>,

        /// Deploy to a specific branch (creates {stack-name}-{branch}.stack.usehyperstack.com)
        #[arg(short, long)]
        branch: Option<String>,

        /// Create a preview deployment with auto-generated branch name
        #[arg(long, conflicts_with = "branch")]
        preview: bool,

        /// Show what would be deployed without actually deploying
        #[arg(long)]
        dry_run: bool,
    },

    /// Show overview of stacks, builds, and deployments
    Status,

    /// Push local stacks to remote (alias for 'stack push')
    Push {
        /// Name of specific stack to push (pushes all if not specified)
        stack_name: Option<String>,
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

    /// Stack management commands - manage your deployed stacks
    #[command(subcommand)]
    Stack(StackCommands),

    /// Build commands (advanced) - low-level build management
    #[command(subcommand, hide = true)]
    Build(BuildCommands),
}

#[derive(Subcommand)]
enum SdkCommands {
    /// Create SDK from a stack
    #[command(subcommand)]
    Create(CreateCommands),

    /// List all available stacks from hyperstack.toml
    List,
}

#[derive(Subcommand)]
enum CreateCommands {
    /// Generate TypeScript SDK
    Typescript {
        /// Name of the stack to generate SDK for
        stack_name: String,

        /// Output file path (overrides config)
        #[arg(short, long)]
        output: Option<String>,

        /// Package name for TypeScript
        #[arg(short, long)]
        package_name: Option<String>,
    },

    /// Generate Rust SDK crate
    Rust {
        /// Name of the stack to generate SDK for
        stack_name: String,

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
enum StackCommands {
    /// List all stacks with their deployment status
    List,

    /// Push local stacks with their AST to remote
    Push {
        /// Name of specific stack to push (pushes all if not specified)
        stack_name: Option<String>,
    },

    /// Show detailed stack information including deployment status and versions
    Show {
        /// Name of the stack
        stack_name: String,

        /// Show specific version details
        #[arg(short, long)]
        version: Option<i32>,
    },

    /// Show version history for a stack
    Versions {
        /// Name of the stack
        stack_name: String,

        /// Maximum number of versions to show
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },

    /// Delete a stack from remote
    Delete {
        /// Name of the stack to delete
        stack_name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Rollback to a previous deployment
    Rollback {
        /// Name of the stack to rollback
        stack_name: String,

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

    /// Stop a deployment
    Stop {
        /// Name of the stack to stop
        stack_name: String,

        /// Branch deployment to stop (default: production)
        #[arg(long)]
        branch: Option<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

/// Build commands - advanced low-level build management
/// These are power-user commands; most users should use `hs up` instead.
#[derive(Subcommand)]
enum BuildCommands {
    /// Create a new build from a stack (watches progress by default)
    Create {
        /// Name of the stack to build
        stack_name: String,

        /// Use specific version (default: latest)
        #[arg(short, long)]
        version: Option<i32>,

        /// Use local AST file directly instead of stack version
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
}

fn main() {
    let cli = Cli::parse();

    if let Some(shell) = cli.completions {
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "hs", &mut io::stdout());
        return;
    }

    if let Err(e) = run(cli) {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        return Ok(());
    };

    match command {
        Commands::Init => commands::config::init(&cli.config),
        Commands::Up {
            stack_name,
            branch,
            preview,
            dry_run,
        } => commands::up::up(&cli.config, stack_name.as_deref(), branch, preview, dry_run),
        Commands::Status => commands::status::status(cli.json),
        Commands::Push { stack_name } => commands::stack::push(&cli.config, stack_name.as_deref()),
        Commands::Sdk(sdk_cmd) => match sdk_cmd {
            SdkCommands::Create(create_cmd) => match create_cmd {
                CreateCommands::Typescript {
                    stack_name,
                    output,
                    package_name,
                } => {
                    commands::sdk::create_typescript(&cli.config, &stack_name, output, package_name)
                }
                CreateCommands::Rust {
                    stack_name,
                    output,
                    crate_name,
                } => commands::sdk::create_rust(&cli.config, &stack_name, output, crate_name),
            },
            SdkCommands::List => commands::sdk::list(&cli.config),
        },
        Commands::Config(config_cmd) => match config_cmd {
            ConfigCommands::Validate => commands::config::validate(&cli.config),
        },
        Commands::Auth(auth_cmd) => match auth_cmd {
            AuthCommands::Register => commands::auth::register(),
            AuthCommands::Login => commands::auth::login(),
            AuthCommands::Logout => commands::auth::logout(),
            AuthCommands::Status => commands::auth::status(),
            AuthCommands::Whoami => commands::auth::whoami(),
        },
        Commands::Stack(stack_cmd) => match stack_cmd {
            StackCommands::List => commands::stack::list(cli.json),
            StackCommands::Push { stack_name } => {
                commands::stack::push(&cli.config, stack_name.as_deref())
            }
            StackCommands::Show {
                stack_name,
                version,
            } => commands::stack::show(&stack_name, version, cli.json),
            StackCommands::Versions { stack_name, limit } => {
                commands::stack::versions(&stack_name, limit, cli.json)
            }
            StackCommands::Delete { stack_name, force } => {
                commands::stack::delete(&stack_name, force)
            }
            StackCommands::Rollback {
                stack_name,
                to,
                build,
                branch,
                rebuild,
                no_wait,
            } => commands::stack::rollback(&stack_name, to, build, &branch, rebuild, !no_wait),
            StackCommands::Stop {
                stack_name,
                branch,
                force,
            } => commands::stack::stop(&stack_name, branch.as_deref(), force),
        },
        Commands::Build(build_cmd) => match build_cmd {
            BuildCommands::Create {
                stack_name,
                version,
                ast_file,
                no_wait,
            } => commands::build::create(
                &cli.config,
                &stack_name,
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
        },
    }
}
