//! Procedural macro for defining nix-flake-parts-lint rules.
//!
//! Usage:
//! ```ignore
//! #[lint(
//!     name = "no-with",
//!     note = "with expressions shadow namespace",
//!     code = 1,
//!     match_with = NODE_WITH
//! )]
//! /// ## What it does
//! /// Checks for `with` expressions.
//! struct NoWith;
//! ```

mod metadata;

use metadata::RawLintMeta;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemStruct};

/// Generate the explanation from doc comments.
fn generate_explain_impl(struct_item: &ItemStruct) -> TokenStream2 {
    let docs = struct_item
        .attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(meta) = &attr.meta {
                    if let syn::Expr::Lit(expr_lit) = &meta.value {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            return Some(lit_str.value());
                        }
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>()
        .join("\n");

    let struct_name = &struct_item.ident;
    quote! {
        impl ::nix_lint_core::Explain for #struct_name {
            fn explanation(&self) -> &'static str {
                #docs
            }
        }
    }
}

/// Generate impl Metadata from the attribute arguments.
fn generate_meta_impl(struct_name: &syn::Ident, meta: &RawLintMeta) -> TokenStream2 {
    metadata::generate_meta_impl(struct_name, meta)
}

/// Generate a simple `impl MyStruct { pub fn new() -> Self { Self } }`.
fn generate_self_impl(struct_name: &syn::Ident) -> TokenStream2 {
    quote! {
        impl #struct_name {
            pub fn new() -> Self {
                Self
            }
        }
    }
}

#[proc_macro_attribute]
pub fn lint(attr: TokenStream, item: TokenStream) -> TokenStream {
    let struct_item = parse_macro_input!(item as ItemStruct);
    let meta = parse_macro_input!(attr as RawLintMeta);

    let struct_name = &struct_item.ident;
    let self_impl = generate_self_impl(struct_name);
    let meta_impl = generate_meta_impl(struct_name, &meta);
    let explain_impl = generate_explain_impl(&struct_item);

   let output = quote! {
        #[derive(Clone, Copy)]
        #struct_item

        ::lazy_static::lazy_static! {
            pub static ref LINT: Box<dyn ::nix_lint_core::Lint> = Box::new(#struct_name::new());
        }

        #self_impl
        #meta_impl
        #explain_impl

        impl ::nix_lint_core::Lint for #struct_name {
            fn as_rule(&self) -> &dyn ::nix_lint_core::Rule { self }
        }

        impl ::nix_lint_core::Rule for #struct_name {
            fn validate(&self, node: &::rnix::SyntaxElement) -> Option<::nix_lint_core::Report> {
                self.check(node)
            }
        }
    };

    output.into()
}
