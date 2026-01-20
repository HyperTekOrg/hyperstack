//! Multi-entity bytecode builder generation.

#![allow(dead_code)]

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::core::to_snake_case;
use crate::parse::proto::ProtoAnalysis;
use crate::proto_codegen;

pub fn generate_multi_entity_builder(
    entity_names: &[String],
    proto_analyses: &[(String, ProtoAnalysis)],
    skip_decoders: bool,
) -> TokenStream {
    let mut builder_calls = Vec::new();

    for (idx, entity_name) in entity_names.iter().enumerate() {
        let spec_fn_name = format_ident!("create_{}_spec", to_snake_case(entity_name));
        let state_id = idx as u32;

        builder_calls.push(quote! {
            .add_entity_with_evaluator(
                #entity_name.to_string(),
                #spec_fn_name(),
                #state_id,
                Some(evaluate_computed_fields)
            )
        });
    }

    let proto_decoders = if !proto_analyses.is_empty() && !skip_decoders {
        proto_codegen::generate_proto_decoders(proto_analyses)
    } else {
        quote! {}
    };

    let proto_router_setup = if !proto_analyses.is_empty() && !skip_decoders {
        proto_codegen::generate_proto_router_setup(proto_analyses)
    } else {
        quote! {}
    };

    let proto_router_assignment = if !proto_analyses.is_empty() && !skip_decoders {
        quote! {
            bytecode.proto_router = setup_proto_router();
        }
    } else {
        quote! {}
    };

    quote! {
        #proto_decoders

        #proto_router_setup

        pub fn create_multi_entity_bytecode() -> hyperstack::runtime::hyperstack_interpreter::compiler::MultiEntityBytecode {
            let mut bytecode = hyperstack::runtime::hyperstack_interpreter::compiler::MultiEntityBytecode::new()
                #(#builder_calls)*
                .build();

            #proto_router_assignment

            bytecode
        }
    }
}

pub fn generate_entity_spec_loader(entity_name: &str) -> TokenStream {
    let spec_fn_name = format_ident!("create_{}_spec", to_snake_case(entity_name));
    let state_name = format_ident!("{}", entity_name);

    quote! {
        pub fn #spec_fn_name() -> hyperstack::runtime::hyperstack_interpreter::ast::TypedStreamSpec<#state_name> {
            let ast_json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.hyperstack/", #entity_name, ".ast.json"));

            let serializable_spec: hyperstack::runtime::hyperstack_interpreter::ast::SerializableStreamSpec = hyperstack::runtime::serde_json::from_str(ast_json)
                .expect(&format!("Failed to parse AST file for {}", #entity_name));

            hyperstack::runtime::hyperstack_interpreter::ast::TypedStreamSpec::from_serializable(serializable_spec)
        }
    }
}
