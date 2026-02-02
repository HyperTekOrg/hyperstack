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
    stack_name: &str,
) -> TokenStream {
    let mut builder_calls = Vec::new();

    for (idx, entity_name) in entity_names.iter().enumerate() {
        let spec_fn_name = format_ident!("create_{}_spec", to_snake_case(entity_name));
        let module_name = format_ident!("{}", to_snake_case(entity_name));
        let state_id = idx as u32;

        builder_calls.push(quote! {
            .add_entity_with_evaluator(
                #entity_name.to_string(),
                #spec_fn_name(),
                #state_id,
                Some(#module_name::evaluate_computed_fields)
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

    let stack_file_name = format!("{}.stack.json", stack_name);
    let view_extraction = quote! {
        {
            let stack_json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.hyperstack/", #stack_file_name));
            let stack_spec: hyperstack::runtime::hyperstack_interpreter::ast::SerializableStackSpec =
                hyperstack::runtime::serde_json::from_str(stack_json)
                    .expect("Failed to parse stack AST file");
            for entity_spec in &stack_spec.entities {
                all_views.extend(entity_spec.views.clone());
            }
        }
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

        pub fn get_view_definitions() -> Vec<hyperstack::runtime::hyperstack_interpreter::ast::ViewDef> {
            let mut all_views = Vec::new();
            #view_extraction
            all_views
        }
    }
}

pub fn generate_entity_spec_loader(entity_name: &str, stack_name: &str) -> TokenStream {
    let spec_fn_name = format_ident!("create_{}_spec", to_snake_case(entity_name));
    let state_name = format_ident!("{}", entity_name);
    let stack_file_name = format!("{}.stack.json", stack_name);

    quote! {
        pub fn #spec_fn_name() -> hyperstack::runtime::hyperstack_interpreter::ast::TypedStreamSpec<#state_name> {
            let stack_json = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.hyperstack/", #stack_file_name));
            let stack_spec: hyperstack::runtime::hyperstack_interpreter::ast::SerializableStackSpec =
                hyperstack::runtime::serde_json::from_str(stack_json)
                    .expect("Failed to parse stack AST file");

            let entity_spec = stack_spec
                .entities
                .iter()
                .find(|e| e.state_name == #entity_name)
                .expect(&format!("Entity {} not found in stack AST", #entity_name));

            let mut spec = entity_spec.clone();
            if spec.idl.is_none() {
                spec.idl = stack_spec.idls.first().cloned();
            }

            hyperstack::runtime::hyperstack_interpreter::ast::TypedStreamSpec::from_serializable(spec)
        }
    }
}
