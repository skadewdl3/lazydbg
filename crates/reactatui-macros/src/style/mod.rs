use proc_macro2::{Delimiter, Group, Ident, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote};
use syn::{Expr, Lit, Token, parse::Parser as _, punctuated::Punctuated};

use crate::layout::RuleTokens;

pub fn style(input: TokenStream2) -> TokenStream2 {
    match crate::layout::parse_style(input) {
        Ok(style) => style,
        Err(error) => error.to_compile_error(),
    }
}

pub(crate) fn property_value(property: &str, value: TokenStream2) -> syn::Result<RuleTokens> {
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

fn leaf_rule(property: &str, value: TokenStream2) -> syn::Result<RuleTokens> {
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

fn color_setter(method: &str, property: &str, value: TokenStream2) -> syn::Result<RuleTokens> {
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

fn parse_hex_digits(raw: &str) -> Option<(u8, u8, u8)> {
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

fn font_weight(value: TokenStream2) -> syn::Result<RuleTokens> {
    single_modifier_property(value, "font-weight", "normal", "bold", "BOLD")
}

fn font_style(value: TokenStream2) -> syn::Result<RuleTokens> {
    single_modifier_property(value, "font-style", "normal", "italic", "ITALIC")
}

fn visibility(value: TokenStream2) -> syn::Result<RuleTokens> {
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

fn text_decoration(value: TokenStream2) -> syn::Result<RuleTokens> {
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

fn text_style(value: TokenStream2) -> syn::Result<RuleTokens> {
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

fn all(value: TokenStream2) -> syn::Result<RuleTokens> {
    match single_name(&value, "all")?.as_str() {
        "initial" => Ok(RuleTokens::Replace(
            quote! { ::ratatui::style::Style::new() },
        )),
        "reset" => Ok(RuleTokens::Replace(
            quote! { ::ratatui::style::Style::reset() },
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

fn single_name(value: &TokenStream2, property: &str) -> syn::Result<String> {
    let names = parse_names(&value.clone().into_iter().collect::<Vec<_>>())?;
    if names.len() == 1 {
        Ok(names.into_iter().next().expect("one name"))
    } else {
        Err(invalid_value(value, property, "a single keyword"))
    }
}

fn parse_names(tokens: &[TokenTree]) -> syn::Result<Vec<String>> {
    let mut names = Vec::new();
    let mut pos = 0;
    while pos < tokens.len() {
        if matches!(&tokens[pos], TokenTree::Punct(punct) if punct.as_char() == ',') {
            pos += 1;
            continue;
        }
        let TokenTree::Ident(first) = &tokens[pos] else {
            return Err(syn::Error::new_spanned(
                tokens[pos].clone(),
                "expected a CSS keyword",
            ));
        };
        let mut name = first.to_string();
        pos += 1;
        while pos + 1 < tokens.len()
            && matches!(&tokens[pos], TokenTree::Punct(punct) if punct.as_char() == '-')
        {
            let TokenTree::Ident(part) = &tokens[pos + 1] else {
                break;
            };
            name.push('-');
            name.push_str(&part.to_string());
            pos += 2;
        }
        names.push(name);
    }
    Ok(names)
}

fn invalid_value(value: &TokenStream2, property: &str, expected: &str) -> syn::Error {
    syn::Error::new_spanned(
        value,
        format!(
            "invalid value `{value}` for style property `{property}`\nhelp: expected {expected}"
        ),
    )
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

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{parse_hex_digits, property_value};

    #[test]
    fn parses_short_and_long_hex() {
        assert_eq!(parse_hex_digits("f80"), Some((255, 136, 0)));
        assert_eq!(parse_hex_digits("Ff8800"), Some((255, 136, 0)));
        assert_eq!(parse_hex_digits("abcd"), None);
    }

    #[test]
    fn rejects_invalid_hex_with_expected_forms() {
        let value = "#abcd".parse().expect("valid token stream");
        let error = property_value("color", value).expect_err("four-digit hex should be rejected");
        let message = error.to_string();

        assert!(message.contains("invalid hex color `#abcd`"));
        assert!(message.contains("expected `#RGB` or `#RRGGBB`"));
    }

    #[test]
    fn rejects_out_of_range_literal_components() {
        let error = property_value("color", quote! { rgb(256, 0, 0) })
            .expect_err("out-of-range component should be rejected");

        assert!(
            error
                .to_string()
                .contains("RGB component must be in the range 0..=255")
        );
    }

    #[test]
    fn rejects_unknown_modifiers_with_property_context() {
        let error = property_value("text-style", quote! { bold sparkling })
            .expect_err("unknown modifier should be rejected");
        let message = error.to_string();

        assert!(message.contains("style property `text-style`"));
        assert!(message.contains("supported ratatui modifier names"));
    }

    #[test]
    fn patch_requires_a_braced_rust_expression() {
        let error = property_value("patch", quote! { base_style })
            .expect_err("unbraced patch should be rejected");

        assert!(error.to_string().contains("a braced Rust style expression"));
    }
}
