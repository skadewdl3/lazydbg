//! Proc-macros for the reactatui crate.
//!
//! Provides the `tui!` macro for declaring TUI node trees and the `#[component]`
//! attribute macro for functional components.

mod layout;
mod style;
mod template;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

use crate::template::{Parser, gen_fragment, gen_node};

/// Declares a tree of TUI nodes using an HTML-like JSX syntax.
///
/// Supports basic control flow like `if` and `for`, and seamlessly integrates
/// custom components and Ratatui widgets.
#[proc_macro]
pub fn tui(input: TokenStream) -> TokenStream {
    match Parser::new(input.into()).parse_nodes_until_close(None) {
        Ok(nodes) => {
            let output = if nodes.len() == 1 {
                gen_node(&nodes[0])
            } else {
                gen_fragment(&nodes)
            };
            output.into()
        }
        Err(error) => error.to_compile_error().into(),
    }
}

/// Builds a `reactatui::layout::Style` from CSS-like layout declarations.
#[proc_macro]
pub fn layout(input: TokenStream) -> TokenStream {
    layout::layout(input.into()).into()
}

/// A react-esque functional component that tracks state automatically.
/// It injects a guard at the top of the function to push the component's unique
/// context to the hook runtime stack.
#[proc_macro_attribute]
pub fn component(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(input as ItemFn);

    let mut errors = Vec::new();

    // Strip component parameter marker attributes from the final function signature.
    for input_arg in &mut func.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input_arg {
            let has_prop_like_attr = pat_type.attrs.iter().any(|attr| {
                attr.path().is_ident("prop")
                    || attr.path().is_ident("bind")
                    || attr.path().is_ident("slot")
            });

            if has_prop_like_attr
                && matches!(pat_type.pat.as_ref(), syn::Pat::Ident(pat) if pat.ident == "style" || pat.ident == "layout")
            {
                let name = match pat_type.pat.as_ref() {
                    syn::Pat::Ident(pat) => pat.ident.to_string(),
                    _ => String::new(),
                };
                errors.push(syn::Error::new_spanned(
                    &pat_type.pat,
                    format!("'{name}' is a reserved prop name and cannot be declared as a #[prop]"),
                ));
            }

            pat_type.attrs.retain(|attr| {
                !(attr.path().is_ident("children")
                    || attr.path().is_ident("prop")
                    || attr.path().is_ident("bind")
                    || attr.path().is_ident("slot"))
            });
        }
    }

    if !errors.is_empty() {
        let compile_errors = errors.into_iter().map(|error| error.to_compile_error());
        return quote! { #(#compile_errors)* #func }.into();
    }

    // Build the name as a string literal for the runtime id hash.
    let fn_name = func.sig.ident.to_string();

    // Prepend `let _guard = ::reactatui::hooks::__enter_component("<name>");`
    // to the existing function body. The ComponentGuard handles popping the id
    // off the runtime stack.
    let guard_stmt: syn::Stmt = syn::parse_quote! {
        let _guard = ::reactatui::hooks::__enter_component(#fn_name);
    };
    func.block.stmts.insert(0, guard_stmt);

    // Automatically allow non-snake case naming for React-like component names.
    func.attrs.push(syn::parse_quote! {
        #[allow(non_snake_case)]
    });

    quote!(#func).into()
}

/// An attribute marker on component arguments to accept child nodes.
#[proc_macro_attribute]
pub fn children(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// An attribute marker on component arguments to mark them as props
/// instead of constructor arguments
#[proc_macro_attribute]
pub fn prop(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// An attribute marker on component arguments to mark them as bindable props.
#[proc_macro_attribute]
pub fn bind(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// An attribute marker on component arguments to accept a named slot.
#[proc_macro_attribute]
pub fn slot(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    input
}
