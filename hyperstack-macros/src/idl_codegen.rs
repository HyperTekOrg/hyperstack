use crate::parse::idl::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn generate_sdk_types(idl: &IdlSpec) -> TokenStream {
    let account_types = generate_account_types(&idl.accounts, &idl.types);
    let instruction_types = generate_instruction_types(&idl.instructions, &idl.types);
    let custom_types = generate_custom_types(&idl.types);

    quote! {
        pub mod generated_sdk {
            use serde::{Deserialize, Serialize};
            use borsh::{BorshDeserialize, BorshSerialize};

            pub mod types {
                use super::*;
                #custom_types
            }

            pub mod accounts {
                use super::*;
                use super::types::*;
                #account_types
            }

            pub mod instructions {
                use super::*;
                use super::types::*;
                #instruction_types
            }
        }
    }
}

fn generate_account_types(accounts: &[IdlAccount], types: &[IdlTypeDef]) -> TokenStream {
    let account_structs = accounts
        .iter()
        .map(|account| generate_account_type(account, types));

    quote! {
        #(#account_structs)*
    }
}

fn generate_account_type(account: &IdlAccount, types: &[IdlTypeDef]) -> TokenStream {
    let name = format_ident!("{}", account.name);

    // First try embedded type definition (Steel format)
    let fields = if let Some(type_def) = &account.type_def {
        match type_def {
            IdlTypeDefKind::Struct { fields, .. } => fields
                .iter()
                .map(|field| {
                    let field_name = format_ident!("{}", to_snake_case(&field.name));
                    let field_type = type_to_token_stream(&field.type_);
                    quote! { pub #field_name: #field_type }
                })
                .collect::<Vec<_>>(),
            IdlTypeDefKind::TupleStruct { .. } | IdlTypeDefKind::Enum { .. } => vec![],
        }
    } else {
        // Fallback: Look up the type definition in the types array (Anchor format)
        if let Some(type_def) = types.iter().find(|t| t.name == account.name) {
            match &type_def.type_def {
                IdlTypeDefKind::Struct { fields, .. } => fields
                    .iter()
                    .map(|field| {
                        let field_name = format_ident!("{}", to_snake_case(&field.name));
                        let field_type = type_to_token_stream(&field.type_);
                        quote! { pub #field_name: #field_type }
                    })
                    .collect::<Vec<_>>(),
                IdlTypeDefKind::TupleStruct { .. } | IdlTypeDefKind::Enum { .. } => vec![],
            }
        } else {
            vec![]
        }
    };

    let discriminator = &account.discriminator;
    let disc_array = quote! { [#(#discriminator),*] };

    quote! {
        #[derive(Debug, Clone, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
        pub struct #name {
            #(#fields),*
        }

        impl #name {
            pub const DISCRIMINATOR: [u8; 8] = #disc_array;

            pub fn try_from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
                if data.len() < 8 {
                    return Err("Data too short for discriminator".into());
                }

                // Skip discriminator and deserialize with borsh
                let mut reader = &data[8..];
                borsh::BorshDeserialize::deserialize_reader(&mut reader)
                    .map_err(|e| e.into())
            }
        }
    }
}

fn generate_instruction_types(
    instructions: &[IdlInstruction],
    _types: &[IdlTypeDef],
) -> TokenStream {
    let instruction_structs = instructions.iter().map(generate_instruction_type);

    quote! {
        #(#instruction_structs)*
    }
}

fn generate_instruction_type(instruction: &IdlInstruction) -> TokenStream {
    let name = format_ident!("{}", to_pascal_case(&instruction.name));

    // Use get_discriminator() to handle both Anchor and Steel formats
    let discriminator = instruction.get_discriminator();
    let disc_array = quote! { [#(#discriminator),*] };

    let args_fields = instruction.args.iter().map(|arg| {
        let arg_name = format_ident!("{}", to_snake_case(&arg.name));
        let arg_type = type_to_token_stream(&arg.type_);
        quote! { pub #arg_name: #arg_type }
    });

    // For instructions with no args, also derive Default so we can construct them without deserialization
    let has_args = !instruction.args.is_empty();
    let derives = if has_args {
        quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize, BorshSerialize, BorshDeserialize)] }
    } else {
        quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize, BorshSerialize, BorshDeserialize)] }
    };

    quote! {
        #derives
        pub struct #name {
            #(#args_fields),*
        }

        impl #name {
            pub const DISCRIMINATOR: [u8; 8] = #disc_array;

            pub fn try_from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
                let mut reader = data;
                match borsh::BorshDeserialize::deserialize_reader(&mut reader) {
                    Ok(v) => Ok(v),
                    Err(_) if !data.is_empty() => {
                        let mut padded = data.to_vec();
                        padded.resize(256, 0);
                        let mut reader = padded.as_slice();
                        borsh::BorshDeserialize::deserialize_reader(&mut reader)
                            .map_err(|e| e.into())
                    }
                    Err(e) => Err(e.into()),
                }
            }
        }
    }
}

fn generate_custom_types(types: &[IdlTypeDef]) -> TokenStream {
    let type_defs = types.iter().map(generate_custom_type);

    quote! {
        #(#type_defs)*
    }
}

fn generate_custom_type(type_def: &IdlTypeDef) -> TokenStream {
    let name = format_ident!("{}", type_def.name);

    match &type_def.type_def {
        IdlTypeDefKind::Struct { kind: _, fields } => {
            let struct_fields = fields.iter().map(|field| {
                let field_name = format_ident!("{}", to_snake_case(&field.name));
                let field_type = type_to_token_stream(&field.type_);
                quote! { pub #field_name: #field_type }
            });

            quote! {
                #[derive(Debug, Clone, Default, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
                pub struct #name {
                    #(#struct_fields),*
                }
            }
        }
        IdlTypeDefKind::TupleStruct { kind: _, fields } => {
            let tuple_fields = fields.iter().map(|t| type_to_token_stream(t));

            quote! {
                #[derive(Debug, Clone, Default, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
                pub struct #name(#(pub #tuple_fields),*);
            }
        }
        IdlTypeDefKind::Enum { kind: _, variants } => {
            let enum_variants = variants.iter().enumerate().map(|(i, variant)| {
                let variant_name = format_ident!("{}", variant.name);
                if i == 0 {
                    quote! { #[default] #variant_name }
                } else {
                    quote! { #variant_name }
                }
            });

            quote! {
                #[derive(Debug, Clone, Default, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
                pub enum #name {
                    #(#enum_variants),*
                }
            }
        }
    }
}

fn type_to_token_stream(idl_type: &IdlType) -> TokenStream {
    let type_str = idl_type.to_rust_type_string();
    let tokens: TokenStream = type_str.parse().unwrap_or_else(|_| {
        // Fallback for complex types
        quote! { serde_json::Value }
    });
    tokens
}
