//! Views macro for declaring derived views.
//!
//! This module provides the `views!` macro for defining derived views using
//! a functional/fluent syntax similar to iterator pipelines.
//!
//! # Example
//!
//! ```rust,ignore
//! views! {
//!     // Derive latest view from list, sorted by id.round_id descending, take first
//!     OreRound/latest = OreRound/list
//!         | sort(fields::id::round_id(), Desc)
//!         | first;
//!     
//!     // Active rounds filtered by expiry time
//!     OreRound/active = OreRound/list
//!         | filter(fields::state::expires_at() > now())
//!         | sort(fields::id::round_id(), Desc);
//! }
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Result, Token};

// ============================================================================
// AST Types for View Definitions
// ============================================================================

/// Sort order for view transforms
#[derive(Debug, Clone)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// A parsed transform in the view pipeline
#[derive(Debug, Clone)]
pub enum Transform {
    Sort { field: syn::Expr, order: SortOrder },
    Filter { predicate: syn::Expr },
    Take { count: usize },
    Skip { count: usize },
    First,
    Last,
    MaxBy { field: syn::Expr },
    MinBy { field: syn::Expr },
}

/// A single view definition: `ViewName = source | transform | transform`
#[derive(Debug)]
struct ViewDefinition {
    name: String,
    source: String,
    transforms: Vec<Transform>,
}

/// Input to the views! macro
#[derive(Debug)]
struct ViewsInput {
    definitions: Vec<ViewDefinition>,
}

// ============================================================================
// Parsing Implementation
// ============================================================================

impl Parse for ViewsInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut definitions = Vec::new();

        while !input.is_empty() {
            // Parse: ViewName = source | transform1 | transform2 ;
            let view_name: Ident = input.parse()?;

            // Check for slash in name (e.g., OreRound/latest)
            let full_name = if input.peek(Token![/]) {
                input.parse::<Token![/]>()?;
                let suffix: Ident = input.parse()?;
                format!("{}/{}", view_name, suffix)
            } else {
                view_name.to_string()
            };

            input.parse::<Token![=]>()?;

            // Parse source (e.g., OreRound/list)
            let source_name: Ident = input.parse()?;
            let source = if input.peek(Token![/]) {
                input.parse::<Token![/]>()?;
                let suffix: Ident = input.parse()?;
                format!("{}/{}", source_name, suffix)
            } else {
                source_name.to_string()
            };

            // Parse transforms (| transform)*
            let mut transforms = Vec::new();
            while input.peek(Token![|]) {
                input.parse::<Token![|]>()?;
                let transform = parse_transform(input)?;
                transforms.push(transform);
            }

            // Expect semicolon
            input.parse::<Token![;]>()?;

            definitions.push(ViewDefinition {
                name: full_name,
                source,
                transforms,
            });
        }

        Ok(ViewsInput { definitions })
    }
}

fn parse_transform(input: ParseStream) -> Result<Transform> {
    let name: Ident = input.parse()?;

    match name.to_string().as_str() {
        "sort" => {
            let content;
            syn::parenthesized!(content in input);
            let field: syn::Expr = content.parse()?;
            content.parse::<Token![,]>()?;
            let order_ident: Ident = content.parse()?;
            let order = match order_ident.to_string().as_str() {
                "Desc" => SortOrder::Desc,
                _ => SortOrder::Asc,
            };
            Ok(Transform::Sort { field, order })
        }
        "filter" => {
            let content;
            syn::parenthesized!(content in input);
            let predicate: syn::Expr = content.parse()?;
            Ok(Transform::Filter { predicate })
        }
        "take" => {
            let content;
            syn::parenthesized!(content in input);
            let count: syn::LitInt = content.parse()?;
            Ok(Transform::Take {
                count: count.base10_parse()?,
            })
        }
        "skip" => {
            let content;
            syn::parenthesized!(content in input);
            let count: syn::LitInt = content.parse()?;
            Ok(Transform::Skip {
                count: count.base10_parse()?,
            })
        }
        "first" => Ok(Transform::First),
        "last" => Ok(Transform::Last),
        "max_by" => {
            let content;
            syn::parenthesized!(content in input);
            let field: syn::Expr = content.parse()?;
            Ok(Transform::MaxBy { field })
        }
        "min_by" => {
            let content;
            syn::parenthesized!(content in input);
            let field: syn::Expr = content.parse()?;
            Ok(Transform::MinBy { field })
        }
        _ => Err(syn::Error::new(
            name.span(),
            format!("Unknown transform: {}", name),
        )),
    }
}

// ============================================================================
// Code Generation
// ============================================================================

/// Generate code for the views! macro
pub fn expand_views(input: TokenStream) -> TokenStream {
    let views_input: ViewsInput = match syn::parse2(input) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let view_defs: Vec<TokenStream> = views_input
        .definitions
        .iter()
        .map(|def| {
            let name = &def.name;
            let source = &def.source;

            // Determine source type (Entity or View)
            let source_parts: Vec<&str> = source.split('/').collect();
            let source_expr = if source_parts.len() == 2
                && (source_parts[1] == "list" || source_parts[1] == "state")
            {
                // It's a base view (list or state), derive from entity
                let entity_name = source_parts[0];
                quote! {
                    hyperstack::runtime::hyperstack_interpreter::ast::ViewSource::Entity {
                        name: #entity_name.to_string()
                    }
                }
            } else {
                // It's a derived view
                quote! {
                    hyperstack::runtime::hyperstack_interpreter::ast::ViewSource::View {
                        id: #source.to_string()
                    }
                }
            };

            // Generate transforms
            let transforms: Vec<TokenStream> = def
                .transforms
                .iter()
                .map(|t| match t {
                    Transform::Sort { field, order } => {
                        let order_expr = match order {
                            SortOrder::Asc => {
                                quote! { hyperstack::runtime::hyperstack_interpreter::ast::SortOrder::Asc }
                            }
                            SortOrder::Desc => {
                                quote! { hyperstack::runtime::hyperstack_interpreter::ast::SortOrder::Desc }
                            }
                        };
                        quote! {
                            hyperstack::runtime::hyperstack_interpreter::ast::ViewTransform::Sort {
                                key: #field,
                                order: #order_expr,
                            }
                        }
                    }
                    Transform::Filter { predicate } => {
                        // For now, filter predicates need to be pre-constructed Predicate expressions
                        // Users can use builder functions to construct predicates
                        quote! {
                            hyperstack::runtime::hyperstack_interpreter::ast::ViewTransform::Filter {
                                predicate: #predicate,
                            }
                        }
                    }
                    Transform::Take { count } => {
                        quote! {
                            hyperstack::runtime::hyperstack_interpreter::ast::ViewTransform::Take { count: #count }
                        }
                    }
                    Transform::Skip { count } => {
                        quote! {
                            hyperstack::runtime::hyperstack_interpreter::ast::ViewTransform::Skip { count: #count }
                        }
                    }
                    Transform::First => {
                        quote! { hyperstack::runtime::hyperstack_interpreter::ast::ViewTransform::First }
                    }
                    Transform::Last => {
                        quote! { hyperstack::runtime::hyperstack_interpreter::ast::ViewTransform::Last }
                    }
                    Transform::MaxBy { field } => {
                        quote! {
                            hyperstack::runtime::hyperstack_interpreter::ast::ViewTransform::MaxBy {
                                key: #field,
                            }
                        }
                    }
                    Transform::MinBy { field } => {
                        quote! {
                            hyperstack::runtime::hyperstack_interpreter::ast::ViewTransform::MinBy {
                                key: #field,
                            }
                        }
                    }
                })
                .collect();

            // Determine output mode based on transforms
            let has_single_transform = def.transforms.iter().any(|t| {
                matches!(
                    t,
                    Transform::First | Transform::Last | Transform::MaxBy { .. } | Transform::MinBy { .. }
                )
            });
            let output_expr = if has_single_transform {
                quote! { hyperstack::runtime::hyperstack_interpreter::ast::ViewOutput::Single }
            } else {
                quote! { hyperstack::runtime::hyperstack_interpreter::ast::ViewOutput::Collection }
            };

            quote! {
                hyperstack::runtime::hyperstack_interpreter::ast::ViewDef {
                    id: #name.to_string(),
                    source: #source_expr,
                    pipeline: vec![#(#transforms),*],
                    output: #output_expr,
                }
            }
        })
        .collect();

    quote! {
        vec![#(#view_defs),*]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_view() {
        let input: TokenStream = quote! {
            OreRound/latest = OreRound/list | first;
        };

        let result = syn::parse2::<ViewsInput>(input);
        assert!(result.is_ok());

        let views = result.unwrap();
        assert_eq!(views.definitions.len(), 1);
        assert_eq!(views.definitions[0].name, "OreRound/latest");
        assert_eq!(views.definitions[0].source, "OreRound/list");
        assert_eq!(views.definitions[0].transforms.len(), 1);
        assert!(matches!(
            views.definitions[0].transforms[0],
            Transform::First
        ));
    }

    #[test]
    fn test_parse_view_with_sort() {
        let input: TokenStream = quote! {
            OreRound/sorted = OreRound/list | sort(some_field(), Desc);
        };

        let result = syn::parse2::<ViewsInput>(input);
        assert!(result.is_ok());

        let views = result.unwrap();
        assert_eq!(views.definitions.len(), 1);
        assert_eq!(views.definitions[0].transforms.len(), 1);
        match &views.definitions[0].transforms[0] {
            Transform::Sort { order, .. } => {
                assert!(matches!(order, SortOrder::Desc));
            }
            _ => panic!("Expected Sort transform"),
        }
    }

    #[test]
    fn test_parse_multiple_views() {
        let input: TokenStream = quote! {
            View1/a = Source1/list | first;
            View2/b = Source2/list | last;
        };

        let result = syn::parse2::<ViewsInput>(input);
        assert!(result.is_ok());

        let views = result.unwrap();
        assert_eq!(views.definitions.len(), 2);
        assert_eq!(views.definitions[0].name, "View1/a");
        assert_eq!(views.definitions[1].name, "View2/b");
    }

    #[test]
    fn test_parse_pipeline_transforms() {
        let input: TokenStream = quote! {
            Test/view = Test/list | sort(field(), Asc) | take(10) | first;
        };

        let result = syn::parse2::<ViewsInput>(input);
        assert!(result.is_ok());

        let views = result.unwrap();
        assert_eq!(views.definitions[0].transforms.len(), 3);
        assert!(matches!(
            views.definitions[0].transforms[0],
            Transform::Sort { .. }
        ));
        assert!(matches!(
            views.definitions[0].transforms[1],
            Transform::Take { count: 10 }
        ));
        assert!(matches!(
            views.definitions[0].transforms[2],
            Transform::First
        ));
    }

    #[test]
    fn test_parse_derived_view() {
        let input: TokenStream = quote! {
            Derived/top5 = OreRound/sorted | take(5);
        };

        let result = syn::parse2::<ViewsInput>(input);
        assert!(result.is_ok());

        let views = result.unwrap();
        // Source is not /list or /state, so it should be treated as a derived view
        assert_eq!(views.definitions[0].source, "OreRound/sorted");
    }

    #[test]
    fn test_unknown_transform_error() {
        let input: TokenStream = quote! {
            Test/view = Test/list | unknown_transform;
        };

        let result = syn::parse2::<ViewsInput>(input);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown transform"));
    }
}
