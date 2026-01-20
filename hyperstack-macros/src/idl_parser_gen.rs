//! IDL parser generation.

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
            use hyperstack::runtime::serde::{Deserialize, Serialize};

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
                hyperstack::runtime::serde_json::json!({
                    "type": #type_name,
                    "data": hyperstack::runtime::serde_json::to_value(data).unwrap_or_default()
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
                hyperstack::runtime::serde_json::to_value(data).unwrap_or_default()
            }
        }
    });

    quote! {
        pub const PROGRAM_ID_STR: &str = #program_id;

        static PROGRAM_ID: std::sync::OnceLock<hyperstack::runtime::yellowstone_vixen_core::Pubkey> = std::sync::OnceLock::new();

        pub fn program_id() -> hyperstack::runtime::yellowstone_vixen_core::Pubkey {
            *PROGRAM_ID.get_or_init(|| {
                let decoded = hyperstack::runtime::bs58::decode(PROGRAM_ID_STR)
                    .into_vec()
                    .expect("Invalid program ID");
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(&decoded);
                hyperstack::runtime::yellowstone_vixen_core::Pubkey::new(bytes)
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
                    _ => Err(format!("Unknown discriminator: {:?}", discriminator).into())
                }
            }

            pub fn to_json(&self) -> hyperstack::runtime::serde_json::Value {
                match self {
                    #(#convert_to_json_arms),*
                }
            }

            pub fn event_type(&self) -> &'static str {
                match self {
                    #(#type_name_arms),*
                }
            }

            pub fn to_value(&self) -> hyperstack::runtime::serde_json::Value {
                match self {
                    #(#to_value_arms),*
                }
            }
        }

        #[derive(Debug, Copy, Clone)]
        pub struct AccountParser;

        impl hyperstack::runtime::yellowstone_vixen_core::Parser for AccountParser {
            type Input = hyperstack::runtime::yellowstone_vixen_core::AccountUpdate;
            type Output = #state_enum_name;

            fn id(&self) -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(concat!(#program_name, "::AccountParser"))
            }

            fn prefilter(&self) -> hyperstack::runtime::yellowstone_vixen_core::Prefilter {
                hyperstack::runtime::yellowstone_vixen_core::Prefilter::builder()
                    .account_owners([program_id()])
                    .build()
                    .unwrap()
            }

            async fn parse(
                &self,
                acct: &hyperstack::runtime::yellowstone_vixen_core::AccountUpdate,
            ) -> hyperstack::runtime::yellowstone_vixen_core::ParseResult<Self::Output> {
                let inner = acct
                    .account
                    .as_ref()
                    .ok_or(hyperstack::runtime::yellowstone_vixen_core::ParseError::from("No account data"))?;

                #state_enum_name::try_unpack(&inner.data)
                    .map_err(|e| {
                        let msg = e.to_string();
                        if msg.contains("Unknown discriminator") || msg.contains("too short") {
                            hyperstack::runtime::yellowstone_vixen_core::ParseError::Filtered
                        } else {
                            hyperstack::runtime::yellowstone_vixen_core::ParseError::from(msg)
                        }
                    })
            }
        }

        impl hyperstack::runtime::yellowstone_vixen_core::ProgramParser for AccountParser {
            #[inline]
            fn program_id(&self) -> hyperstack::runtime::yellowstone_vixen_core::Pubkey {
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

    let uses_steel_discriminant = idl
        .instructions
        .iter()
        .any(|ix| ix.discriminant.is_some() && ix.discriminator.is_empty());

    let discriminator_size: usize = if uses_steel_discriminant { 1 } else { 8 };

    let unpack_arms = idl.instructions.iter().map(|ix| {
        let variant_name = format_ident!("{}", to_pascal_case(&ix.name));
        let discriminator = ix.get_discriminator();
        let discriminant_value = discriminator.first().copied().unwrap_or(0u8);
        let disc_size = discriminator_size;

        let has_args = !ix.args.is_empty();
        if has_args {
            quote! {
                #discriminant_value => {
                    let data = instructions::#variant_name::try_from_bytes(&data[#disc_size..])?;
                    Ok(#ix_enum_name::#variant_name(data))
                }
            }
        } else {
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
                hyperstack::runtime::serde_json::json!({
                    "type": #type_name,
                    "data": hyperstack::runtime::serde_json::to_value(data).unwrap_or_default()
                })
            }
        }
    });

    let type_name_arms = idl.instructions.iter().map(|ix| {
        let variant_name = format_ident!("{}", to_pascal_case(&ix.name));
        let type_name = format!("{}IxState", to_pascal_case(&ix.name));

        quote! {
            #ix_enum_name::#variant_name(_) => #type_name
        }
    });

    let to_value_arms = idl.instructions.iter().map(|ix| {
        let variant_name = format_ident!("{}", to_pascal_case(&ix.name));

        quote! {
            #ix_enum_name::#variant_name(data) => {
                hyperstack::runtime::serde_json::json!({
                    "data": hyperstack::runtime::serde_json::to_value(data).unwrap_or_default()
                })
            }
        }
    });

    let to_value_with_accounts_arms = idl.instructions.iter().map(|ix| {
        let variant_name = format_ident!("{}", to_pascal_case(&ix.name));
        let account_names: Vec<_> = ix.accounts.iter().map(|acc| &acc.name).collect();

        quote! {
            #ix_enum_name::#variant_name(data) => {
                let mut value = hyperstack::runtime::serde_json::json!({
                    "data": hyperstack::runtime::serde_json::to_value(data).unwrap_or_default()
                });

                if let Some(obj) = value.as_object_mut() {
                    let account_names = vec![#(#account_names),*];
                    let mut accounts_obj = hyperstack::runtime::serde_json::Map::new();
                    for (i, name) in account_names.iter().enumerate() {
                        if i < accounts.len() {
                            accounts_obj.insert(
                                name.to_string(),
                                hyperstack::runtime::serde_json::Value::String(hyperstack::runtime::bs58::encode(&accounts[i].0).into_string())
                            );
                        }
                    }
                    obj.insert("accounts".to_string(), hyperstack::runtime::serde_json::Value::Object(accounts_obj));
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

            pub fn to_json(&self) -> hyperstack::runtime::serde_json::Value {
                match self {
                    #(#convert_to_json_arms),*
                }
            }

            pub fn event_type(&self) -> &'static str {
                match self {
                    #(#type_name_arms),*
                }
            }

            pub fn to_value(&self) -> hyperstack::runtime::serde_json::Value {
                match self {
                    #(#to_value_arms),*
                }
            }

            pub fn to_value_with_accounts(&self, accounts: &[hyperstack::runtime::yellowstone_vixen_core::KeyBytes<32>]) -> hyperstack::runtime::serde_json::Value {
                match self {
                    #(#to_value_with_accounts_arms),*
                }
            }
        }

        #[derive(Debug, Copy, Clone)]
        pub struct InstructionParser;

        impl hyperstack::runtime::yellowstone_vixen_core::Parser for InstructionParser {
            type Input = hyperstack::runtime::yellowstone_vixen_core::instruction::InstructionUpdate;
            type Output = #ix_enum_name;

            fn id(&self) -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(concat!(#program_name, "::InstructionParser"))
            }

            fn prefilter(&self) -> hyperstack::runtime::yellowstone_vixen_core::Prefilter {
                hyperstack::runtime::yellowstone_vixen_core::Prefilter::builder()
                    .transaction_accounts([program_id()])
                    .build()
                    .unwrap()
            }

            async fn parse(
                &self,
                ix_update: &hyperstack::runtime::yellowstone_vixen_core::instruction::InstructionUpdate,
            ) -> hyperstack::runtime::yellowstone_vixen_core::ParseResult<Self::Output> {
                if ix_update.program.equals_ref(program_id()) {
                    let parsed = #ix_enum_name::try_unpack(&ix_update.data)
                        .map_err(|e| {
                            if e.to_string().contains("Unknown instruction discriminator") {
                                hyperstack::runtime::yellowstone_vixen_core::ParseError::Filtered
                            } else {
                                hyperstack::runtime::yellowstone_vixen_core::ParseError::from(e.to_string())
                            }
                        })?;

                    Ok(parsed)
                } else {
                    Err(hyperstack::runtime::yellowstone_vixen_core::ParseError::Filtered)
                }
            }
        }

        impl hyperstack::runtime::yellowstone_vixen_core::ProgramParser for InstructionParser {
            #[inline]
            fn program_id(&self) -> hyperstack::runtime::yellowstone_vixen_core::Pubkey {
                program_id()
            }
        }
    }
}
