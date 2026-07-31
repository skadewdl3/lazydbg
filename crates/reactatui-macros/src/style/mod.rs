//! Implementation of the `style!` macro.
//!
//! Style parsing reuses the generic declaration-block parser from `layout`.
//! This module only coordinates style-specific property handling; the actual
//! value parsers live in focused submodules.

mod block;
mod color;
mod common;
mod conditional;
mod modifiers;
mod properties;

use proc_macro2::TokenStream as TokenStream2;

use crate::layout::RuleTokens;

pub fn style(input: TokenStream2) -> TokenStream2 {
    match crate::layout::parse_style(input) {
        Ok(style) => style,
        Err(error) => error.to_compile_error(),
    }
}

pub(crate) fn property_value(property: &str, value: TokenStream2) -> syn::Result<RuleTokens> {
    conditional::property_value(property, value)
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::{color::parse_hex_digits, property_value};

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
