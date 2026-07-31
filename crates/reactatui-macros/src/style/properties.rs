//! Style property dispatch table.
//!
//! To add a new style property, add its CSS names here and implement the value
//! parser in a focused module. Parsers return [`RuleTokens`](crate::layout::RuleTokens)
//! so they can either call a `Style` setter, replace the style, or emit custom
//! statements when declaration order matters.

use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::quote;

use crate::layout::RuleTokens;

use super::block;
use super::color::color_setter;
use super::common::{invalid_value, single_name};
use super::modifiers::{font_style, font_weight, text_decoration, text_style, visibility};

pub(super) fn leaf_rule(property: &str, value: TokenStream2) -> syn::Result<RuleTokens> {
    if let Some(rule) = block::rule(property, value.clone())? {
        return Ok(rule);
    }

    match property {
        "color" | "fg" => color_setter("fg", property, value),
        "background-color" | "background" | "bg" => color_setter("bg", property, value),
        "text-decoration-color" | "underline-color" => {
            color_setter("underline_color", property, value)
        }
        "font-weight" => font_weight(value),
        "font-style" => font_style(value),
        "text-decoration-line" => text_decoration(value),
        "visibility" => visibility(value),
        "text-style" => text_style(value),
        "all" => all(value),
        "patch" => patch(value),
        _ => Err(syn::Error::new_spanned(
            value,
            format!("unknown style property `{property}`"),
        )),
    }
}

fn all(value: TokenStream2) -> syn::Result<RuleTokens> {
    match single_name(&value, "all")?.as_str() {
        "initial" => Ok(RuleTokens::Setter(
            quote! { base_style(::ratatui::style::Style::new()) },
        )),
        "reset" => Ok(RuleTokens::Setter(
            quote! { base_style(::ratatui::style::Style::reset()) },
        )),
        _ => Err(invalid_value(&value, "all", "`initial` or `reset`")),
    }
}

fn patch(value: TokenStream2) -> syn::Result<RuleTokens> {
    let tokens: Vec<_> = value.clone().into_iter().collect();
    if let [TokenTree::Group(group)] = tokens.as_slice()
        && group.delimiter() == Delimiter::Brace
    {
        let expression = group.stream();
        return Ok(RuleTokens::Setter(quote! { patch({ #expression }) }));
    }
    Err(invalid_value(
        &value,
        "patch",
        "a braced Rust style expression such as `{base_style}`",
    ))
}
