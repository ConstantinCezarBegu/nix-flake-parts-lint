//! Parsing and code generation for the `#[lint(...)]` attribute.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, Ident,
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    token,
};

struct KeyValue {
    key: Ident,
    _eq: token::Eq,
    value: Expr,
}

impl Parse for KeyValue {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            key: input.parse()?,
            _eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

pub struct RawLintMeta(std::collections::HashMap<Ident, Expr>);

impl Parse for RawLintMeta {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(
            Punctuated::<KeyValue, token::Comma>::parse_terminated(input)?
                .into_iter()
                .map(|item| (item.key, item.value))
                .collect(),
        ))
    }
}

fn extract_str(raw: &RawLintMeta, id: &str) -> String {
    raw.0
        .get(&syn::Ident::new(id, proc_macro2::Span::call_site()))
        .expect("`{id}` required in #[lint(...)]")
        .clone()
        .into_syn_lit_str()
        .expect("`{id}` must be a string literal")
        .value()
}

fn extract_u32(raw: &RawLintMeta, id: &str) -> u32 {
    raw.0
        .get(&syn::Ident::new(id, proc_macro2::Span::call_site()))
        .expect("`{id}` required in #[lint(...)]")
        .clone()
        .into_syn_lit_expr()
        .expect("`{id}` must be a numeric literal")
        .lit
        .clone()
        .into_syn_lit_int()
        .expect("`{id}` must be an integer")
        .base10_parse()
        .expect("`{id}` must be a valid integer")
}

fn extract_match_with(raw: &RawLintMeta) -> MatchWith {
    let expr = raw
        .0
        .get(&syn::Ident::new(
            "match_with",
            proc_macro2::Span::call_site(),
        ))
        .expect("`match_with` required in #[lint(...)]")
        .clone();

    match expr {
        Expr::Path(p) => MatchWith::Single(
            p.path
                .segments
                .last()
                .map(|s| s.ident.clone())
                .expect("match_with path must have a segment"),
        ),
        Expr::Array(a) => MatchWith::Multiple(
            a.elems
                .iter()
                .filter_map(|e| {
                    if let Expr::Path(p) = e {
                        p.path.segments.last().map(|s| s.ident.clone())
                    } else {
                        None
                    }
                })
                .collect(),
        ),
        _ => panic!("match_with must be a path (single) or array (multiple)"),
    }
}

fn extract_severity(raw: &RawLintMeta) -> TokenStream2 {
    let default_severity = quote! { ::nix_lint_core::Severity::Warn };

    raw.0
        .get(&syn::Ident::new("severity", proc_macro2::Span::call_site()))
        .map(|expr| {
            if let Expr::Path(p) = expr {
                p.path
                    .segments
                    .last()
                    .map(|s| {
                        let seg_name = s.ident.to_string();
                        match seg_name.as_str() {
                            "Error" => quote! { ::nix_lint_core::Severity::Error },
                            "Hint" => quote! { ::nix_lint_core::Severity::Hint },
                            _ => default_severity.clone(),
                        }
                    })
                    .unwrap_or(default_severity.clone())
            } else {
                default_severity.clone()
            }
        })
        .unwrap_or(default_severity)
}

trait IntoSynLitStr {
    fn into_syn_lit_str(self) -> Option<syn::LitStr>;
}

impl IntoSynLitStr for Expr {
    fn into_syn_lit_str(self) -> Option<syn::LitStr> {
        if let Expr::Lit(expr_lit) = self
            && let syn::Lit::Str(lit_str) = expr_lit.lit
        {
            return Some(lit_str);
        }
        None
    }
}

trait IntoSynLitExpr {
    fn into_syn_lit_expr(self) -> Option<syn::ExprLit>;
}

impl IntoSynLitExpr for Expr {
    fn into_syn_lit_expr(self) -> Option<syn::ExprLit> {
        if let Expr::Lit(expr_lit) = self {
            return Some(expr_lit);
        }
        None
    }
}

trait IntoSynLitInt {
    fn into_syn_lit_int(self) -> Option<syn::LitInt>;
}

impl IntoSynLitInt for syn::Lit {
    fn into_syn_lit_int(self) -> Option<syn::LitInt> {
        if let syn::Lit::Int(lit_int) = self {
            return Some(lit_int);
        }
        None
    }
}

enum MatchWith {
    Single(Ident),
    Multiple(Vec<Ident>),
}

fn generate_match_with_fn(match_with: &MatchWith) -> TokenStream2 {
    match match_with {
        MatchWith::Single(ident) => {
            quote! {
                fn match_with(&self, kind: &crate::rnix::SyntaxKind) -> bool {
                    *kind == crate::rnix::SyntaxKind::#ident
                }
            }
        }
        MatchWith::Multiple(idents) => {
            let kinds: Vec<_> = idents
                .iter()
                .map(|ident| {
                    quote! { crate::rnix::SyntaxKind::#ident }
                })
                .collect();
            quote! {
                fn match_with(&self, kind: &crate::rnix::SyntaxKind) -> bool {
                    [ #( #kinds ),* ].contains(kind)
                }
            }
        }
    }
}

fn generate_match_kind_fn(match_with: &MatchWith) -> TokenStream2 {
    match match_with {
        MatchWith::Single(ident) => {
            quote! { fn match_kind(&self) -> Vec<crate::rnix::SyntaxKind> { vec![crate::rnix::SyntaxKind::#ident] } }
        }
        MatchWith::Multiple(idents) => {
            let kinds: Vec<_> = idents
                .iter()
                .map(|ident| {
                    quote! { crate::rnix::SyntaxKind::#ident }
                })
                .collect();
            quote! { fn match_kind(&self) -> Vec<crate::rnix::SyntaxKind> { vec![ #( #kinds ),* ] } }
        }
    }
}

/// Generate `impl Metadata for MyStruct { ... }` from parsed attribute data.
pub fn generate_meta_impl(struct_name: &syn::Ident, raw: &RawLintMeta) -> TokenStream2 {
    let name = extract_str(raw, "name");
    let note = extract_str(raw, "note");
    let code = extract_u32(raw, "code");
    let match_with = extract_match_with(raw);
    let severity = extract_severity(raw);

    let name_fn = quote! { fn name(&self) -> &'static str { #name } };
    let note_fn = quote! { fn note(&self) -> &'static str { #note } };
    let code_fn = quote! { fn code(&self) -> u32 { #code } };
    let severity_fn = quote! { fn severity(&self) -> ::nix_lint_core::Severity { #severity } };
    let match_with_fn = generate_match_with_fn(&match_with);
    let match_kind_fn = generate_match_kind_fn(&match_with);
    let report_fn = quote! {
        fn report(&self) -> ::nix_lint_core::Report {
            ::nix_lint_core::Report::new(#note, #code, #severity)
        }
    };

    quote! {
        impl ::nix_lint_core::Metadata for #struct_name {
            #name_fn
            #note_fn
            #code_fn
            #severity_fn
            #match_with_fn
            #match_kind_fn
            #report_fn
        }
    }
}
