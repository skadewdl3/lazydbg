//! Implementation of the `layout!` macro.
//!
//! The layout macro is built from three layers:
//! - [`parser`] understands declaration-block syntax shared with `style!`.
//! - [`values`] converts CSS-like values into Rust layout values.
//! - property modules such as [`flex`] and [`grid`] map CSS property names to
//!   builder methods on `reactatui::layout::Style`.

mod flex;
mod grid;
mod parser;
mod properties;
mod values;

use proc_macro2::TokenStream as TokenStream2;

pub(crate) use parser::parse_style;
use parser::{CssName, Parser, Target};
pub(crate) use properties::properties;
pub(crate) use values::{
    CssValue, RustExpr, TrackList, deserialize_css, enum_value, parse_css_value,
};

pub fn layout(input: TokenStream2) -> TokenStream2 {
    match Parser::new(input, Target::Layout).parse() {
        Ok(styles) => styles,
        Err(error) => error.to_compile_error(),
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum RuleTokens {
    Setter(TokenStream2),
    Replace(TokenStream2),
    Raw(TokenStream2),
}

fn property_value(target: Target, name: &CssName, value: TokenStream2) -> syn::Result<RuleTokens> {
    let property = name.as_kebab();
    match target {
        Target::Layout => {
            if let Some(rule) = flex::rule(&property, value.clone())? {
                return Ok(rule);
            }
            if let Some(rule) = grid::rule(&property, value)? {
                return Ok(rule);
            }
        }
        Target::Style => return crate::style::property_value(&property, value),
    }

    Err(syn::Error::new_spanned(
        name.first_part(),
        format!("unknown {} property `{property}`", target.name()),
    ))
}

fn invalid_layout_value(value: &TokenStream2, property: &str, expected: &str) -> syn::Error {
    let display_value = value.to_string();
    syn::Error::new_spanned(
        value,
        format!(
            "invalid value `{display_value}` for layout property `{property}`\nhelp: expected {expected}\nhelp: use `{{...}}` to pass a Rust expression instead"
        ),
    )
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::parser::{Parser, Target};

    #[test]
    fn invalid_enum_value_explains_the_property_and_fix() {
        let error = Parser::new(quote! { direction: diagonal; }, Target::Layout)
            .parse()
            .expect_err("invalid direction should fail");
        let message = error.to_string();

        assert!(message.contains("invalid value `diagonal` for layout property `direction`"));
        assert!(message.contains("expected `horizontal` or `vertical`"));
        assert!(message.contains("use `{...}` to pass a Rust expression instead"));
    }

    #[test]
    fn invalid_track_value_explains_the_expected_syntax() {
        let error = Parser::new(quote! { columns: 1fr fill; }, Target::Layout)
            .parse()
            .expect_err("invalid track should fail");
        let message = error.to_string();

        assert!(message.contains("layout property `columns`"));
        assert!(message.contains("space- or comma-separated list"));
    }
}
