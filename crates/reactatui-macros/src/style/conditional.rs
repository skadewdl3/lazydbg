//! Inline conditional values for style rules.

use proc_macro2::{Delimiter, Group, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote};

use crate::layout::RuleTokens;

use super::properties::leaf_rule;

pub(super) fn property_value(property: &str, value: TokenStream2) -> syn::Result<RuleTokens> {
    if starts_with_if(&value) {
        return conditional_rule(property, value);
    }
    leaf_rule(property, value)
}

fn conditional_rule(property: &str, value: TokenStream2) -> syn::Result<RuleTokens> {
    let ParsedIf {
        condition,
        then_value,
        else_value,
    } = parse_if(value)?;
    let then_rule = leaf_or_conditional_rule(property, then_value)?;
    let then_rule = emit_rule(then_rule);
    let else_rule = else_value
        .map(|value| leaf_or_conditional_rule(property, value))
        .transpose()?
        .map(emit_rule);

    Ok(RuleTokens::Raw(match else_rule {
        Some(else_rule) => quote! {
            if #condition {
                #then_rule
            } else {
                #else_rule
            }
        },
        None => quote! {
            if #condition {
                #then_rule
            }
        },
    }))
}

fn leaf_or_conditional_rule(property: &str, value: TokenStream2) -> syn::Result<RuleTokens> {
    if starts_with_if(&value) {
        conditional_rule(property, value)
    } else {
        leaf_rule(property, value)
    }
}

fn emit_rule(rule: RuleTokens) -> TokenStream2 {
    let style = format_ident!("__reactatui_style");
    match rule {
        RuleTokens::Setter(setter) => quote! { #style = #style.#setter; },
        RuleTokens::Replace(value) => quote! { #style = #value; },
        RuleTokens::Raw(statements) => statements,
    }
}

struct ParsedIf {
    condition: TokenStream2,
    then_value: TokenStream2,
    else_value: Option<TokenStream2>,
}

fn starts_with_if(value: &TokenStream2) -> bool {
    matches!(value.clone().into_iter().next(), Some(TokenTree::Ident(ident)) if ident == "if")
}

fn parse_if(value: TokenStream2) -> syn::Result<ParsedIf> {
    let tokens: Vec<_> = value.into_iter().collect();
    let mut pos = 1;
    let mut condition = TokenStream2::new();
    let then_value = loop {
        let Some(token) = tokens.get(pos).cloned() else {
            return Err(syn::Error::new_spanned(
                condition,
                "inline `if` requires a braced value",
            ));
        };
        pos += 1;
        if let TokenTree::Group(group) = &token
            && group.delimiter() == Delimiter::Brace
        {
            break group.stream();
        }
        condition.extend([token]);
    };
    if condition.is_empty() {
        return Err(syn::Error::new_spanned(
            then_value,
            "inline `if` requires a condition",
        ));
    }

    let else_value = if pos < tokens.len() {
        match tokens.get(pos) {
            Some(TokenTree::Ident(ident)) if ident == "else" => pos += 1,
            Some(token) => {
                return Err(syn::Error::new_spanned(
                    token,
                    "expected `else` after inline `if` value",
                ));
            }
            None => unreachable!(),
        }
        if matches!(tokens.get(pos), Some(TokenTree::Ident(ident)) if ident == "if") {
            let nested = tokens[pos..].iter().cloned().collect();
            pos = tokens.len();
            Some(nested)
        } else {
            let Some(TokenTree::Group(group)) = tokens.get(pos) else {
                return Err(syn::Error::new_spanned(
                    tokens.get(pos).cloned().unwrap_or_else(|| {
                        TokenTree::Group(Group::new(Delimiter::Brace, TokenStream2::new()))
                    }),
                    "`else` requires a braced value",
                ));
            };
            if group.delimiter() != Delimiter::Brace {
                return Err(syn::Error::new_spanned(
                    group,
                    "`else` requires a braced value",
                ));
            }
            pos += 1;
            Some(group.stream())
        }
    } else {
        None
    };
    if pos != tokens.len() {
        return Err(syn::Error::new_spanned(
            tokens[pos].clone(),
            "unexpected tokens after inline conditional value",
        ));
    }
    Ok(ParsedIf {
        condition,
        then_value,
        else_value,
    })
}
