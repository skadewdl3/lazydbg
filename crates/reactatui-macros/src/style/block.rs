//! Ratatui block-related style properties.
//!
//! Each table row associates a CSS property with a builder method on
//! `ReactatuiStyle` and a focused value parser. The runtime registry generates
//! conversions to the concrete type accepted by the corresponding callback.

use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use serde::Deserialize;

use crate::layout::{CssValue, RuleTokens, deserialize_css, enum_value, properties};

use super::common::invalid_value;

pub(super) fn rule(property: &str, value: TokenStream2) -> syn::Result<Option<RuleTokens>> {
    deserialize_css::<Property>(property)
        .map(|property| property.emit(value).map(Some))
        .unwrap_or(Ok(None))
}

properties! {
    Property {
        Borders => ("borders", borders, BordersValue),
        BorderType => ("border-type", border_type, BorderTypeValue),
        BorderSet => ("border-set", border_set, BracedExpression),
        TitleAlignment => ("title-alignment", title_alignment, AlignmentValue),
        TitlePosition => ("title-position", title_position, TitlePositionValue),
        Padding => ("padding", padding, PaddingValue),
        MergeBorders => ("merge-borders", merge_borders, MergeStrategyValue),
        Shadow => ("shadow", shadow, ShadowValue),
    }
}

struct BordersValue;

impl CssValue for BordersValue {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        if is_braced_expression(&value) {
            return Ok(value);
        }

        let names = value
            .clone()
            .into_iter()
            .map(|token| match token {
                TokenTree::Ident(name) => Ok(name.to_string()),
                token => Err(syn::Error::new_spanned(token, "expected a border side")),
            })
            .collect::<syn::Result<Vec<_>>>()?;
        if names.is_empty() {
            return Err(invalid_value(
                &value,
                property,
                "`none`, `all`, one or more of `top right bottom left`, or `{...}`",
            ));
        }
        if names.len() == 1 && names[0] == "none" {
            return Ok(quote! { ::ratatui::widgets::Borders::NONE });
        }
        if names.len() == 1 && names[0] == "all" {
            return Ok(quote! { ::ratatui::widgets::Borders::ALL });
        }

        let sides = names
            .iter()
            .map(|name| match name.as_str() {
                "top" => Ok(quote! { ::ratatui::widgets::Borders::TOP }),
                "right" => Ok(quote! { ::ratatui::widgets::Borders::RIGHT }),
                "bottom" => Ok(quote! { ::ratatui::widgets::Borders::BOTTOM }),
                "left" => Ok(quote! { ::ratatui::widgets::Borders::LEFT }),
                _ => Err(invalid_value(
                    &value,
                    property,
                    "`none`, `all`, one or more of `top right bottom left`, or `{...}`",
                )),
            })
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(quote! {
            ::ratatui::widgets::Borders::NONE #(.union(#sides))*
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum BorderTypeValue {
    Plain,
    Rounded,
    Double,
    Thick,
    LightDoubleDashed,
    HeavyDoubleDashed,
    LightTripleDashed,
    HeavyTripleDashed,
    LightQuadrupleDashed,
    HeavyQuadrupleDashed,
    QuadrantInside,
    QuadrantOutside,
}

impl CssValue for BorderTypeValue {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        enum_value(
            value,
            |value| {
                let variant = match value {
                    BorderTypeValue::Plain => quote! { Plain },
                    BorderTypeValue::Rounded => quote! { Rounded },
                    BorderTypeValue::Double => quote! { Double },
                    BorderTypeValue::Thick => quote! { Thick },
                    BorderTypeValue::LightDoubleDashed => quote! { LightDoubleDashed },
                    BorderTypeValue::HeavyDoubleDashed => quote! { HeavyDoubleDashed },
                    BorderTypeValue::LightTripleDashed => quote! { LightTripleDashed },
                    BorderTypeValue::HeavyTripleDashed => quote! { HeavyTripleDashed },
                    BorderTypeValue::LightQuadrupleDashed => quote! { LightQuadrupleDashed },
                    BorderTypeValue::HeavyQuadrupleDashed => quote! { HeavyQuadrupleDashed },
                    BorderTypeValue::QuadrantInside => quote! { QuadrantInside },
                    BorderTypeValue::QuadrantOutside => quote! { QuadrantOutside },
                };
                quote! { ::ratatui::widgets::BorderType::#variant }
            },
            property,
            "a Ratatui border type such as `plain`, `rounded`, `double`, or `thick`",
        )
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum AlignmentValue {
    Left,
    Center,
    Right,
}

impl CssValue for AlignmentValue {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        enum_value(
            value,
            |value| match value {
                AlignmentValue::Left => quote! { ::ratatui::layout::Alignment::Left },
                AlignmentValue::Center => quote! { ::ratatui::layout::Alignment::Center },
                AlignmentValue::Right => quote! { ::ratatui::layout::Alignment::Right },
            },
            property,
            "`left`, `center`, or `right`",
        )
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum TitlePositionValue {
    Top,
    Bottom,
}

impl CssValue for TitlePositionValue {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        enum_value(
            value,
            |value| match value {
                TitlePositionValue::Top => quote! { ::ratatui::widgets::TitlePosition::Top },
                TitlePositionValue::Bottom => quote! { ::ratatui::widgets::TitlePosition::Bottom },
            },
            property,
            "`top` or `bottom`",
        )
    }
}

struct PaddingValue;

impl CssValue for PaddingValue {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        if is_braced_expression(&value) {
            return Ok(value);
        }
        let values = value
            .to_string()
            .split_whitespace()
            .map(str::parse::<u16>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                invalid_value(
                    &value,
                    property,
                    "one to four non-negative cell counts or `{...}`",
                )
            })?;
        match values.as_slice() {
            [all] => Ok(quote! { ::ratatui::widgets::Padding::uniform(#all) }),
            [vertical, horizontal] => {
                Ok(quote! { ::ratatui::widgets::Padding::symmetric(#horizontal, #vertical) })
            }
            [top, horizontal, bottom] => Ok(quote! {
                ::ratatui::widgets::Padding::new(#horizontal, #horizontal, #top, #bottom)
            }),
            [top, right, bottom, left] => Ok(quote! {
                ::ratatui::widgets::Padding::new(#left, #right, #top, #bottom)
            }),
            _ => Err(invalid_value(
                &value,
                property,
                "one to four non-negative cell counts or `{...}`",
            )),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum MergeStrategyValue {
    Replace,
    Exact,
    Fuzzy,
}

impl CssValue for MergeStrategyValue {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        enum_value(
            value,
            |value| match value {
                MergeStrategyValue::Replace => {
                    quote! { ::ratatui::symbols::merge::MergeStrategy::Replace }
                }
                MergeStrategyValue::Exact => {
                    quote! { ::ratatui::symbols::merge::MergeStrategy::Exact }
                }
                MergeStrategyValue::Fuzzy => {
                    quote! { ::ratatui::symbols::merge::MergeStrategy::Fuzzy }
                }
            },
            property,
            "`replace`, `exact`, or `fuzzy`",
        )
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ShadowValue {
    Overlay,
    Block,
    LightShade,
    MediumShade,
    DarkShade,
}

impl CssValue for ShadowValue {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        enum_value(
            value,
            |value| match value {
                ShadowValue::Overlay => quote! { ::ratatui::widgets::Shadow::overlay() },
                ShadowValue::Block => quote! { ::ratatui::widgets::Shadow::block() },
                ShadowValue::LightShade => quote! { ::ratatui::widgets::Shadow::light_shade() },
                ShadowValue::MediumShade => quote! { ::ratatui::widgets::Shadow::medium_shade() },
                ShadowValue::DarkShade => quote! { ::ratatui::widgets::Shadow::dark_shade() },
            },
            property,
            "`overlay`, `block`, `light-shade`, `medium-shade`, `dark-shade`, or `{...}`",
        )
    }
}

struct BracedExpression;

impl CssValue for BracedExpression {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        if let Some(expression) = braced_expression(&value) {
            Ok(expression)
        } else {
            Err(invalid_value(
                &value,
                property,
                "a braced Rust expression such as `{symbols::border::DOUBLE}`",
            ))
        }
    }
}

fn is_braced_expression(value: &TokenStream2) -> bool {
    braced_expression(value).is_some()
}

fn braced_expression(value: &TokenStream2) -> Option<TokenStream2> {
    matches!(
        value.clone().into_iter().collect::<Vec<_>>().as_slice(),
        [TokenTree::Group(group)] if group.delimiter() == Delimiter::Brace
    )
    .then(|| match value.clone().into_iter().next() {
        Some(TokenTree::Group(group)) => group.stream(),
        _ => unreachable!("braced expression shape was checked"),
    })
}
