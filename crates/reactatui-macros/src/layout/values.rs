//! CSS-like value parsers shared by layout property modules.
//!
//! Property tables stay small by naming a [`CssValue`] implementation. Each
//! implementation receives the raw tokens for a rule value and either converts
//! a CSS-looking value into Rust tokens or leaves a Rust expression untouched.

use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::quote;
use serde::Deserialize;
use serde::de::value::{BorrowedStrDeserializer, Error as SerdeValueError};

use super::parser::{Parser, Target};
use super::{flex, invalid_layout_value};

pub(crate) trait CssValue {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2>;
}

/// Passes the rule value through unchanged.
pub(crate) struct RustExpr;

impl CssValue for RustExpr {
    fn parse_tokens(value: TokenStream2, _: &'static str) -> syn::Result<TokenStream2> {
        Ok(value)
    }
}

/// Parses grid row/column track lists.
pub(crate) struct TrackList;

impl CssValue for TrackList {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        let tokens: Vec<_> = value.clone().into_iter().collect();
        if is_css_track_list(&tokens) {
            let tracks = value.to_string();
            if tracks
                .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
                .filter(|track| !track.is_empty())
                .all(|track| deserialize_css::<flex::Size>(track).is_some())
            {
                let sizes = tracks
                    .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
                    .filter(|track| !track.is_empty())
                    .map(
                        |track| match deserialize_css::<flex::Size>(track).expect("validated") {
                            flex::Size::Auto => quote! { ::reactatui::layout::Size::Auto },
                            flex::Size::Length(value) => {
                                quote! { ::reactatui::layout::Size::Length(#value) }
                            }
                            flex::Size::Fr(value) => {
                                quote! { ::reactatui::layout::Size::Fr(#value) }
                            }
                            flex::Size::Percent(value) => {
                                quote! { ::reactatui::layout::Size::Percent(#value) }
                            }
                        },
                    )
                    .collect::<Vec<_>>();
                return Ok(quote! { vec![#(#sizes),*] });
            }

            return Err(invalid_layout_value(
                &value,
                property,
                "a space- or comma-separated list of `auto`, integers, `<number>fr`, or `<number>%` tracks",
            ));
        }

        Ok(value)
    }
}

pub(crate) fn enum_value<T>(
    value: TokenStream2,
    mapper: fn(T) -> TokenStream2,
    property: &'static str,
    expected: &'static str,
) -> syn::Result<TokenStream2>
where
    T: for<'de> Deserialize<'de>,
{
    parse_css_value(
        value,
        move |name| deserialize_css::<T>(name).map(mapper),
        |value| invalid_layout_value(value, property, expected),
    )
}

pub(crate) fn parse_css_value(
    value: TokenStream2,
    mapper: impl Fn(&str) -> Option<TokenStream2> + Copy,
    invalid: impl Fn(&TokenStream2) -> syn::Error + Copy,
) -> syn::Result<TokenStream2> {
    let tokens: Vec<_> = value.into_iter().collect();
    if starts_ident(&tokens, "if") {
        return parse_inline_if(tokens, mapper, invalid);
    }

    let name = css_value_from_tokens(&tokens);
    if let Some(name) = name
        && let Some(mapped) = mapper(&name)
    {
        return Ok(mapped);
    }

    if is_css_value(&tokens) {
        return Err(invalid(&tokens.into_iter().collect()));
    }

    Ok(tokens.into_iter().collect())
}

fn parse_inline_if(
    tokens: Vec<TokenTree>,
    mapper: impl Fn(&str) -> Option<TokenStream2> + Copy,
    invalid: impl Fn(&TokenStream2) -> syn::Error + Copy,
) -> syn::Result<TokenStream2> {
    let mut parser = Parser {
        tokens,
        pos: 0,
        target: Target::Layout,
    };
    parser.expect_keyword("if")?;
    let (condition, body) = parser.collect_until_brace_group()?;
    let then_value = parse_css_value(body, mapper, invalid)?;

    let else_value = if parser.peek_ident("else") {
        parser.expect_keyword("else")?;
        if parser.peek_ident("if") {
            let nested = parse_inline_if(parser.tokens[parser.pos..].to_vec(), mapper, invalid)?;
            parser.pos = parser.tokens.len();
            Some(nested)
        } else {
            Some(parse_css_value(
                parser.expect_brace_group()?,
                mapper,
                invalid,
            )?)
        }
    } else {
        None
    };

    if !parser.is_done() {
        return Err(parser.error("unexpected tokens after inline if value"));
    }

    Ok(match else_value {
        Some(else_value) => quote! {
            if #condition {
                #then_value
            } else {
                #else_value
            }
        },
        None => quote! {
            if #condition {
                #then_value
            }
        },
    })
}

pub(crate) fn deserialize_css<T>(value: &str) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    T::deserialize(BorrowedStrDeserializer::<SerdeValueError>::new(value)).ok()
}

pub(crate) fn css_value_from_tokens(tokens: &[TokenTree]) -> Option<String> {
    if let [TokenTree::Literal(literal)] = tokens {
        return Some(literal.to_string());
    }

    if let [TokenTree::Literal(literal), TokenTree::Punct(punct)] = tokens
        && punct.as_char() == '%'
    {
        return Some(format!("{}%", literal));
    }

    let mut out = String::new();
    let mut expect_ident = true;

    for token in tokens {
        match token {
            TokenTree::Ident(ident) if expect_ident => {
                if !out.is_empty() {
                    out.push('-');
                }
                out.push_str(&ident.to_string());
                expect_ident = false;
            }
            TokenTree::Punct(punct) if punct.as_char() == '-' && !expect_ident => {
                expect_ident = true;
            }
            _ => return None,
        }
    }

    (!out.is_empty() && !expect_ident).then_some(out)
}

fn starts_ident(tokens: &[TokenTree], ident: &str) -> bool {
    matches!(tokens.first(), Some(TokenTree::Ident(first)) if first == ident)
}

fn is_css_value(tokens: &[TokenTree]) -> bool {
    !tokens.is_empty()
        && tokens.iter().all(|token| {
            matches!(token, TokenTree::Ident(_) | TokenTree::Literal(_))
                || matches!(token, TokenTree::Punct(punct) if matches!(punct.as_char(), '-' | '%'))
        })
}

fn is_css_track_list(tokens: &[TokenTree]) -> bool {
    !tokens.is_empty()
        && tokens.iter().all(|token| {
            matches!(token, TokenTree::Ident(_) | TokenTree::Literal(_))
                || matches!(token, TokenTree::Punct(punct) if matches!(punct.as_char(), '-' | '%' | ','))
        })
}
