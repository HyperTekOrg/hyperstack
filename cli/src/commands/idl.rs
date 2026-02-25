use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use serde::Serialize;

use hyperstack_idl::discriminator::compute_discriminator;
use hyperstack_idl::parse::parse_idl_file;
use hyperstack_idl::search::{search_idl, suggest_similar, IdlSection, MatchType, SearchResult};
use hyperstack_idl::types::{
    IdlAccount, IdlField, IdlInstruction, IdlSpec, IdlType, IdlTypeArrayElement, IdlTypeDef,
    IdlTypeDefKind, IdlTypeDefinedInner,
};

/// Load and parse an IDL file, returning a clear error on failure.
fn load_idl(path: &Path) -> Result<IdlSpec> {
    parse_idl_file(path)
        .map_err(|e| anyhow::anyhow!("Failed to load IDL file '{}': {}", path.display(), e))
}

/// Print a value as pretty-printed JSON.
#[allow(dead_code)]
fn print_json<T: Serialize>(val: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(val)?);
    Ok(())
}

fn format_discriminator(bytes: &[u8]) -> String {
    let hex = bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{}]", hex)
}

fn instruction_discriminator(ix: &IdlInstruction) -> Vec<u8> {
    if !ix.discriminator.is_empty() {
        return ix.discriminator.clone();
    }

    if let Some(disc) = &ix.discriminant {
        let value = disc.value as u8;
        return vec![value, 0, 0, 0, 0, 0, 0, 0];
    }

    compute_discriminator("global", &ix.name).to_vec()
}

fn account_discriminator(account: &IdlAccount) -> Vec<u8> {
    if !account.discriminator.is_empty() {
        return account.discriminator.clone();
    }

    compute_discriminator("account", &account.name).to_vec()
}

fn format_idl_type(ty: &IdlType) -> String {
    match ty {
        IdlType::Simple(s) => s.clone(),
        IdlType::Array(arr) => {
            if arr.array.len() == 2 {
                let item = match &arr.array[0] {
                    IdlTypeArrayElement::Nested(t) => format_idl_type(t),
                    IdlTypeArrayElement::Type(t) => t.clone(),
                    IdlTypeArrayElement::Size(n) => n.to_string(),
                };
                let size = match &arr.array[1] {
                    IdlTypeArrayElement::Size(n) => n.to_string(),
                    IdlTypeArrayElement::Nested(t) => format_idl_type(t),
                    IdlTypeArrayElement::Type(t) => t.clone(),
                };
                format!("[{}; {}]", item, size)
            } else {
                let parts = arr
                    .array
                    .iter()
                    .map(|el| match el {
                        IdlTypeArrayElement::Nested(t) => format_idl_type(t),
                        IdlTypeArrayElement::Type(t) => t.clone(),
                        IdlTypeArrayElement::Size(n) => n.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("array({})", parts)
            }
        }
        IdlType::Option(o) => format!("Option<{}>", format_idl_type(&o.option)),
        IdlType::Vec(v) => format!("Vec<{}>", format_idl_type(&v.vec)),
        IdlType::HashMap(m) => format!(
            "HashMap<{}, {}>",
            format_idl_type(&m.hash_map.0),
            format_idl_type(&m.hash_map.1)
        ),
        IdlType::Defined(d) => match &d.defined {
            IdlTypeDefinedInner::Named { name } => name.clone(),
            IdlTypeDefinedInner::Simple(name) => name.clone(),
        },
    }
}

fn type_kind_label(kind: &IdlTypeDefKind) -> &'static str {
    match kind {
        IdlTypeDefKind::Struct { .. } => "Struct",
        IdlTypeDefKind::TupleStruct { .. } => "TupleStruct",
        IdlTypeDefKind::Enum { .. } => "Enum",
    }
}

fn type_member_count(kind: &IdlTypeDefKind) -> usize {
    match kind {
        IdlTypeDefKind::Struct { fields, .. } => fields.len(),
        IdlTypeDefKind::TupleStruct { fields, .. } => fields.len(),
        IdlTypeDefKind::Enum { variants, .. } => variants.len(),
    }
}

fn account_field_count(account: &IdlAccount) -> usize {
    account
        .type_def
        .as_ref()
        .map(type_member_count)
        .unwrap_or(0)
}

fn find_instruction<'a>(idl: &'a IdlSpec, name: &str) -> Option<&'a IdlInstruction> {
    idl.instructions
        .iter()
        .find(|ix| ix.name.eq_ignore_ascii_case(name))
}

fn find_account<'a>(idl: &'a IdlSpec, name: &str) -> Option<&'a IdlAccount> {
    idl.accounts
        .iter()
        .find(|account| account.name.eq_ignore_ascii_case(name))
}

fn find_type<'a>(idl: &'a IdlSpec, name: &str) -> Option<&'a IdlTypeDef> {
    idl.types
        .iter()
        .find(|type_def| type_def.name.eq_ignore_ascii_case(name))
}

fn not_found_error(section: &str, name: &str, candidates: &[String]) -> anyhow::Error {
    let candidate_refs = candidates.iter().map(String::as_str).collect::<Vec<_>>();
    if let Some(suggestion) = suggest_similar(name, &candidate_refs, 3).first() {
        anyhow::anyhow!(
            "{} '{}' not found, did you mean: {}?",
            section,
            name,
            suggestion.candidate
        )
    } else {
        anyhow::anyhow!("{} '{}' not found", section, name)
    }
}

fn print_fields(fields: &[IdlField]) {
    if fields.is_empty() {
        println!("  {}", "(none)".dimmed());
        return;
    }

    for field in fields {
        println!(
            "  {:<30} {}",
            field.name.green(),
            format_idl_type(&field.type_).cyan()
        );
    }
}

#[derive(Serialize)]
struct SearchResultJson {
    name: String,
    section: String,
    match_type: String,
}

fn format_section(section: &IdlSection) -> String {
    match section {
        IdlSection::Instruction => "instruction".to_string(),
        IdlSection::Account => "account".to_string(),
        IdlSection::Type => "type".to_string(),
        IdlSection::Error => "error".to_string(),
        IdlSection::Event => "event".to_string(),
        IdlSection::Constant => "constant".to_string(),
    }
}

fn format_match_type(mt: &MatchType) -> String {
    match mt {
        MatchType::Exact => "exact".to_string(),
        MatchType::CaseInsensitive => "case-insensitive".to_string(),
        MatchType::Contains => "contains".to_string(),
        MatchType::Fuzzy(d) => format!("fuzzy({})", d),
    }
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
            let idl = load_idl(path)?;
            let format = if idl.address.is_some() {
                "modern"
            } else {
                "legacy"
            };
            let address = idl
                .address
                .as_deref()
                .or_else(|| idl.metadata.as_ref().and_then(|m| m.address.as_deref()))
                .unwrap_or("-");

            println!("{}", "IDL Summary".bold());
            println!("  {} {}", "Name:".bold(), idl.get_name().green());
            println!("  {} {}", "Format:".bold(), format.cyan());
            println!("  {} {}", "Address:".bold(), address.yellow());
            println!("  {} {}", "Version:".bold(), idl.get_version().cyan());
            println!();
            println!("{}", "Counts".bold());
            println!("  {:<14} {}", "Instructions".bold(), idl.instructions.len());
            println!("  {:<14} {}", "Accounts".bold(), idl.accounts.len());
            println!("  {:<14} {}", "Types".bold(), idl.types.len());
            println!("  {:<14} {}", "Events".bold(), idl.events.len());
            println!("  {:<14} {}", "Errors".bold(), idl.errors.len());
            println!("  {:<14} {}", "Constants".bold(), idl.constants.len());
        }
        IdlCommands::Instructions { ref path, json } => {
            let idl = load_idl(path)?;
            if json {
                return print_json(&idl.instructions);
            }

            println!("{}", "Instructions".bold());
            println!(
                "  {:<32} {:>8} {:>6}  {}",
                "Name".bold(),
                "Accounts".bold(),
                "Args".bold(),
                "Discriminator".bold()
            );
            println!("  {}", "-".repeat(76).dimmed());
            for ix in &idl.instructions {
                let disc = format_discriminator(&instruction_discriminator(ix));
                println!(
                    "  {:<32} {:>8} {:>6}  {}",
                    ix.name.green(),
                    ix.accounts.len().to_string().cyan(),
                    ix.args.len().to_string().cyan(),
                    disc.yellow()
                );
            }
        }
        IdlCommands::Instruction {
            ref path,
            ref name,
            json,
        } => {
            let idl = load_idl(path)?;
            let ix = find_instruction(&idl, name).ok_or_else(|| {
                let candidates = idl
                    .instructions
                    .iter()
                    .map(|it| it.name.clone())
                    .collect::<Vec<_>>();
                not_found_error("instruction", name, &candidates)
            })?;

            if json {
                return print_json(ix);
            }

            println!("{} {}", "Instruction:".bold(), ix.name.green().bold());
            println!(
                "  {} {}",
                "Discriminator:".bold(),
                format_discriminator(&instruction_discriminator(ix)).yellow()
            );
            println!();
            println!("{}", "Accounts".bold());
            if ix.accounts.is_empty() {
                println!("  {}", "(none)".dimmed());
            } else {
                println!(
                    "  {:<26} {:<8} {:<8} {}",
                    "Name".bold(),
                    "Writable".bold(),
                    "Signer".bold(),
                    "PDA".bold()
                );
                println!("  {}", "-".repeat(70).dimmed());
                for account in &ix.accounts {
                    let pda = if account.pda.is_some() { "yes" } else { "no" };
                    println!(
                        "  {:<26} {:<8} {:<8} {}",
                        account.name.green(),
                        account.is_mut.to_string().cyan(),
                        account.is_signer.to_string().cyan(),
                        pda.yellow()
                    );
                }
            }

            println!();
            println!("{}", "Args".bold());
            print_fields(&ix.args);
        }
        IdlCommands::Accounts { ref path, json } => {
            let idl = load_idl(path)?;
            if json {
                return print_json(&idl.accounts);
            }

            println!("{}", "Accounts".bold());
            println!(
                "  {:<32} {:>8}  {}",
                "Name".bold(),
                "Fields".bold(),
                "Discriminator".bold()
            );
            println!("  {}", "-".repeat(70).dimmed());
            for account in &idl.accounts {
                println!(
                    "  {:<32} {:>8}  {}",
                    account.name.green(),
                    account_field_count(account).to_string().cyan(),
                    format_discriminator(&account_discriminator(account)).yellow()
                );
            }
        }
        IdlCommands::Account {
            ref path,
            ref name,
            json,
        } => {
            let idl = load_idl(path)?;
            let account = find_account(&idl, name).ok_or_else(|| {
                let candidates = idl
                    .accounts
                    .iter()
                    .map(|it| it.name.clone())
                    .collect::<Vec<_>>();
                not_found_error("account", name, &candidates)
            })?;

            if json {
                return print_json(account);
            }

            println!("{} {}", "Account:".bold(), account.name.green().bold());
            println!(
                "  {} {}",
                "Discriminator:".bold(),
                format_discriminator(&account_discriminator(account)).yellow()
            );
            println!("{}", "Fields".bold());
            match &account.type_def {
                Some(IdlTypeDefKind::Struct { fields, .. }) => print_fields(fields),
                Some(IdlTypeDefKind::TupleStruct { fields, .. }) => {
                    if fields.is_empty() {
                        println!("  {}", "(none)".dimmed());
                    } else {
                        for (idx, field_type) in fields.iter().enumerate() {
                            println!(
                                "  {:<30} {}",
                                format!("field_{}", idx).green(),
                                format_idl_type(field_type).cyan()
                            );
                        }
                    }
                }
                Some(IdlTypeDefKind::Enum { variants, .. }) => {
                    if variants.is_empty() {
                        println!("  {}", "(none)".dimmed());
                    } else {
                        for variant in variants {
                            println!("  {}", variant.name.green());
                        }
                    }
                }
                None => println!("  {}", "(no embedded fields in IDL account type)".dimmed()),
            }
        }
        IdlCommands::Types { ref path, json } => {
            let idl = load_idl(path)?;
            if json {
                return print_json(&idl.types);
            }

            println!("{}", "Types".bold());
            println!(
                "  {:<32} {:<12} {}",
                "Name".bold(),
                "Kind".bold(),
                "Fields/Variants".bold()
            );
            println!("  {}", "-".repeat(60).dimmed());
            for type_def in &idl.types {
                println!(
                    "  {:<32} {:<12} {}",
                    type_def.name.green(),
                    type_kind_label(&type_def.type_def).cyan(),
                    type_member_count(&type_def.type_def).to_string().yellow()
                );
            }
        }
        IdlCommands::Type {
            ref path,
            ref name,
            json,
        } => {
            let idl = load_idl(path)?;
            let type_def = find_type(&idl, name).ok_or_else(|| {
                let candidates = idl
                    .types
                    .iter()
                    .map(|it| it.name.clone())
                    .collect::<Vec<_>>();
                not_found_error("type", name, &candidates)
            })?;

            if json {
                return print_json(type_def);
            }

            println!("{} {}", "Type:".bold(), type_def.name.green().bold());
            println!(
                "  {} {}",
                "Kind:".bold(),
                type_kind_label(&type_def.type_def).cyan()
            );
            match &type_def.type_def {
                IdlTypeDefKind::Struct { fields, .. } => {
                    println!("{}", "Fields".bold());
                    print_fields(fields);
                }
                IdlTypeDefKind::TupleStruct { fields, .. } => {
                    println!("{}", "Tuple Fields".bold());
                    if fields.is_empty() {
                        println!("  {}", "(none)".dimmed());
                    } else {
                        for (idx, field_type) in fields.iter().enumerate() {
                            println!(
                                "  {:<30} {}",
                                format!("item_{}", idx).green(),
                                format_idl_type(field_type).cyan()
                            );
                        }
                    }
                }
                IdlTypeDefKind::Enum { variants, .. } => {
                    println!("{}", "Variants".bold());
                    if variants.is_empty() {
                        println!("  {}", "(none)".dimmed());
                    } else {
                        for variant in variants {
                            println!("  {}", variant.name.green());
                        }
                    }
                }
            }
        }
        IdlCommands::Errors { ref path, json } => {
            let idl = load_idl(path)?;
            if json {
                return print_json(&idl.errors);
            }

            println!("{}", "Errors".bold());
            if idl.errors.is_empty() {
                println!("  {}", "(none)".dimmed());
            } else {
                println!(
                    "  {:<8} {:<32} {}",
                    "Code".bold(),
                    "Name".bold(),
                    "Message".bold()
                );
                println!("  {}", "-".repeat(70).dimmed());
                for error in &idl.errors {
                    let msg = error.msg.as_deref().unwrap_or("-");
                    println!(
                        "  {:<8} {:<32} {}",
                        error.code.to_string().cyan(),
                        error.name.green(),
                        msg.yellow()
                    );
                }
            }
        }
        IdlCommands::Events { ref path, json } => {
            let idl = load_idl(path)?;
            if json {
                return print_json(&idl.events);
            }

            println!("{}", "Events".bold());
            if idl.events.is_empty() {
                println!("  {}", "(none)".dimmed());
            } else {
                println!(
                    "  {:<32} {}",
                    "Name".bold(),
                    "Discriminator".bold()
                );
                println!("  {}", "-".repeat(60).dimmed());
                for event in &idl.events {
                    let disc = format_discriminator(&event.get_discriminator());
                    println!(
                        "  {:<32} {}",
                        event.name.green(),
                        disc.yellow()
                    );
                }
            }
        }
        IdlCommands::Constants { ref path, json } => {
            let idl = load_idl(path)?;
            if json {
                return print_json(&idl.constants);
            }

            println!("{}", "Constants".bold());
            if idl.constants.is_empty() {
                println!("  {}", "(none)".dimmed());
            } else {
                println!(
                    "  {:<32} {:<16} {}",
                    "Name".bold(),
                    "Type".bold(),
                    "Value".bold()
                );
                println!("  {}", "-".repeat(70).dimmed());
                for constant in &idl.constants {
                    println!(
                        "  {:<32} {:<16} {}",
                        constant.name.green(),
                        format_idl_type(&constant.type_).cyan(),
                        constant.value.yellow()
                    );
                }
            }
        }
        IdlCommands::Search {
            ref path,
            ref query,
            json,
        } => {
            let idl = load_idl(path)?;
            let results = search_idl(&idl, query);

            if json {
                let json_results: Vec<SearchResultJson> = results
                    .iter()
                    .map(|r| SearchResultJson {
                        name: r.name.clone(),
                        section: format_section(&r.section),
                        match_type: format_match_type(&r.match_type),
                    })
                    .collect();
                return print_json(&json_results);
            }

            if results.is_empty() {
                println!("  {} '{}'", "No results found for".dimmed(), query);
            } else {
                // Group by section
                let sections = [
                    ("Instructions", IdlSection::Instruction),
                    ("Accounts", IdlSection::Account),
                    ("Types", IdlSection::Type),
                    ("Errors", IdlSection::Error),
                    ("Events", IdlSection::Event),
                    ("Constants", IdlSection::Constant),
                ];
                for (label, section) in &sections {
                    let section_results: Vec<&SearchResult> = results
                        .iter()
                        .filter(|r| std::mem::discriminant(&r.section) == std::mem::discriminant(section))
                        .collect();
                    if !section_results.is_empty() {
                        println!("{}", label.bold());
                        for r in &section_results {
                            println!(
                                "  {} {}",
                                r.name.green(),
                                format!("({})", format_match_type(&r.match_type)).dimmed()
                            );
                        }
                        println!();
                    }
                }
            }
        }
        IdlCommands::Discriminator {
            ref path,
            ref name,
            json,
        } => {
            let idl = load_idl(path)?;

            #[derive(Serialize)]
            struct DiscriminatorResult {
                name: String,
                namespace: String,
                hex: String,
                bytes: Vec<u8>,
            }

            let mut results: Vec<DiscriminatorResult> = Vec::new();

            // Check instructions
            if let Some(ix) = find_instruction(&idl, name) {
                let disc = instruction_discriminator(ix);
                results.push(DiscriminatorResult {
                    name: ix.name.clone(),
                    namespace: "global".to_string(),
                    hex: format_discriminator(&disc),
                    bytes: disc,
                });
            }

            // Check accounts
            if let Some(acc) = find_account(&idl, name) {
                let disc = account_discriminator(acc);
                results.push(DiscriminatorResult {
                    name: acc.name.clone(),
                    namespace: "account".to_string(),
                    hex: format_discriminator(&disc),
                    bytes: disc,
                });
            }

            if results.is_empty() {
                let mut candidates: Vec<String> = idl
                    .instructions
                    .iter()
                    .map(|ix| ix.name.clone())
                    .collect();
                candidates.extend(idl.accounts.iter().map(|a| a.name.clone()));
                return Err(not_found_error("instruction or account", name, &candidates));
            }

            if json {
                return print_json(&results);
            }

            for r in &results {
                println!("{} {}", "Name:".bold(), r.name.green());
                println!("{} {}", "Namespace:".bold(), r.namespace.cyan());
                println!("{} {}", "Discriminator:".bold(), r.hex.yellow());
                println!(
                    "{} {:?}",
                    "Bytes:".bold(),
                    r.bytes
                );
                println!();
            }
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
