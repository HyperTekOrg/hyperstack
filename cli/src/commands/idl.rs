use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
#[allow(dead_code)]
use serde::Serialize;

use hyperstack_idl::parse::parse_idl_file;
use hyperstack_idl::types::IdlSpec;

/// Load and parse an IDL file, returning a clear error on failure.
fn load_idl(path: &Path) -> Result<IdlSpec> {
    parse_idl_file(path).map_err(|e| anyhow::anyhow!("Failed to load IDL file '{}': {}", path.display(), e))
}

/// Print a value as pretty-printed JSON.
#[allow(dead_code)]
fn print_json<T: Serialize>(val: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(val)?);
    Ok(())
}

#[derive(Parser)]
#[command(about = "Inspect and analyze Anchor/Shank IDL files")]
pub struct IdlArgs {
    #[command(subcommand)]
    pub command: IdlCommands,
}

#[derive(Subcommand)]
pub enum IdlCommands {
    /// Show a high-level summary of the IDL (instruction count, accounts, types, etc.)
    Summary {
        /// Path to the IDL JSON file
        path: PathBuf,
    },

    /// List all instructions
    Instructions {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show details for a single instruction
    Instruction {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Instruction name
        name: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List all account types
    Accounts {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show details for a single account type
    Account {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Account name
        name: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List all type definitions
    Types {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show details for a single type definition
    Type {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Type name
        name: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List all error codes
    Errors {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List all events
    Events {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List all constants
    Constants {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Fuzzy-search instructions, accounts, types, and errors
    Search {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Search query
        query: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Compute the Anchor discriminator for an instruction or account
    Discriminator {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Instruction or account name
        name: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show cross-instruction account relationships
    Relations {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show which instructions use a given account
    AccountUsage {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Account name
        name: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show instructions that link two accounts together
    Links {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// First account name
        a: String,

        /// Second account name
        b: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Extract PDA seed graph from instructions
    PdaGraph {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Extract type-level pubkey reference graph
    TypeGraph {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Find how a new account connects to existing accounts
    Connect {
        /// Path to the IDL JSON file
        path: PathBuf,

        /// New account name to connect
        new_account: String,

        /// Existing account names (comma-separated)
        #[arg(long, value_delimiter = ',')]
        existing: Vec<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Suggest HyperStack integration points
        #[arg(long)]
        suggest_hs: bool,
    },
}

pub fn run(args: IdlArgs) -> Result<()> {
    match args.command {
        IdlCommands::Summary { ref path } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement summary");
        }
        IdlCommands::Instructions { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement instructions");
        }
        IdlCommands::Instruction { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement instruction");
        }
        IdlCommands::Accounts { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement accounts");
        }
        IdlCommands::Account { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement account");
        }
        IdlCommands::Types { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement types");
        }
        IdlCommands::Type { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement type");
        }
        IdlCommands::Errors { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement errors");
        }
        IdlCommands::Events { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement events");
        }
        IdlCommands::Constants { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement constants");
        }
        IdlCommands::Search { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement search");
        }
        IdlCommands::Discriminator { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement discriminator");
        }
        IdlCommands::Relations { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement relations");
        }
        IdlCommands::AccountUsage { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement account-usage");
        }
        IdlCommands::Links { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement links");
        }
        IdlCommands::PdaGraph { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement pda-graph");
        }
        IdlCommands::TypeGraph { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement type-graph");
        }
        IdlCommands::Connect { ref path, .. } => {
            let _idl = load_idl(path)?;
            println!("TODO: implement connect");
        }
    }

    Ok(())
}
