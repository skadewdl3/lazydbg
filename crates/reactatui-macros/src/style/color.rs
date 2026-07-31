//! Color property parsers.
//!
//! Supported values intentionally mirror the macro docs: named ANSI colors,
//! `rgb(r, g, b)`, `#RGB`, `#RRGGBB`, `indexed(n)`, or a braced Rust color
//! expression.

use proc_macro2::{Delimiter, Group, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote};
use syn::{Expr, Lit, Token, parse::Parser as _, punctuated::Punctuated};

use crate::layout::RuleTokens;

use super::common::parse_names;

pub(super) fn color_setter(
    method: &str,
    property: &str,
    value: TokenStream2,
) -> syn::Result<RuleTokens> {
    let color = parse_color(value, property)?;
    let method = format_ident!("{method}");
    Ok(RuleTokens::Setter(quote! { #method(#color) }))
}

fn parse_color(value: TokenStream2, property: &str) -> syn::Result<TokenStream2> {
    let tokens: Vec<_> = value.clone().into_iter().collect();

    if let [TokenTree::Group(group)] = tokens.as_slice()
        && group.delimiter() == Delimiter::Brace
    {
        let expression = group.stream();
        return Ok(quote! {{ #expression }});
    }

    if tokens.len() == 2 && matches!(&tokens[0], TokenTree::Punct(punct) if punct.as_char() == '#')
    {
        return parse_hex(&tokens[1]);
    }

    if let [TokenTree::Ident(function), TokenTree::Group(arguments)] = tokens.as_slice()
        && arguments.delimiter() == Delimiter::Parenthesis
    {
        return match function.to_string().as_str() {
            "rgb" => parse_rgb(arguments),
            "indexed" => parse_indexed(arguments),
            _ => Err(invalid_color(&value, property)),
        };
    }

    if let Some(name) = parse_names(&tokens)
        .ok()
        .and_then(|names| (names.len() == 1).then(|| names.into_iter().next().expect("one name")))
        && let Some(variant) = named_color(&name)
    {
        let variant = format_ident!("{variant}");
        return Ok(quote! { ::ratatui::style::Color::#variant });
    }

    Err(invalid_color(&value, property))
}

fn parse_hex(token: &TokenTree) -> syn::Result<TokenStream2> {
    let raw = token.to_string();
    let Some((r, g, b)) = parse_hex_digits(&raw) else {
        return Err(syn::Error::new_spanned(
            token,
            format!(
                "invalid hex color `#{raw}`\nhelp: expected `#RGB` or `#RRGGBB`, such as `#f80` or `#ff8800`"
            ),
        ));
    };
    Ok(quote! { ::ratatui::style::Color::Rgb(#r, #g, #b) })
}

pub(super) fn parse_hex_digits(raw: &str) -> Option<(u8, u8, u8)> {
    let hex = raw.as_bytes();
    let digit = |value| match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    };
    match hex.len() {
        3 => {
            let r = digit(hex[0])?;
            let g = digit(hex[1])?;
            let b = digit(hex[2])?;
            Some((r * 17, g * 17, b * 17))
        }
        6 => {
            let pair = |index| Some(digit(hex[index])? * 16 + digit(hex[index + 1])?);
            Some((pair(0)?, pair(2)?, pair(4)?))
        }
        _ => None,
    }
}

fn parse_rgb(group: &Group) -> syn::Result<TokenStream2> {
    let values = Punctuated::<Expr, Token![,]>::parse_terminated.parse2(group.stream())?;
    if values.len() != 3 {
        return Err(syn::Error::new_spanned(
            group,
            format!(
                "`rgb(...)` requires exactly 3 components, found {}",
                values.len()
            ),
        ));
    }
    for value in &values {
        validate_u8_literal(value, "RGB component")?;
    }
    let values: Vec<_> = values.into_iter().collect();
    let (r, g, b) = (&values[0], &values[1], &values[2]);
    Ok(quote! { ::ratatui::style::Color::Rgb(#r, #g, #b) })
}

fn parse_indexed(group: &Group) -> syn::Result<TokenStream2> {
    let value: Expr = syn::parse2(group.stream())?;
    validate_u8_literal(&value, "indexed color")?;
    Ok(quote! { ::ratatui::style::Color::Indexed(#value) })
}

fn validate_u8_literal(value: &Expr, label: &str) -> syn::Result<()> {
    if let Expr::Lit(literal) = value
        && let Lit::Int(integer) = &literal.lit
        && integer.base10_parse::<u8>().is_err()
    {
        return Err(syn::Error::new_spanned(
            value,
            format!("{label} must be in the range 0..=255"),
        ));
    }
    Ok(())
}

fn invalid_color(value: &TokenStream2, property: &str) -> syn::Error {
    syn::Error::new_spanned(
        value,
        format!(
            "invalid color `{value}` for style property `{property}`\nhelp: expected a named color, `rgb(r, g, b)`, `#RGB`, `#RRGGBB`, `indexed(n)`, or `{{Color expression}}`"
        ),
    )
}

fn named_color(name: &str) -> Option<&'static str> {
    Some(match name {
        "reset" => "Reset",
        "black" => "Black",
        "red" => "Red",
        "green" => "Green",
        "yellow" => "Yellow",
        "blue" => "Blue",
        "magenta" => "Magenta",
        "cyan" => "Cyan",
        "gray" | "grey" | "silver" => "Gray",
        "dark-gray" | "dark-grey" | "darkgray" | "darkgrey" | "light-black" | "bright-black" => {
            "DarkGray"
        }
        "light-red" | "bright-red" => "LightRed",
        "light-green" | "bright-green" => "LightGreen",
        "light-yellow" | "bright-yellow" => "LightYellow",
        "light-blue" | "bright-blue" => "LightBlue",
        "light-magenta" | "bright-magenta" => "LightMagenta",
        "light-cyan" | "bright-cyan" => "LightCyan",
        "white" | "light-white" | "bright-white" => "White",
        _ => return None,
    })
}
