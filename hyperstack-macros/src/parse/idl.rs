//! IDL file parsing.
//!
//! Parses Anchor IDL JSON files into Rust structures for code generation.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

fn anchor_discriminator(preimage: &str) -> Vec<u8> {
    let hash = Sha256::digest(preimage.as_bytes());
    hash[..8].to_vec()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlSpec {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    pub instructions: Vec<IdlInstruction>,
    pub accounts: Vec<IdlAccount>,
    #[serde(default)]
    pub types: Vec<IdlTypeDef>,
    #[serde(default)]
    pub events: Vec<IdlEvent>,
    #[serde(default)]
    pub errors: Vec<IdlError>,
    pub metadata: Option<IdlMetadata>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlMetadata {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub spec: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub origin: Option<String>,
}

/// Steel-style discriminant format: {"type": "u8", "value": N}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SteelDiscriminant {
    #[serde(rename = "type")]
    pub type_: String,
    pub value: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlInstruction {
    pub name: String,
    /// Anchor-style discriminator: 8-byte array
    #[serde(default)]
    pub discriminator: Vec<u8>,
    /// Steel-style discriminant: {"type": "u8", "value": N}
    #[serde(default)]
    pub discriminant: Option<SteelDiscriminant>,
    #[serde(default)]
    pub docs: Vec<String>,
    pub accounts: Vec<IdlAccountArg>,
    pub args: Vec<IdlField>,
}

impl IdlInstruction {
    pub fn get_discriminator(&self) -> Vec<u8> {
        if !self.discriminator.is_empty() {
            return self.discriminator.clone();
        }

        if let Some(disc) = &self.discriminant {
            let value = disc.value as u8;
            return vec![value, 0, 0, 0, 0, 0, 0, 0];
        }

        // Legacy IDLs omit discriminators — derive from Anchor convention:
        // sha256("global:<instruction_name>")[0..8]
        anchor_discriminator(&format!("global:{}", self.name))
    }
}

/// PDA definition in Anchor IDL format
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlPda {
    pub seeds: Vec<IdlPdaSeed>,
    #[serde(default)]
    pub program: Option<IdlPdaProgram>,
}

/// PDA seed in Anchor IDL format
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum IdlPdaSeed {
    /// Constant byte array seed
    Const { value: Vec<u8> },
    /// Reference to another account in the instruction
    Account {
        path: String,
        #[serde(default)]
        account: Option<String>,
    },
    /// Reference to an instruction argument
    Arg {
        path: String,
        #[serde(rename = "type", default)]
        arg_type: Option<String>,
    },
}

/// Program reference for cross-program PDAs
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum IdlPdaProgram {
    /// Reference to another account that holds the program ID
    Account { kind: String, path: String },
    /// Literal program ID
    Literal { kind: String, value: String },
    /// Constant program ID as bytes
    Const { kind: String, value: Vec<u8> },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlAccountArg {
    pub name: String,
    #[serde(rename = "isMut", alias = "writable", default)]
    pub is_mut: bool,
    #[serde(rename = "isSigner", alias = "signer", default)]
    pub is_signer: bool,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub docs: Vec<String>,
    #[serde(default)]
    pub pda: Option<IdlPda>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlAccount {
    pub name: String,
    #[serde(default)]
    pub discriminator: Vec<u8>,
    #[serde(default)]
    pub docs: Vec<String>,
    /// Steel format embedded type definition
    #[serde(rename = "type", default)]
    pub type_def: Option<IdlTypeDefKind>,
}

impl IdlAccount {
    pub fn get_discriminator(&self) -> Vec<u8> {
        if !self.discriminator.is_empty() {
            return self.discriminator.clone();
        }

        // Legacy IDLs omit discriminators — derive from Anchor convention:
        // sha256("account:<AccountName>")[0..8]
        anchor_discriminator(&format!("account:{}", self.name))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlTypeDefStruct {
    pub kind: String,
    pub fields: Vec<IdlField>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlField {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: IdlType,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum IdlType {
    Simple(String),
    Array(IdlTypeArray),
    Option(IdlTypeOption),
    Vec(IdlTypeVec),
    HashMap(IdlTypeHashMap),
    Defined(IdlTypeDefined),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlTypeOption {
    pub option: Box<IdlType>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlTypeVec {
    pub vec: Box<IdlType>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlTypeHashMap {
    #[serde(alias = "bTreeMap")]
    #[serde(rename = "hashMap")]
    pub hash_map: (Box<IdlType>, Box<IdlType>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlTypeArray {
    pub array: Vec<IdlTypeArrayElement>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum IdlTypeArrayElement {
    Nested(IdlType),
    Type(String),
    Size(u32),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlTypeDefined {
    pub defined: IdlTypeDefinedInner,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum IdlTypeDefinedInner {
    Named { name: String },
    Simple(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlRepr {
    pub kind: String,
}

/// Account serialization format as specified in the IDL.
/// Defaults to Borsh when not specified.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IdlSerialization {
    #[default]
    Borsh,
    Bytemuck,
    #[serde(alias = "bytemuckunsafe")]
    BytemuckUnsafe,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlTypeDef {
    pub name: String,
    #[serde(default)]
    pub docs: Vec<String>,
    /// Serialization format: "borsh" (default), "bytemuck", or "bytemuckunsafe"
    #[serde(default)]
    pub serialization: Option<IdlSerialization>,
    /// Repr annotation for zero-copy types (e.g., {"kind": "c"})
    #[serde(default)]
    pub repr: Option<IdlRepr>,
    #[serde(rename = "type")]
    pub type_def: IdlTypeDefKind,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum IdlTypeDefKind {
    Struct {
        kind: String,
        fields: Vec<IdlField>,
    },
    TupleStruct {
        kind: String,
        fields: Vec<IdlType>,
    },
    Enum {
        kind: String,
        variants: Vec<IdlEnumVariant>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlEnumVariant {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlEvent {
    pub name: String,
    #[serde(default)]
    pub discriminator: Vec<u8>,
    #[serde(default)]
    pub docs: Vec<String>,
}

impl IdlEvent {
    pub fn get_discriminator(&self) -> Vec<u8> {
        if !self.discriminator.is_empty() {
            return self.discriminator.clone();
        }
        anchor_discriminator(&format!("event:{}", self.name))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdlError {
    pub code: u32,
    pub name: String,
    #[serde(default)]
    pub msg: Option<String>,
}

pub fn parse_idl_file<P: AsRef<Path>>(path: P) -> Result<IdlSpec, String> {
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read IDL file {:?}: {}", path.as_ref(), e))?;

    parse_idl_content(&content)
}

pub fn parse_idl_content(content: &str) -> Result<IdlSpec, String> {
    serde_json::from_str(content).map_err(|e| format!("Failed to parse IDL JSON: {}", e))
}

impl IdlSpec {
    pub fn get_name(&self) -> &str {
        self.name
            .as_deref()
            .or_else(|| self.metadata.as_ref().and_then(|m| m.name.as_deref()))
            .unwrap_or("unknown")
    }

    pub fn get_version(&self) -> &str {
        self.version
            .as_deref()
            .or_else(|| self.metadata.as_ref().and_then(|m| m.version.as_deref()))
            .unwrap_or("0.1.0")
    }

    /// Check if a field is an account (vs an arg/data field) for a given instruction
    /// Returns Some("accounts") if it's an account, Some("data") if it's an arg, None if not found
    pub fn get_instruction_field_prefix(
        &self,
        instruction_name: &str,
        field_name: &str,
    ) -> Option<&'static str> {
        // Normalize instruction name to snake_case for comparison
        // IDL uses snake_case (e.g., "create_v2") but code uses PascalCase (e.g., "CreateV2")
        let normalized_name = to_snake_case(instruction_name);

        for instruction in &self.instructions {
            if instruction.name == normalized_name {
                // Check if it's an account
                for account in &instruction.accounts {
                    if account.name == field_name {
                        return Some("accounts");
                    }
                }
                // Check if it's an arg (instruction data)
                for arg in &instruction.args {
                    if arg.name == field_name {
                        return Some("data");
                    }
                }
                // Field not found in this instruction
                return None;
            }
        }
        // Instruction not found
        None
    }

    /// Get the discriminator bytes for an instruction by name
    pub fn get_instruction_discriminator(&self, instruction_name: &str) -> Option<Vec<u8>> {
        let normalized_name = to_snake_case(instruction_name);
        for instruction in &self.instructions {
            if instruction.name == normalized_name {
                let disc = instruction.get_discriminator();
                if !disc.is_empty() {
                    return Some(disc);
                }
            }
        }
        None
    }
}

impl IdlType {
    pub fn to_rust_type_string(&self) -> String {
        match self {
            IdlType::Simple(s) => map_simple_type(s),
            IdlType::Array(arr) => {
                if arr.array.len() == 2 {
                    match (&arr.array[0], &arr.array[1]) {
                        (IdlTypeArrayElement::Type(t), IdlTypeArrayElement::Size(size)) => {
                            format!("[{}; {}]", map_simple_type(t), size)
                        }
                        (IdlTypeArrayElement::Nested(nested), IdlTypeArrayElement::Size(size)) => {
                            let inner = nested.to_rust_type_string();
                            format!("[{}; {}]", inner, size)
                        }
                        _ => "Vec<u8>".to_string(),
                    }
                } else {
                    "Vec<u8>".to_string()
                }
            }
            IdlType::Defined(def) => match &def.defined {
                IdlTypeDefinedInner::Named { name } => name.clone(),
                IdlTypeDefinedInner::Simple(s) => s.clone(),
            },
            IdlType::Option(opt) => {
                let inner_type = opt.option.to_rust_type_string();
                format!("Option<{}>", inner_type)
            }
            IdlType::Vec(vec) => {
                let inner_type = vec.vec.to_rust_type_string();
                format!("Vec<{}>", inner_type)
            }
            IdlType::HashMap(hm) => {
                let key_type = hm.hash_map.0.to_rust_type_string();
                let val_type = hm.hash_map.1.to_rust_type_string();
                format!("std::collections::HashMap<{}, {}>", key_type, val_type)
            }
        }
    }

    pub fn to_rust_type_string_bytemuck(&self) -> String {
        match self {
            IdlType::Simple(s) => map_simple_type_bytemuck(s),
            IdlType::Array(arr) => {
                if arr.array.len() == 2 {
                    match (&arr.array[0], &arr.array[1]) {
                        (IdlTypeArrayElement::Type(t), IdlTypeArrayElement::Size(size)) => {
                            format!("[{}; {}]", map_simple_type_bytemuck(t), size)
                        }
                        (IdlTypeArrayElement::Nested(nested), IdlTypeArrayElement::Size(size)) => {
                            let inner = nested.to_rust_type_string_bytemuck();
                            format!("[{}; {}]", inner, size)
                        }
                        _ => "Vec<u8>".to_string(),
                    }
                } else {
                    "Vec<u8>".to_string()
                }
            }
            IdlType::Defined(def) => match &def.defined {
                IdlTypeDefinedInner::Named { name } => name.clone(),
                IdlTypeDefinedInner::Simple(s) => s.clone(),
            },
            IdlType::Option(opt) => {
                let inner_type = opt.option.to_rust_type_string_bytemuck();
                format!("Option<{}>", inner_type)
            }
            IdlType::Vec(vec) => {
                let inner_type = vec.vec.to_rust_type_string_bytemuck();
                format!("Vec<{}>", inner_type)
            }
            IdlType::HashMap(hm) => {
                let key_type = hm.hash_map.0.to_rust_type_string_bytemuck();
                let val_type = hm.hash_map.1.to_rust_type_string_bytemuck();
                format!("std::collections::HashMap<{}, {}>", key_type, val_type)
            }
        }
    }
}

fn map_simple_type(idl_type: &str) -> String {
    match idl_type {
        "u8" => "u8".to_string(),
        "u16" => "u16".to_string(),
        "u32" => "u32".to_string(),
        "u64" => "u64".to_string(),
        "u128" => "u128".to_string(),
        "i8" => "i8".to_string(),
        "i16" => "i16".to_string(),
        "i32" => "i32".to_string(),
        "i64" => "i64".to_string(),
        "i128" => "i128".to_string(),
        "bool" => "bool".to_string(),
        "string" => "String".to_string(),
        "publicKey" | "pubkey" => "solana_pubkey::Pubkey".to_string(),
        "bytes" => "Vec<u8>".to_string(),
        _ => idl_type.to_string(),
    }
}

fn map_simple_type_bytemuck(idl_type: &str) -> String {
    match idl_type {
        "u8" => "u8".to_string(),
        "u16" => "u16".to_string(),
        "u32" => "u32".to_string(),
        "u64" => "u64".to_string(),
        "u128" => "u128".to_string(),
        "i8" => "i8".to_string(),
        "i16" => "i16".to_string(),
        "i32" => "i32".to_string(),
        "i64" => "i64".to_string(),
        "i128" => "i128".to_string(),
        // bool is NOT Pod-safe in bytemuck (not all bit patterns are valid).
        // Map to u8 instead: 0 = false, non-zero = true.
        "bool" => "u8".to_string(),
        "string" => "String".to_string(),
        "publicKey" | "pubkey" => "[u8; 32]".to_string(),
        "bytes" => "Vec<u8>".to_string(),
        _ => idl_type.to_string(),
    }
}

pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for c in s.chars() {
        if c.is_uppercase() {
            if !result.is_empty() {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }

    result
}

pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchor_discriminator_known_values() {
        let disc = anchor_discriminator("global:initialize");
        assert_eq!(disc.len(), 8);
        assert_eq!(disc, &Sha256::digest(b"global:initialize")[..8]);
    }

    #[test]
    fn test_anchor_account_discriminator() {
        let disc = anchor_discriminator("account:LendingMarket");
        assert_eq!(disc.len(), 8);
        assert_eq!(disc, &Sha256::digest(b"account:LendingMarket")[..8]);
    }

    #[test]
    fn test_legacy_idl_parses_without_discriminator() {
        let json = r#"{
            "address": "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
            "version": "0.3.0",
            "name": "raydium_amm",
            "instructions": [
                {
                    "name": "initialize",
                    "accounts": [
                        { "name": "tokenProgram", "isMut": false, "isSigner": false }
                    ],
                    "args": [
                        { "name": "nonce", "type": "u8" }
                    ]
                }
            ],
            "accounts": [
                {
                    "name": "TargetOrders",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "owner", "type": { "array": ["u64", 4] } }
                        ]
                    }
                }
            ],
            "types": [],
            "errors": []
        }"#;
        let idl = parse_idl_content(json).expect("legacy IDL should parse");

        assert_eq!(idl.instructions.len(), 1);
        assert_eq!(idl.accounts.len(), 1);
        assert!(idl.accounts[0].discriminator.is_empty());
        assert!(idl.instructions[0].discriminator.is_empty());
        assert!(idl.instructions[0].discriminant.is_none());
    }

    #[test]
    fn test_legacy_instruction_computes_discriminator() {
        let json = r#"{
            "name": "raydium_amm",
            "instructions": [
                {
                    "name": "initialize",
                    "accounts": [],
                    "args": []
                }
            ],
            "accounts": [],
            "types": [],
            "errors": []
        }"#;
        let idl = parse_idl_content(json).unwrap();
        let disc = idl.instructions[0].get_discriminator();

        assert_eq!(disc.len(), 8);
        let expected = anchor_discriminator("global:initialize");
        assert_eq!(disc, expected);
    }

    #[test]
    fn test_legacy_account_computes_discriminator() {
        let json = r#"{
            "name": "test",
            "instructions": [],
            "accounts": [
                {
                    "name": "LendingMarket",
                    "type": { "kind": "struct", "fields": [] }
                }
            ],
            "types": [],
            "errors": []
        }"#;
        let idl = parse_idl_content(json).unwrap();
        let disc = idl.accounts[0].get_discriminator();

        assert_eq!(disc.len(), 8);
        let expected = anchor_discriminator("account:LendingMarket");
        assert_eq!(disc, expected);
    }

    #[test]
    fn test_explicit_discriminator_not_overridden() {
        let json = r#"{
            "name": "test",
            "instructions": [
                {
                    "name": "transfer",
                    "discriminator": [1, 2, 3, 4, 5, 6, 7, 8],
                    "accounts": [],
                    "args": []
                }
            ],
            "accounts": [
                {
                    "name": "TokenAccount",
                    "discriminator": [10, 20, 30, 40, 50, 60, 70, 80]
                }
            ],
            "types": [],
            "errors": []
        }"#;
        let idl = parse_idl_content(json).unwrap();

        assert_eq!(
            idl.instructions[0].get_discriminator(),
            vec![1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(
            idl.accounts[0].get_discriminator(),
            vec![10, 20, 30, 40, 50, 60, 70, 80]
        );
    }

    #[test]
    fn test_steel_discriminant_still_works() {
        let json = r#"{
            "name": "test",
            "instructions": [
                {
                    "name": "CreateMetadataAccount",
                    "accounts": [],
                    "args": [],
                    "discriminant": { "type": "u8", "value": 0 }
                },
                {
                    "name": "UpdateMetadataAccount",
                    "accounts": [],
                    "args": [],
                    "discriminant": { "type": "u8", "value": 1 }
                }
            ],
            "accounts": [],
            "types": [],
            "errors": []
        }"#;
        let idl = parse_idl_content(json).unwrap();

        assert_eq!(
            idl.instructions[0].get_discriminator(),
            vec![0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            idl.instructions[1].get_discriminator(),
            vec![1, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_legacy_event_computes_discriminator() {
        let json = r#"{
            "name": "test",
            "instructions": [],
            "accounts": [],
            "types": [],
            "events": [
                { "name": "TransferEvent" }
            ],
            "errors": []
        }"#;
        let idl = parse_idl_content(json).unwrap();
        let disc = idl.events[0].get_discriminator();

        assert_eq!(disc.len(), 8);
        let expected = anchor_discriminator("event:TransferEvent");
        assert_eq!(disc, expected);
    }

    #[test]
    fn test_is_mut_is_signer_aliases() {
        let json = r#"{
            "name": "test",
            "instructions": [
                {
                    "name": "do_thing",
                    "accounts": [
                        { "name": "payer", "isMut": true, "isSigner": true },
                        { "name": "dest", "writable": true, "signer": false }
                    ],
                    "args": []
                }
            ],
            "accounts": [],
            "types": [],
            "errors": []
        }"#;
        let idl = parse_idl_content(json).unwrap();
        let accounts = &idl.instructions[0].accounts;

        assert!(accounts[0].is_mut);
        assert!(accounts[0].is_signer);
        assert!(accounts[1].is_mut);
        assert!(!accounts[1].is_signer);
    }
}
