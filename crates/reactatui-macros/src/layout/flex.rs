use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use serde::Deserialize;
use serde::de::{self, Visitor};
use std::fmt;

use crate::layout::{
    CssValue, RuleTokens, RustExpr, deserialize_css, enum_value, parse_css_value, properties,
};

pub fn rule(property: &str, value: TokenStream2) -> syn::Result<Option<RuleTokens>> {
    deserialize_css::<Property>(property)
        .map(|property| property.emit(value).map(Some))
        .unwrap_or(Ok(None))
}

properties! {
    Property {
        Direction => ("direction", direction, Direction),
        JustifyContent => ("justify-content", justify_content, Justify),
        AlignContent => ("align-content", align_content, Justify),
        AlignItems => ("align-items", align_items, Align),
        JustifyItems => ("justify-items", justify_items, Align),
        Gap => ("gap", gap, RustExpr),
        GapX => ("gap-x", gap_x, RustExpr),
        GapY => ("gap-y", gap_y, RustExpr),
        Padding => ("padding", padding, RustExpr),
        PaddingTop => ("padding-top", padding_top, RustExpr),
        PaddingRight => ("padding-right", padding_right, RustExpr),
        PaddingBottom => ("padding-bottom", padding_bottom, RustExpr),
        PaddingLeft => ("padding-left", padding_left, RustExpr),
        AlignSelf => ("align-self", align_self, Align),
        JustifySelf => ("justify-self", justify_self, Align),
        Ignore => ("ignore", ignore, RustExpr),
        Size => ("size", size, Size),
        Width => ("width", width, Size),
        Height => ("height", height, Size),
        MinWidth => ("min-width", min_width, RustExpr),
        MaxWidth => ("max-width", max_width, RustExpr),
        MinHeight => ("min-height", min_height, RustExpr),
        MaxHeight => ("max-height", max_height, RustExpr),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Direction {
    Horizontal,
    Vertical,
}

impl CssValue for Direction {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        enum_value(
            value,
            |value| match value {
                Direction::Horizontal => quote! { ::ratatui::layout::Direction::Horizontal },
                Direction::Vertical => quote! { ::ratatui::layout::Direction::Vertical },
            },
            property,
            "`horizontal` or `vertical`",
        )
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Justify {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl CssValue for Justify {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        enum_value(
            value,
            |value| match value {
                Justify::Start => quote! { ::reactatui::layout::style::Justify::Start },
                Justify::End => quote! { ::reactatui::layout::style::Justify::End },
                Justify::Center => quote! { ::reactatui::layout::style::Justify::Center },
                Justify::SpaceBetween => {
                    quote! { ::reactatui::layout::style::Justify::SpaceBetween }
                }
                Justify::SpaceAround => {
                    quote! { ::reactatui::layout::style::Justify::SpaceAround }
                }
                Justify::SpaceEvenly => {
                    quote! { ::reactatui::layout::style::Justify::SpaceEvenly }
                }
            },
            property,
            "`start`, `end`, `center`, `space-between`, `space-around`, or `space-evenly`",
        )
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Align {
    Start,
    End,
    Center,
    Stretch,
}

impl CssValue for Align {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        enum_value(
            value,
            |value| match value {
                Align::Start => quote! { ::reactatui::layout::style::Align::Start },
                Align::End => quote! { ::reactatui::layout::style::Align::End },
                Align::Center => quote! { ::reactatui::layout::style::Align::Center },
                Align::Stretch => quote! { ::reactatui::layout::style::Align::Stretch },
            },
            property,
            "`start`, `end`, `center`, or `stretch`",
        )
    }
}

pub(crate) enum Size {
    Auto,
    Length(u16),
    Fr(u16),
    Percent(u16),
}

impl<'de> Deserialize<'de> for Size {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(SizeVisitor)
    }
}

struct SizeVisitor;

impl Visitor<'_> for SizeVisitor {
    type Value = Size;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a layout size like auto, 3, 1fr, or 10%")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = value.trim();
        if value.eq_ignore_ascii_case("auto") {
            return Ok(Size::Auto);
        }
        if let Some(percent) = value.strip_suffix('%') {
            return parse_u16(percent, value).map(Size::Percent);
        }
        if let Some(fr) = value.strip_suffix("fr") {
            return parse_u16(fr, value).map(|fr| Size::Fr(fr.max(1)));
        }
        parse_u16(value, value).map(Size::Length)
    }
}

fn parse_u16<E>(value: &str, original: &str) -> Result<u16, E>
where
    E: de::Error,
{
    value
        .trim()
        .parse()
        .map_err(|_| E::custom(format!("invalid layout size `{original}`")))
}

impl CssValue for Size {
    fn parse_tokens(value: TokenStream2, property: &'static str) -> syn::Result<TokenStream2> {
        parse_css_value(
            value,
            |value| {
                deserialize_css::<Size>(value).map(|value| match value {
                    Size::Auto => quote! { ::reactatui::layout::Size::Auto },
                    Size::Length(length) => quote! { ::reactatui::layout::Size::Length(#length) },
                    Size::Fr(fr) => quote! { ::reactatui::layout::Size::Fr(#fr) },
                    Size::Percent(percent) => {
                        quote! { ::reactatui::layout::Size::Percent(#percent) }
                    }
                })
            },
            |value| {
                super::invalid_layout_value(
                    value,
                    property,
                    "`auto`, a non-negative integer, `<number>fr`, or `<number>%`",
                )
            },
        )
    }
}
