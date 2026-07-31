//! Text modifier property parsers.

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};

use crate::layout::RuleTokens;

use super::common::{invalid_value, parse_names, single_name};

pub(super) fn font_weight(value: TokenStream2) -> syn::Result<RuleTokens> {
    single_modifier_property(value, "font-weight", "normal", "bold", "BOLD")
}

pub(super) fn font_style(value: TokenStream2) -> syn::Result<RuleTokens> {
    single_modifier_property(value, "font-style", "normal", "italic", "ITALIC")
}

pub(super) fn visibility(value: TokenStream2) -> syn::Result<RuleTokens> {
    single_modifier_property(value, "visibility", "visible", "hidden", "HIDDEN")
}

fn single_modifier_property(
    value: TokenStream2,
    property: &str,
    remove_value: &str,
    add_value: &str,
    modifier: &str,
) -> syn::Result<RuleTokens> {
    let name = single_name(&value, property)?;
    let modifier = format_ident!("{modifier}");
    if name == add_value {
        Ok(RuleTokens::Setter(quote! {
            add_modifier(::ratatui::style::Modifier::#modifier)
        }))
    } else if name == remove_value {
        Ok(RuleTokens::Setter(quote! {
            remove_modifier(::ratatui::style::Modifier::#modifier)
        }))
    } else {
        Err(invalid_value(
            &value,
            property,
            &format!("`{remove_value}` or `{add_value}`"),
        ))
    }
}

pub(super) fn text_decoration(value: TokenStream2) -> syn::Result<RuleTokens> {
    let names = parse_names(&value.clone().into_iter().collect::<Vec<_>>())?;
    if names.is_empty() {
        return Err(invalid_value(
            &value,
            "text-decoration-line",
            "`none` or a list of `underline`, `line-through`, `blink`, and `rapid-blink`",
        ));
    }
    if names.iter().any(|name| name == "none") && names.len() != 1 {
        return Err(syn::Error::new_spanned(
            value,
            "`none` cannot be combined with other text decorations",
        ));
    }

    let mut modifiers = Vec::new();
    for name in names {
        let modifier = match name.as_str() {
            "none" => continue,
            "underline" | "underlined" => "UNDERLINED",
            "line-through" | "strikethrough" | "crossed-out" => "CROSSED_OUT",
            "blink" | "slow-blink" => "SLOW_BLINK",
            "rapid-blink" => "RAPID_BLINK",
            _ => {
                return Err(invalid_value(
                    &value,
                    "text-decoration-line",
                    "`none` or a list of `underline`, `line-through`, `blink`, and `rapid-blink`",
                ));
            }
        };
        modifiers.push(format_ident!("{modifier}"));
    }

    let group = quote! {
        ::ratatui::style::Modifier::UNDERLINED
            | ::ratatui::style::Modifier::SLOW_BLINK
            | ::ratatui::style::Modifier::RAPID_BLINK
            | ::ratatui::style::Modifier::CROSSED_OUT
    };
    let add = modifier_union(&modifiers);
    if modifiers.is_empty() {
        Ok(RuleTokens::Setter(quote! { remove_modifier(#group) }))
    } else {
        Ok(RuleTokens::Setter(quote! {
            remove_modifier(#group).add_modifier(#add)
        }))
    }
}

pub(super) fn text_style(value: TokenStream2) -> syn::Result<RuleTokens> {
    let names = parse_names(&value.clone().into_iter().collect::<Vec<_>>())?;
    if names.is_empty() {
        return Err(invalid_value(
            &value,
            "text-style",
            "a space- or comma-separated modifier list",
        ));
    }

    let style = format_ident!("__reactatui_style");
    let all = all_modifiers();
    let mut statements = Vec::new();
    for name in names {
        if name == "none" {
            statements.push(quote! { #style = #style.remove_modifier(#all); });
            continue;
        }
        let (remove, modifier) = modifier_name(&name).ok_or_else(|| {
            invalid_value(
                &value,
                "text-style",
                "`none` or supported ratatui modifier names, optionally prefixed with `not-`",
            )
        })?;
        let modifier = format_ident!("{modifier}");
        if remove {
            statements.push(quote! {
                #style = #style.remove_modifier(::ratatui::style::Modifier::#modifier);
            });
        } else {
            statements.push(quote! {
                #style = #style.add_modifier(::ratatui::style::Modifier::#modifier);
            });
        }
    }
    Ok(RuleTokens::Raw(quote! { #(#statements)* }))
}

fn modifier_name(name: &str) -> Option<(bool, &'static str)> {
    let (remove, name) = name
        .strip_prefix("not-")
        .map_or((false, name), |name| (true, name));
    let modifier = match name {
        "bold" => "BOLD",
        "dim" => "DIM",
        "italic" => "ITALIC",
        "underline" | "underlined" => "UNDERLINED",
        "slow-blink" | "blink" => "SLOW_BLINK",
        "rapid-blink" => "RAPID_BLINK",
        "reversed" | "reverse" => "REVERSED",
        "hidden" => "HIDDEN",
        "crossed-out" | "line-through" | "strikethrough" => "CROSSED_OUT",
        _ => return None,
    };
    Some((remove, modifier))
}

fn all_modifiers() -> TokenStream2 {
    let modifiers = [
        "BOLD",
        "DIM",
        "ITALIC",
        "UNDERLINED",
        "SLOW_BLINK",
        "RAPID_BLINK",
        "REVERSED",
        "HIDDEN",
        "CROSSED_OUT",
    ]
    .map(|name| format_ident!("{name}"));
    modifier_union(&modifiers)
}

fn modifier_union(modifiers: &[Ident]) -> TokenStream2 {
    quote! { ::ratatui::style::Modifier::empty() #(| ::ratatui::style::Modifier::#modifiers)* }
}
