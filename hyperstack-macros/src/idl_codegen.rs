use crate::parse::idl::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

fn is_bytemuck_serialization(serialization: &Option<IdlSerialization>) -> bool {
    matches!(
        serialization,
        Some(IdlSerialization::Bytemuck) | Some(IdlSerialization::BytemuckUnsafe)
    )
}

fn lookup_account_serialization<'a>(
    account_name: &str,
    types: &'a [IdlTypeDef],
) -> &'a Option<IdlSerialization> {
    types
        .iter()
        .find(|t| t.name == account_name)
        .map(|t| &t.serialization)
        .unwrap_or(&None)
}

fn type_to_token_stream_bytemuck(idl_type: &IdlType) -> TokenStream {
    let type_str = idl_type.to_rust_type_string_bytemuck();
    let tokens: TokenStream = type_str.parse().unwrap_or_else(|_| {
        quote! { serde_json::Value }
    });
    tokens
}

fn is_pubkey_type(idl_type: &IdlType) -> bool {
    matches!(idl_type, IdlType::Simple(s) if s == "pubkey" || s == "publicKey")
}

fn generate_bytemuck_field(field: &IdlField) -> TokenStream {
    let field_name = format_ident!("{}", to_snake_case(&field.name));
    let field_type = type_to_token_stream_bytemuck(&field.type_);
    if is_pubkey_type(&field.type_) {
        quote! {
            #[serde(with = "hyperstack::runtime::serde_helpers::pubkey_base58")]
            pub #field_name: #field_type
        }
    } else {
        quote! { pub #field_name: #field_type }
    }
}

pub fn generate_sdk_types(idl: &IdlSpec, module_name: &str) -> TokenStream {
    let account_types = generate_account_types(&idl.accounts, &idl.types);
    let instruction_types = generate_instruction_types(&idl.instructions, &idl.types);
    let custom_types = generate_custom_types(&idl.types);
    let module_ident = format_ident!("{}", module_name);

    quote! {
        pub mod #module_ident {
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

fn generate_struct_fields(fields: &[IdlField], use_bytemuck: bool) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            if use_bytemuck {
                generate_bytemuck_field(field)
            } else {
                let field_name = format_ident!("{}", to_snake_case(&field.name));
                let field_type = type_to_token_stream(&field.type_);
                quote! { pub #field_name: #field_type }
            }
        })
        .collect()
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
    let serialization = lookup_account_serialization(&account.name, types);
    let use_bytemuck = is_bytemuck_serialization(serialization);

    let fields = if let Some(type_def) = &account.type_def {
        match type_def {
            IdlTypeDefKind::Struct { fields, .. } => generate_struct_fields(fields, use_bytemuck),
            IdlTypeDefKind::TupleStruct { .. } | IdlTypeDefKind::Enum { .. } => vec![],
        }
    } else if let Some(type_def) = types.iter().find(|t| t.name == account.name) {
        match &type_def.type_def {
            IdlTypeDefKind::Struct { fields, .. } => generate_struct_fields(fields, use_bytemuck),
            IdlTypeDefKind::TupleStruct { .. } | IdlTypeDefKind::Enum { .. } => vec![],
        }
    } else {
        vec![]
    };

    let discriminator = &account.discriminator;
    let disc_array = quote! { [#(#discriminator),*] };

    if use_bytemuck {
        quote! {
            #[derive(Debug, Copy, Clone, Serialize, Deserialize, hyperstack::runtime::bytemuck::Pod, hyperstack::runtime::bytemuck::Zeroable)]
            #[bytemuck(crate = "hyperstack::runtime::bytemuck")]
            #[repr(C)]
            pub struct #name {
                #(#fields),*
            }

            impl #name {
                pub const DISCRIMINATOR: [u8; 8] = #disc_array;

                pub fn try_from_bytes(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
                    if data.len() < 8 {
                        return Err("Data too short for discriminator".into());
                    }
                    let body = &data[8..];
                    let struct_size = std::mem::size_of::<Self>();
                    if body.len() < struct_size {
                        return Err(format!(
                            "Data too short for {}: need {} bytes, got {}",
                            stringify!(#name), struct_size, body.len()
                        ).into());
                    }
                    Ok(hyperstack::runtime::bytemuck::pod_read_unaligned::<Self>(&body[..struct_size]))
                }
            }
        }
    } else {
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
                    let mut reader = &data[8..];
                    borsh::BorshDeserialize::deserialize_reader(&mut reader)
                        .map_err(|e| e.into())
                }
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

    let derives = quote! { #[derive(Debug, Clone, Default, Serialize, Deserialize, BorshSerialize, BorshDeserialize)] };

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
    let use_bytemuck = is_bytemuck_serialization(&type_def.serialization);

    match &type_def.type_def {
        IdlTypeDefKind::Struct { kind: _, fields } => {
            let struct_fields = generate_struct_fields(fields, use_bytemuck);

            if use_bytemuck {
                quote! {
                    #[derive(Debug, Copy, Clone, Serialize, Deserialize, hyperstack::runtime::bytemuck::Pod, hyperstack::runtime::bytemuck::Zeroable)]
                    #[bytemuck(crate = "hyperstack::runtime::bytemuck")]
                    #[repr(C)]
                    pub struct #name {
                        #(#struct_fields),*
                    }
                }
            } else {
                quote! {
                    #[derive(Debug, Clone, Default, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
                    pub struct #name {
                        #(#struct_fields),*
                    }
                }
            }
        }
        IdlTypeDefKind::TupleStruct { kind: _, fields } => {
            let tuple_fields = fields.iter().map(type_to_token_stream);

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

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_bytemuck_idl() -> IdlSpec {
        let json = r#"{
            "address": "TestBytemuckProgram111111111111111111111",
            "metadata": {
                "name": "test_bytemuck",
                "version": "0.1.0",
                "spec": "0.1.0"
            },
            "instructions": [],
            "accounts": [
                {
                    "name": "MyHeader",
                    "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]
                },
                {
                    "name": "RegularAccount",
                    "discriminator": [10, 20, 30, 40, 50, 60, 70, 80]
                }
            ],
            "types": [
                {
                    "name": "MyHeader",
                    "serialization": "bytemuck",
                    "repr": { "kind": "c" },
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "authority", "type": "pubkey" },
                            { "name": "total_fees", "type": "u128" },
                            { "name": "bump", "type": "u8" },
                            { "name": "is_active", "type": "bool" },
                            { "name": "_padding", "type": { "array": ["u8", 14] } }
                        ]
                    }
                },
                {
                    "name": "RegularAccount",
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "owner", "type": "pubkey" },
                            { "name": "balance", "type": "u64" },
                            { "name": "is_initialized", "type": "bool" }
                        ]
                    }
                }
            ],
            "events": [],
            "errors": []
        }"#;
        parse_idl_content(json).expect("test IDL should parse")
    }

    #[test]
    fn test_bytemuck_idl_parses_serialization_field() {
        let idl = minimal_bytemuck_idl();
        let header = idl.types.iter().find(|t| t.name == "MyHeader").unwrap();
        assert_eq!(header.serialization, Some(IdlSerialization::Bytemuck));
        assert!(header.repr.is_some());
        assert_eq!(header.repr.as_ref().unwrap().kind, "c");

        let regular = idl
            .types
            .iter()
            .find(|t| t.name == "RegularAccount")
            .unwrap();
        assert_eq!(regular.serialization, None);
    }

    #[test]
    fn test_bytemuck_codegen_produces_pod_derives() {
        let idl = minimal_bytemuck_idl();
        let output = generate_sdk_types(&idl, "generated_sdk");
        let code = output.to_string();

        assert!(code.contains("Pod"), "bytemuck account should derive Pod");
        assert!(
            code.contains("Zeroable"),
            "bytemuck account should derive Zeroable"
        );
        assert!(
            code.contains("repr (C)"),
            "bytemuck account should have #[repr(C)]"
        );
    }

    #[test]
    fn test_bytemuck_maps_pubkey_to_byte_array() {
        let idl = minimal_bytemuck_idl();
        let output = generate_sdk_types(&idl, "generated_sdk");
        let code = output.to_string();

        assert!(
            code.contains("[u8 ; 32]"),
            "bytemuck pubkey should map to [u8; 32], got: {}",
            code
        );
    }

    #[test]
    fn test_bytemuck_maps_bool_to_u8() {
        let idl = minimal_bytemuck_idl();
        let output = generate_sdk_types(&idl, "generated_sdk");
        let code = output.to_string();

        assert!(
            !code.contains("pub is_active : bool"),
            "bytemuck bool should NOT remain bool"
        );
        assert!(
            code.contains("pub is_active : u8"),
            "bytemuck bool should map to u8, got: {}",
            code
        );
    }

    #[test]
    fn test_bytemuck_pubkey_gets_serde_base58() {
        let idl = minimal_bytemuck_idl();
        let output = generate_sdk_types(&idl, "generated_sdk");
        let code = output.to_string();

        assert!(
            code.contains("pubkey_base58"),
            "bytemuck pubkey fields should have serde(with = pubkey_base58) attribute, got: {}",
            code
        );
    }

    #[test]
    fn test_regular_account_uses_borsh() {
        let idl = minimal_bytemuck_idl();
        let output = generate_sdk_types(&idl, "generated_sdk");
        let code = output.to_string();

        assert!(
            code.contains("BorshDeserialize"),
            "regular account should derive BorshDeserialize"
        );
    }

    #[test]
    fn test_regular_account_keeps_bool_as_bool() {
        let idl = minimal_bytemuck_idl();
        let output = generate_sdk_types(&idl, "generated_sdk");
        let code = output.to_string();

        assert!(
            code.contains("pub is_initialized : bool"),
            "regular account bool should stay bool, got: {}",
            code
        );
    }

    #[test]
    fn test_bytemuck_try_from_bytes_uses_bytemuck() {
        let idl = minimal_bytemuck_idl();
        let output = generate_sdk_types(&idl, "generated_sdk");
        let code = output.to_string();

        assert!(
            code.contains("pod_read_unaligned"),
            "bytemuck try_from_bytes should use bytemuck::pod_read_unaligned"
        );
    }

    #[test]
    fn test_regular_try_from_bytes_uses_borsh() {
        let idl = minimal_bytemuck_idl();
        let output = generate_sdk_types(&idl, "generated_sdk");
        let code = output.to_string();

        assert!(
            code.contains("deserialize_reader"),
            "regular try_from_bytes should use borsh::deserialize_reader"
        );
    }

    #[test]
    fn test_bytemuck_unsafe_variant_parses() {
        let json = r#"{
            "metadata": { "name": "test", "version": "0.1.0" },
            "instructions": [],
            "accounts": [],
            "types": [
                {
                    "name": "UnsafeHeader",
                    "serialization": "bytemuckunsafe",
                    "repr": { "kind": "c" },
                    "type": {
                        "kind": "struct",
                        "fields": [
                            { "name": "data", "type": "u64" }
                        ]
                    }
                }
            ],
            "events": [],
            "errors": []
        }"#;
        let idl = parse_idl_content(json).expect("bytemuckunsafe should parse");
        let t = &idl.types[0];
        assert_eq!(t.serialization, Some(IdlSerialization::BytemuckUnsafe));
    }
}
