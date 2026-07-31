//! Shared style-property parsing utilities.

use proc_macro2::{TokenStream as TokenStream2, TokenTree};

pub(super) fn single_name(value: &TokenStream2, property: &str) -> syn::Result<String> {
    let names = parse_names(&value.clone().into_iter().collect::<Vec<_>>())?;
    if names.len() == 1 {
        Ok(names.into_iter().next().expect("one name"))
    } else {
        Err(invalid_value(value, property, "a single keyword"))
    }
}

pub(super) fn parse_names(tokens: &[TokenTree]) -> syn::Result<Vec<String>> {
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

pub(super) fn invalid_value(value: &TokenStream2, property: &str, expected: &str) -> syn::Error {
    syn::Error::new_spanned(
        value,
        format!(
            "invalid value `{value}` for style property `{property}`\nhelp: expected {expected}"
        ),
    )
}
