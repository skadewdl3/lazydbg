use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;

use crate::template::{
    ast::{Element, ElseBranch, IfNode, MatchNode, Node, Prop},
    generate::{gen_element, gen_node},
};

pub struct BuilderProps<'a> {
    /// Prop names handled outside the normal chained builder path.
    pub skip: &'a [&'a str],
    /// Whether `style={...}` should be emitted as `.style(...)`.
    pub include_style: bool,
    pub bind_error: &'static str,
}

/// Applies JSX-like props to a builder expression as chained method calls.
///
/// Named props become `.name(value)`, boolean props become `.name(true)`.
/// Events are ignored here because node-level generation wires them separately.
pub fn apply_builder_props(
    mut builder: TokenStream2,
    props: &[Prop],
    options: BuilderProps<'_>,
) -> TokenStream2 {
    for prop in props {
        match prop {
            Prop::Named { name, value } if name == "layout" && should_chain(name, &options) => {
                builder = quote! { #builder.style(#value) };
            }
            Prop::Named { name, value } if should_chain(name, &options) => {
                builder = quote! { #builder.#name(#value) };
            }
            Prop::Boolean(name) if should_chain(name, &options) => {
                builder = quote! { #builder.#name(true) };
            }
            Prop::Spread(value) => {
                let _ = value;
                return quote! { compile_error!("spread props are not supported by reactatui v0.3 yet") };
            }
            Prop::Bind { name, value } => {
                let _ = (name, value);
                let error = options.bind_error;
                return quote! { compile_error!(#error) };
            }
            Prop::Event { .. } | Prop::Named { .. } | Prop::Boolean(_) => {}
        }
    }
    builder
}

pub fn named_prop(props: &[Prop], expected: &str) -> Option<TokenStream2> {
    props.iter().find_map(|prop| match prop {
        Prop::Named { name, value } if name == expected => Some(value.clone()),
        _ => None,
    })
}

pub fn has_bind_prop(props: &[Prop]) -> bool {
    props.iter().any(|prop| matches!(prop, Prop::Bind { .. }))
}

/// Returns positional argument expressions for the current legacy component call path.
///
/// Once component prop metadata exists, this should be replaced by named prop struct
/// construction rather than relying on prop order.
pub fn normal_component_args<'a>(
    props: &'a [Prop],
    skip_names: &'a [&'a str],
) -> impl Iterator<Item = TokenStream2> + 'a {
    props.iter().filter_map(move |prop| match prop {
        Prop::Named { name, value } if !is_name_skipped(name, skip_names) => {
            Some(quote! { #value })
        }
        Prop::Boolean(name) if !is_name_skipped(name, skip_names) => Some(quote! { true }),
        _ => None,
    })
}

/// Emits pushes for layout-container children, preserving control flow.
pub fn gen_container_item_push<F>(
    node: &Node,
    items_ident: &Ident,
    item_type: TokenStream2,
    gen_item: &F,
) -> TokenStream2
where
    F: Fn(&Element) -> TokenStream2,
{
    match node {
        Node::Element(element) => {
            let item = gen_item(element);
            quote! { #items_ident.push(#item); }
        }
        Node::Fragment(children) => {
            let pushes = children.iter().map(|child| {
                gen_container_item_push(child, items_ident, item_type.clone(), gen_item)
            });
            quote! { #(#pushes)* }
        }
        Node::For(node) => {
            let head = &node.head;
            let pushes = node.body.iter().map(|child| {
                gen_container_item_push(child, items_ident, item_type.clone(), gen_item)
            });
            quote! {
                for #head {
                    #(#pushes)*
                }
            }
        }
        Node::If(node) => gen_container_if_push(node, items_ident, item_type, gen_item),
        Node::Match(node) => gen_container_match_push(node, items_ident, item_type, gen_item),
        child => {
            let node = gen_node(child);
            quote! { #items_ident.push(#item_type::new(#node)); }
        }
    }
}

/// Removes props that are consumed by the parent layout item before rendering a child.
pub fn gen_item_without_layout_props(element: &Element) -> TokenStream2 {
    let is_nested_container = matches!(
        element.tag.root_name().as_deref(),
        Some("Flex") | Some("Grid")
    );

    let mut clone = element.clone();
    clone.props.retain(|prop| match prop {
        Prop::Named { name, .. } | Prop::Boolean(name) => {
            let name = name.to_string();
            let strip_layout = (name == "style" || name == "layout") && !is_nested_container;
            !strip_layout
        }
        Prop::Spread(_) | Prop::Event { .. } | Prop::Bind { .. } => true,
    });
    gen_element(&clone)
}

fn gen_container_if_push<F>(
    node: &IfNode,
    items_ident: &Ident,
    item_type: TokenStream2,
    gen_item: &F,
) -> TokenStream2
where
    F: Fn(&Element) -> TokenStream2,
{
    let condition = &node.condition;
    let then_pushes = node
        .then_branch
        .iter()
        .map(|child| gen_container_item_push(child, items_ident, item_type.clone(), gen_item));
    let else_pushes = match &node.else_branch {
        Some(ElseBranch::If(node)) => {
            gen_container_if_push(node, items_ident, item_type.clone(), gen_item)
        }
        Some(ElseBranch::Nodes(nodes)) => {
            let pushes = nodes.iter().map(|child| {
                gen_container_item_push(child, items_ident, item_type.clone(), gen_item)
            });
            quote! { #(#pushes)* }
        }
        None => quote! {},
    };

    quote! {
        if #condition {
            #(#then_pushes)*
        } else {
            #else_pushes
        }
    }
}

fn gen_container_match_push<F>(
    node: &MatchNode,
    items_ident: &Ident,
    item_type: TokenStream2,
    gen_item: &F,
) -> TokenStream2
where
    F: Fn(&Element) -> TokenStream2,
{
    let scrutinee = &node.scrutinee;
    let arms = node.arms.iter().map(|arm| {
        let pattern = &arm.pattern;
        let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
        let pushes = arm
            .body
            .iter()
            .map(|child| gen_container_item_push(child, items_ident, item_type.clone(), gen_item));
        quote! {
            #pattern #guard => {
                #(#pushes)*
            }
        }
    });

    quote! {
        match #scrutinee {
            #(#arms),*
        }
    }
}

fn should_chain(name: &Ident, options: &BuilderProps<'_>) -> bool {
    (options.include_style || (name != "style" && name != "layout"))
        && !is_name_skipped(name, options.skip)
}

fn is_name_skipped(name: &Ident, skip_names: &[&str]) -> bool {
    skip_names.iter().any(|skip| name == skip)
}
