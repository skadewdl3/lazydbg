//! Proc-macros for the reactatui crate.
//!
//! Provides the `tui!` macro for declaring TUI node trees and the `#[component]`
//! attribute macro for functional components.

mod layout;
mod style;
mod template;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, GenericArgument, ItemFn, LitStr, PathArguments, Type, parse_macro_input};

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

/// Compiles a literal key or chord specification into a promoted static slice.
#[doc(hidden)]
#[proc_macro]
pub fn key_pattern(input: TokenStream) -> TokenStream {
    let literal = parse_macro_input!(input as LitStr);
    match compile_key_pattern(&literal.value()) {
        Ok(pattern) => pattern.into(),
        Err(message) => syn::Error::new(literal.span(), message)
            .to_compile_error()
            .into(),
    }
}

fn compile_key_pattern(spec: &str) -> Result<proc_macro2::TokenStream, String> {
    let mut steps = Vec::new();
    for step in spec.split('-') {
        let parts: Vec<_> = step.split('+').map(str::trim).collect();
        let Some(key) = parts.last().copied().filter(|key| !key.is_empty()) else {
            return Err(format!("empty key in `{spec}`"));
        };
        let mut bits = 0u8;
        for modifier in &parts[..parts.len() - 1] {
            bits |= match modifier.to_ascii_lowercase().as_str() {
                "shift" => 0b0000_0001,
                "ctrl" | "control" => 0b0000_0010,
                "alt" | "opt" | "option" => 0b0000_0100,
                "super" | "cmd" | "command" | "meta" | "win" => 0b0000_1000,
                other => return Err(format!("unknown modifier `{other}` in `{spec}`")),
            };
        }
        let lower = key.to_ascii_lowercase();
        let code = match lower.as_str() {
            "esc" | "escape" => quote! { ::ratatui::crossterm::event::KeyCode::Esc },
            "enter" | "return" => quote! { ::ratatui::crossterm::event::KeyCode::Enter },
            "tab" => quote! { ::ratatui::crossterm::event::KeyCode::Tab },
            "backtab" => quote! { ::ratatui::crossterm::event::KeyCode::BackTab },
            "backspace" | "bs" => quote! { ::ratatui::crossterm::event::KeyCode::Backspace },
            "left" => quote! { ::ratatui::crossterm::event::KeyCode::Left },
            "right" => quote! { ::ratatui::crossterm::event::KeyCode::Right },
            "up" => quote! { ::ratatui::crossterm::event::KeyCode::Up },
            "down" => quote! { ::ratatui::crossterm::event::KeyCode::Down },
            "home" => quote! { ::ratatui::crossterm::event::KeyCode::Home },
            "end" => quote! { ::ratatui::crossterm::event::KeyCode::End },
            "pageup" | "pgup" => quote! { ::ratatui::crossterm::event::KeyCode::PageUp },
            "pagedown" | "pgdn" => quote! { ::ratatui::crossterm::event::KeyCode::PageDown },
            "delete" | "del" => quote! { ::ratatui::crossterm::event::KeyCode::Delete },
            "insert" | "ins" => quote! { ::ratatui::crossterm::event::KeyCode::Insert },
            "space" => quote! { ::ratatui::crossterm::event::KeyCode::Char(' ') },
            "null" => quote! { ::ratatui::crossterm::event::KeyCode::Null },
            "minus" | "hyphen" => quote! { ::ratatui::crossterm::event::KeyCode::Char('-') },
            "plus" => quote! { ::ratatui::crossterm::event::KeyCode::Char('+') },
            function if function.starts_with('f') && function[1..].parse::<u8>().is_ok() => {
                let number = function[1..].parse::<u8>().expect("checked above");
                quote! { ::ratatui::crossterm::event::KeyCode::F(#number) }
            }
            _ if key.chars().count() == 1 => {
                let character = key
                    .chars()
                    .next()
                    .expect("checked length")
                    .to_ascii_lowercase();
                quote! { ::ratatui::crossterm::event::KeyCode::Char(#character) }
            }
            _ => return Err(format!("unrecognized key `{key}` in `{spec}`")),
        };
        let shifted = bits == 1 && (matches!(lower.as_str(), "tab") || key.chars().count() == 1);
        steps.push(quote! {
            ::reactatui::keys::ParsedKeySpec::__new(#bits, #code, #shifted)
        });
    }
    Ok(quote! {{
        const __REACTATUI_KEY_PATTERN: &[::reactatui::keys::ParsedKeySpec] = &[#(#steps),*];
        __REACTATUI_KEY_PATTERN
    }})
}

/// Builds a `reactatui::layout::Style` from CSS-like layout declarations.
#[proc_macro]
pub fn layout(input: TokenStream) -> TokenStream {
    layout::layout(input.into()).into()
}

/// Builds a `ratatui::style::Style` from CSS-like declarations.
///
/// Colors accept ratatui's named ANSI palette, `rgb(r, g, b)`, `#RGB`,
/// `#RRGGBB`, `indexed(n)`, or a braced Rust `Color` expression. Supported
/// properties are `color`, `background-color`, `text-decoration-color`,
/// `font-weight`, `font-style`, `text-decoration-line`, `visibility`,
/// `text-style`, `all`, and `patch`, along with the documented color aliases.
///
/// The macro supports the same inline and block `if` chains and top-level
/// `match` shorthand as `layout!`.
///
/// ```ignore
/// style! {
///     color: #f80;
///     background-color: rgb(20, 24, 31);
///     font-weight: bold;
///     text-style: dim underline not-reversed;
///
///     if focused {
///         color: yellow;
///     }
/// }
/// ```
#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    style::style(input.into()).into()
}

/// A react-esque functional component that tracks state automatically.
/// It injects a guard at the top of the function to push the component's unique
/// context to the hook runtime stack.
///
/// Unmarked parameters are positional constructor arguments. Only parameters
/// marked `#[prop]` can be supplied as named template attributes; attribute
/// order does not affect binding.
///
/// A `#[slot]` parameter receives children marked with the same name through a
/// `slot={...}` attribute. `#[slot(default)]` receives unmarked children.
/// `TuiNode` slots are required, while `Option<TuiNode>` slots may be omitted.
/// Slot parameters render through the usual dynamic-node syntax, `<{slot} />`.
#[proc_macro_attribute]
pub fn component(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(input as ItemFn);
    let visibility = func.vis.clone();

    let mut errors = Vec::new();
    let mut render_sig = func.sig.clone();
    render_sig.ident = syn::Ident::new("__reactatui_render", render_sig.ident.span());
    render_sig.inputs.clear();
    let mut render_props = Vec::new();
    let mut prop_markers = Vec::new();
    let mut call_args = Vec::new();
    let mut slot_extractors = Vec::new();
    let mut has_default_slot = false;
    let mut slot_lifetime = None;

    for input_arg in &mut func.sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input_arg {
            let Some(arg_name) = (match pat_type.pat.as_ref() {
                syn::Pat::Ident(pat) => Some(pat.ident.clone()),
                _ => None,
            }) else {
                errors.push(syn::Error::new_spanned(
                    &pat_type.pat,
                    "component arguments must be simple identifiers",
                ));
                continue;
            };
            let slot_attrs: Vec<_> = pat_type
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("slot"))
                .collect();
            let is_children = pat_type
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("children"));
            let is_prop = pat_type
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("prop"));
            let prop_count = pat_type
                .attrs
                .iter()
                .filter(|attr| attr.path().is_ident("prop"))
                .count();
            let has_named_marker = pat_type.attrs.iter().any(|attr| {
                attr.path().is_ident("prop")
                    || attr.path().is_ident("bind")
                    || attr.path().is_ident("slot")
            });

            if has_named_marker && is_reserved_component_prop(&arg_name) {
                errors.push(syn::Error::new_spanned(
                    &pat_type.pat,
                    format!(
                        "`{arg_name}` is reserved and cannot have a component parameter attribute"
                    ),
                ));
            }

            if slot_attrs.len() > 1 {
                errors.push(syn::Error::new_spanned(
                    &pat_type.pat,
                    "a component parameter can only have one `slot` attribute",
                ));
            }
            if prop_count > 1 {
                errors.push(syn::Error::new_spanned(
                    &pat_type.pat,
                    "a component parameter can only have one `prop` attribute",
                ));
            }
            if is_prop && (!slot_attrs.is_empty() || is_children) {
                errors.push(syn::Error::new_spanned(
                    &pat_type.pat,
                    "a component parameter cannot combine `prop` with `slot` or `children`",
                ));
            }
            if !slot_attrs.is_empty() && is_children {
                errors.push(syn::Error::new_spanned(
                    &pat_type.pat,
                    "a component parameter cannot be both `slot` and `children`",
                ));
            }

            if let Some(attr) = slot_attrs.first() {
                match parse_slot_parameter(attr, &pat_type.ty) {
                    Ok(slot) => {
                        if slot.is_default && has_default_slot {
                            errors.push(syn::Error::new_spanned(
                                attr,
                                "a component can only have one default slot",
                            ));
                        }
                        has_default_slot |= slot.is_default;
                        slot_extractors.push(slot.extractor(&arg_name));
                        slot_lifetime = slot_lifetime.or(slot.lifetime);
                    }
                    Err(error) => errors.push(error),
                }
            } else if is_children {
                match parse_wrapped_tui_node_type(&pat_type.ty, "Vec") {
                    Some(node_type) => {
                        slot_lifetime = slot_lifetime.or(node_type.lifetime);
                        slot_extractors.push(quote! {
                            let #arg_name = __reactatui_slots.take_all(None);
                        });
                    }
                    None => errors.push(syn::Error::new_spanned(
                        &pat_type.ty,
                        "children parameters must have type `Vec<TuiNode>`",
                    )),
                }
            } else {
                let mut render_arg = pat_type.clone();
                render_arg.attrs.retain(|attr| !is_parameter_marker(attr));
                if is_prop {
                    let marker = component_prop_marker(&arg_name);
                    prop_markers.push(quote! {
                        #[doc(hidden)]
                        #visibility fn #marker() {}
                    });
                    render_props.push((component_prop_name(&arg_name), render_arg));
                } else {
                    render_sig.inputs.push(syn::FnArg::Typed(render_arg));
                }
            }
            call_args.push(arg_name);

            pat_type.attrs.retain(|attr| !is_parameter_marker(attr));
        }
    }

    render_props.sort_by(|left, right| left.0.cmp(&right.0));
    render_sig.inputs.extend(
        render_props
            .into_iter()
            .map(|(_, argument)| syn::FnArg::Typed(argument)),
    );

    let slot_lifetime = slot_lifetime
        .map(|lifetime| quote! { #lifetime })
        .unwrap_or_else(|| quote! { '_ });
    render_sig.inputs.push(syn::parse_quote! {
        mut __reactatui_slots: ::reactatui::Slot<#slot_lifetime>
    });

    if !errors.is_empty() {
        let compile_errors = errors.into_iter().map(|error| error.to_compile_error());
        return quote! { #(#compile_errors)* #func }.into();
    }

    let component_name = func.sig.ident.clone();
    let cfg_attrs: Vec<_> = func
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
        .cloned()
        .collect();

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

    // A same-named type can coexist with the function in Rust's type namespace.
    // `tui!` calls this renderer to supply slots without changing direct function calls.
    quote! {
        #func

        #(#cfg_attrs)*
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #visibility struct #component_name {
            __reactatui_private: (),
        }

        #(#cfg_attrs)*
        impl #component_name {
            #(#prop_markers)*

            #[doc(hidden)]
            #visibility #render_sig {
                #(#slot_extractors)*
                #component_name(#(#call_args),*)
            }
        }
    }
    .into()
}

pub(crate) fn component_prop_marker(prop: &syn::Ident) -> syn::Ident {
    let name = component_prop_name(prop);
    format_ident!("__reactatui_prop_{name}", span = prop.span())
}

pub(crate) fn component_prop_name(prop: &syn::Ident) -> String {
    let name = prop.to_string();
    name.strip_prefix("r#").unwrap_or(&name).to_owned()
}

struct SlotParameter {
    is_default: bool,
    is_optional: bool,
    lifetime: Option<syn::Lifetime>,
}

struct TuiNodeType {
    lifetime: Option<syn::Lifetime>,
}

impl SlotParameter {
    fn extractor(&self, argument: &syn::Ident) -> proc_macro2::TokenStream {
        let name = (!self.is_default).then(|| argument.to_string());
        let selector = match &name {
            Some(name) => quote! { Some(#name) },
            None => quote! { None },
        };
        if self.is_optional {
            quote! {
                let #argument = __reactatui_slots.take(#selector);
            }
        } else {
            let missing = name.as_ref().map_or_else(
                || "required default slot was not provided".to_owned(),
                |name| format!("required slot `{name}` was not provided"),
            );
            quote! {
                let #argument = __reactatui_slots
                    .take(#selector)
                    .unwrap_or_else(|| panic!(#missing));
            }
        }
    }
}

fn parse_slot_parameter(attr: &Attribute, ty: &Type) -> syn::Result<SlotParameter> {
    let is_default = match &attr.meta {
        syn::Meta::Path(_) => false,
        syn::Meta::List(_) => {
            let value = attr.parse_args::<syn::Ident>()?;
            if value != "default" {
                return Err(syn::Error::new_spanned(
                    value,
                    "the only supported slot argument is `default`",
                ));
            }
            true
        }
        syn::Meta::NameValue(_) => {
            return Err(syn::Error::new_spanned(
                attr,
                "write `#[slot]` or `#[slot(default)]`",
            ));
        }
    };

    let (is_optional, node_type) = if let Some(node_type) = parse_tui_node_type(ty) {
        (false, node_type)
    } else if let Some(node_type) = parse_wrapped_tui_node_type(ty, "Option") {
        (true, node_type)
    } else {
        return Err(syn::Error::new_spanned(
            ty,
            "slot parameters must have type `TuiNode` or `Option<TuiNode>`",
        ));
    };

    Ok(SlotParameter {
        is_default,
        is_optional,
        lifetime: node_type.lifetime,
    })
}

fn parse_tui_node_type(ty: &Type) -> Option<TuiNodeType> {
    let segment = last_type_segment(ty)?;
    (segment.ident == "TuiNode").then(|| TuiNodeType {
        lifetime: first_lifetime(&segment.arguments),
    })
}

fn parse_wrapped_tui_node_type(ty: &Type, wrapper: &str) -> Option<TuiNodeType> {
    let segment = last_type_segment(ty)?;
    if segment.ident != wrapper {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let mut arguments = arguments.args.iter();
    let Some(GenericArgument::Type(inner)) = arguments.next() else {
        return None;
    };
    if arguments.next().is_some() {
        return None;
    }
    parse_tui_node_type(inner)
}

fn last_type_segment(ty: &Type) -> Option<&syn::PathSegment> {
    let Type::Path(path) = ty else { return None };
    path.path.segments.last()
}

fn first_lifetime(arguments: &PathArguments) -> Option<syn::Lifetime> {
    let PathArguments::AngleBracketed(arguments) = arguments else {
        return None;
    };
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Lifetime(lifetime) => Some(lifetime.clone()),
        _ => None,
    })
}

fn is_parameter_marker(attr: &Attribute) -> bool {
    ["children", "prop", "bind", "slot"]
        .iter()
        .any(|name| attr.path().is_ident(name))
}

fn is_reserved_component_prop(name: &syn::Ident) -> bool {
    ["children", "key", "layout", "slot", "style"]
        .iter()
        .any(|reserved| name == reserved)
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

/// Marks a component argument as a named slot, or as the default slot when
/// invoked as `#[slot(default)]`.
#[proc_macro_attribute]
pub fn slot(_metadata: TokenStream, input: TokenStream) -> TokenStream {
    input
}
