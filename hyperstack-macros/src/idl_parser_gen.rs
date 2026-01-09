//! IDL parser generation.
//!
//! Generates Vixen-compatible parsers for accounts and instructions from IDL.

#![allow(dead_code)]

use crate::parse::idl::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn generate_parsers(idl: &IdlSpec, program_id: &str) -> TokenStream {
    let account_parser = generate_account_parser(idl, program_id);
    let instruction_parser = generate_instruction_parser(idl, program_id);

    quote! {
        pub mod parsers {
            use super::generated_sdk::*;
            use serde::{Deserialize, Serialize};

            #account_parser
            #instruction_parser
        }
    }
}

fn generate_account_parser(idl: &IdlSpec, program_id: &str) -> TokenStream {
    let program_name = idl.get_name();
    let state_enum_name = format_ident!("{}State", to_pascal_case(program_name));

    let state_enum_variants = idl.accounts.iter().map(|acc| {
        let variant_name = format_ident!("{}", acc.name);
        quote! { #variant_name(accounts::#variant_name) }
    });

    let unpack_arms = idl.accounts.iter().map(|acc| {
        let variant_name = format_ident!("{}", acc.name);
        let _discriminator = &acc.discriminator;

        quote! {
            d if d == accounts::#variant_name::DISCRIMINATOR => {
                Ok(#state_enum_name::#variant_name(
                    accounts::#variant_name::try_from_bytes(data)?
                ))
            }
        }
    });

    let convert_to_json_arms = idl.accounts.iter().map(|acc| {
        let variant_name = format_ident!("{}", acc.name);
        let type_name = format!("{}State", acc.name);

        quote! {
            #state_enum_name::#variant_name(data) => {
                serde_json::json!({
                    "type": #type_name,
                    "data": serde_json::to_value(data).unwrap_or_default()
                })
            }
        }
    });

    let type_name_arms = idl.accounts.iter().map(|acc| {
        let variant_name = format_ident!("{}", acc.name);
        let type_name = format!("{}State", acc.name);

        quote! {
            #state_enum_name::#variant_name(_) => #type_name
        }
    });

    let to_value_arms = idl.accounts.iter().map(|acc| {
        let variant_name = format_ident!("{}", acc.name);

        quote! {
            #state_enum_name::#variant_name(data) => {
                serde_json::to_value(data).unwrap_or_default()
            }
        }
    });

    quote! {
        pub const PROGRAM_ID_STR: &str = #program_id;

        // Lazy-initialized program ID
        static PROGRAM_ID: std::sync::OnceLock<yellowstone_vixen_core::Pubkey> = std::sync::OnceLock::new();

        pub fn program_id() -> yellowstone_vixen_core::Pubkey {
            *PROGRAM_ID.get_or_init(|| {
                let decoded = bs58::decode(PROGRAM_ID_STR)
                    .into_vec()
                    .expect("Invalid program ID");
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(&decoded);
                yellowstone_vixen_core::Pubkey::new(bytes)
            })
        }

        #[derive(Debug)]
        pub enum #state_enum_name {
            #(#state_enum_variants),*
        }

        impl #state_enum_name {
            pub fn try_unpack(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
                if data.len() < 8 {
                    return Err("Data too short for discriminator".into());
                }

                let discriminator = &data[0..8];
                match discriminator {
                    #(#unpack_arms),*
                    // _ => Err(format!("Unknown discriminator: {:?}", discriminator).into())
                    _ => Err(format!("Unknown discriminator: {:?}", discriminator).into())
                }
            }

            pub fn to_json(&self) -> serde_json::Value {
                match self {
                    #(#convert_to_json_arms),*
                }
            }

            /// Returns the event type name for use with the bytecode VM
            pub fn event_type(&self) -> &'static str {
                match self {
                    #(#type_name_arms),*
                }
            }

            /// Converts to serde_json::Value for VM processing
            pub fn to_value(&self) -> serde_json::Value {
                match self {
                    #(#to_value_arms),*
                }
            }
        }

        // Vixen parser implementation
        #[derive(Debug, Copy, Clone)]
        pub struct AccountParser;

        impl yellowstone_vixen_core::Parser for AccountParser {
            type Input = yellowstone_vixen_core::AccountUpdate;
            type Output = #state_enum_name;

            fn id(&self) -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(concat!(#program_name, "::AccountParser"))
            }

            fn prefilter(&self) -> yellowstone_vixen_core::Prefilter {
                yellowstone_vixen_core::Prefilter::builder()
                    .account_owners([program_id()])
                    .build()
                    .unwrap()
            }

            async fn parse(
                &self,
                acct: &yellowstone_vixen_core::AccountUpdate,
            ) -> yellowstone_vixen_core::ParseResult<Self::Output> {
                let inner = acct
                    .account
                    .as_ref()
                    .ok_or(yellowstone_vixen_core::ParseError::from("No account data"))?;

                #state_enum_name::try_unpack(&inner.data)
                    .map_err(|e| {
                        let msg = e.to_string();
                        // Filter out discriminator-related errors (unknown type, short data, etc.)
                        // These are expected when accounts are being initialized/closed or
                        // when we receive accounts that don't match our known types
                        if msg.contains("Unknown discriminator") || msg.contains("too short") {
                            yellowstone_vixen_core::ParseError::Filtered
                        } else {
                            yellowstone_vixen_core::ParseError::from(msg)
                        }
                    })
            }
        }

        impl yellowstone_vixen_core::ProgramParser for AccountParser {
            #[inline]
            fn program_id(&self) -> yellowstone_vixen_core::Pubkey {
                program_id()
            }
        }
    }
}

fn generate_instruction_parser(idl: &IdlSpec, _program_id: &str) -> TokenStream {
    let program_name = idl.get_name();
    let ix_enum_name = format_ident!("{}Instruction", to_pascal_case(program_name));

    let ix_enum_variants = idl.instructions.iter().map(|ix| {
        let variant_name = format_ident!("{}", to_pascal_case(&ix.name));
        quote! { #variant_name(instructions::#variant_name) }
    });

    // Determine if this IDL uses Steel-style discriminants (1 byte) or Anchor-style (8 bytes)
    // Steel IDLs have a `discriminant` field with {"type": "u8", "value": N}
    // Anchor IDLs have a `discriminator` array with 8 bytes
    let uses_steel_discriminant = idl
        .instructions
        .iter()
        .any(|ix| ix.discriminant.is_some() && ix.discriminator.is_empty());

    let discriminator_size: usize = if uses_steel_discriminant { 1 } else { 8 };

    let unpack_arms = idl.instructions.iter().map(|ix| {
        let variant_name = format_ident!("{}", to_pascal_case(&ix.name));
        // Use get_discriminator() to handle both Anchor and Steel formats
        let discriminator = ix.get_discriminator();

        // Use first byte of discriminator as the discriminant value
        let discriminant_value = discriminator.first().copied().unwrap_or(0u8);

        // Use the correct offset based on IDL type
        let disc_size = discriminator_size;

        // Check if instruction has no args - use Default instead of deserializing
        let has_args = !ix.args.is_empty();
        if has_args {
            quote! {
                #discriminant_value => {
                    let data = instructions::#variant_name::try_from_bytes(&data[#disc_size..])?;
                    Ok(#ix_enum_name::#variant_name(data))
                }
            }
        } else {
            // For instructions with no args, just use Default to create an empty struct
            quote! {
                #discriminant_value => {
                    Ok(#ix_enum_name::#variant_name(instructions::#variant_name::default()))
                }
            }
        }
    });

    let convert_to_json_arms = idl.instructions.iter().map(|ix| {
        let variant_name = format_ident!("{}", to_pascal_case(&ix.name));
        let type_name = format!("{}::{}", program_name, to_pascal_case(&ix.name));

        quote! {
            #ix_enum_name::#variant_name(data) => {
                serde_json::json!({
                    "type": #type_name,
                    "data": serde_json::to_value(data).unwrap_or_default()
                })
            }
        }
    });

    let type_name_arms = idl.instructions.iter().map(|ix| {
        let variant_name = format_ident!("{}", to_pascal_case(&ix.name));
        // Use IxState suffix to match bytecode compiler's event routing
        let type_name = format!("{}IxState", to_pascal_case(&ix.name));

        quote! {
            #ix_enum_name::#variant_name(_) => #type_name
        }
    });

    let to_value_arms = idl.instructions.iter().map(|ix| {
        let variant_name = format_ident!("{}", to_pascal_case(&ix.name));

        quote! {
            #ix_enum_name::#variant_name(data) => {
                // Wrap instruction data in a "data" field to match bytecode expectations
                serde_json::json!({
                    "data": serde_json::to_value(data).unwrap_or_default()
                })
            }
        }
    });

    // Generate arms for to_value_with_accounts that includes account names from IDL
    let to_value_with_accounts_arms = idl.instructions.iter().map(|ix| {
        let variant_name = format_ident!("{}", to_pascal_case(&ix.name));
        let account_names: Vec<_> = ix.accounts.iter().map(|acc| &acc.name).collect();

        quote! {
            #ix_enum_name::#variant_name(data) => {
                let mut value = serde_json::json!({
                    "data": serde_json::to_value(data).unwrap_or_default()
                });

                // Add named accounts
                if let Some(obj) = value.as_object_mut() {
                    let account_names = vec![#(#account_names),*];
                    let mut accounts_obj = serde_json::Map::new();
                    for (i, name) in account_names.iter().enumerate() {
                        if i < accounts.len() {
                            accounts_obj.insert(
                                name.to_string(),
                                serde_json::Value::String(bs58::encode(&accounts[i].0).into_string())
                            );
                        }
                    }
                    obj.insert("accounts".to_string(), serde_json::Value::Object(accounts_obj));
                }

                value
            }
        }
    });

    quote! {
        #[derive(Debug)]
        pub enum #ix_enum_name {
            #(#ix_enum_variants),*
        }

        impl #ix_enum_name {
            pub fn try_unpack(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
                if data.is_empty() {
                    return Err("Empty instruction data".into());
                }

                let discriminator = data[0];
                match discriminator {
                    #(#unpack_arms),*
                    _ => Err(format!("Unknown instruction discriminator: {}", discriminator).into())
                }
            }

            pub fn to_json(&self) -> serde_json::Value {
                match self {
                    #(#convert_to_json_arms),*
                }
            }

            /// Returns the event type name for use with the bytecode VM
            pub fn event_type(&self) -> &'static str {
                match self {
                    #(#type_name_arms),*
                }
            }

            /// Converts to serde_json::Value for VM processing
            pub fn to_value(&self) -> serde_json::Value {
                match self {
                    #(#to_value_arms),*
                }
            }

            /// Converts to serde_json::Value with accounts information
            pub fn to_value_with_accounts(&self, accounts: &[yellowstone_vixen_core::KeyBytes<32>]) -> serde_json::Value {
                match self {
                    #(#to_value_with_accounts_arms),*
                }
            }
        }

        // Vixen parser implementation
        #[derive(Debug, Copy, Clone)]
        pub struct InstructionParser;

        impl yellowstone_vixen_core::Parser for InstructionParser {
            type Input = yellowstone_vixen_core::instruction::InstructionUpdate;
            type Output = #ix_enum_name;

            fn id(&self) -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(concat!(#program_name, "::InstructionParser"))
            }

            fn prefilter(&self) -> yellowstone_vixen_core::Prefilter {
                yellowstone_vixen_core::Prefilter::builder()
                    .transaction_accounts([program_id()])
                    .build()
                    .unwrap()
            }

            async fn parse(
                &self,
                ix_update: &yellowstone_vixen_core::instruction::InstructionUpdate,
            ) -> yellowstone_vixen_core::ParseResult<Self::Output> {
                if ix_update.program.equals_ref(program_id()) {
                    let parsed = #ix_enum_name::try_unpack(&ix_update.data)
                        .map_err(|e| {
                            // Filter out unknown discriminator errors, propagate others
                            if e.to_string().contains("Unknown instruction discriminator") {
                                yellowstone_vixen_core::ParseError::Filtered
                            } else {
                                yellowstone_vixen_core::ParseError::from(e.to_string())
                            }
                        })?;

                    Ok(parsed)
                } else {
                    Err(yellowstone_vixen_core::ParseError::Filtered)
                }
            }
        }

        impl yellowstone_vixen_core::ProgramParser for InstructionParser {
            #[inline]
            fn program_id(&self) -> yellowstone_vixen_core::Pubkey {
                program_id()
            }
        }
    }
}
