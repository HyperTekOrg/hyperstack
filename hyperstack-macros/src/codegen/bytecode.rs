//! Bytecode creation from AST specifications.
//!
//! Generates the `create_multi_entity_bytecode` function that creates
//! runtime bytecode from embedded AST JSON.

use proc_macro2::TokenStream;
use quote::quote;

use crate::ast::SerializableStreamSpec;

/// Generate bytecode creation function from a serializable stream spec.
pub fn generate_bytecode_from_spec(spec: &SerializableStreamSpec) -> TokenStream {
    // Serialize the spec to JSON for embedding
    let spec_json = serde_json::to_string(spec).unwrap_or_else(|_| "{}".to_string());
    let entity_name = &spec.state_name;

    quote! {
        /// Create the multi-entity bytecode from the embedded AST specification
        pub fn create_multi_entity_bytecode() -> hyperstack_interpreter::compiler::MultiEntityBytecode {
            // Parse the embedded AST JSON
            let ast_json = #spec_json;
            let spec: hyperstack_interpreter::ast::SerializableStreamSpec = serde_json::from_str(ast_json)
                .expect("Failed to parse embedded AST JSON");

            // Convert to typed spec and compile
            let typed_spec = hyperstack_interpreter::ast::TypedStreamSpec::<serde_json::Value>::from_serializable(spec);

            // Create multi-entity bytecode using from_single
            hyperstack_interpreter::compiler::MultiEntityBytecode::from_single(
                #entity_name.to_string(),
                typed_spec,
                0, // state_id
            )
        }
    }
}
