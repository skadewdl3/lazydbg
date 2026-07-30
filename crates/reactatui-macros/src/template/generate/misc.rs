use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::template::{
    ast::{Element, ElseBranch, ForNode, IfNode, Node, Prop},
    generate::{gen_fragment, gen_node},
};

pub fn gen_if(node: &IfNode) -> TokenStream2 {
    let condition = &node.condition;
    let then_branch = gen_branch(&node.then_branch);
    let else_branch = match &node.else_branch {
        Some(ElseBranch::If(node)) => gen_if(node),
        Some(ElseBranch::Nodes(nodes)) => gen_branch(nodes),
        None => quote! { ::reactatui::TuiNode::empty() },
    };

    quote! {
        if #condition {
            #then_branch
        } else {
            #else_branch
        }
    }
}

pub fn gen_branch(nodes: &[Node]) -> TokenStream2 {
    if nodes.len() == 1 {
        gen_node(&nodes[0])
    } else {
        gen_fragment(nodes)
    }
}

pub fn gen_for_fragment(node: &ForNode) -> TokenStream2 {
    let head = &node.head;
    let body = gen_branch(&node.body);
    quote! {{
        let mut __reactatui_children = Vec::new();
        for #head {
            __reactatui_children.push(#body);
        }
        ::reactatui::TuiNode::fragment(__reactatui_children)
    }}
}

pub fn gen_widget_expr(element: &Element, omit_flex_props: bool) -> TokenStream2 {
    let ty = element.tag.type_path_tokens();
    let ty_name = element.tag.type_name();
    let constructor = element
        .tag
        .constructor
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| default_constructor(&ty_name).to_string());

    let ctor_ident = format_ident!("{constructor}");

    let mut widget = if let Some(ctor_args) = &element.tag.constructor_args {
        // Explicit positional args provided via `(arg1, arg2)` syntax.
        if constructor == "default" {
            quote! { #ty::default() }
        } else {
            quote! { #ty::#ctor_ident(#ctor_args) }
        }
    } else {
        if constructor == "default" {
            quote! { #ty::default() }
        } else {
            quote! { #ty::#ctor_ident() }
        }
    };

    for prop in &element.props {
        match prop {
            Prop::Named { name, .. } if name == "state" => {}
            Prop::Named { name, .. }
                if omit_flex_props
                    && matches!(name.to_string().as_str(), "flex" | "min" | "max") => {}
            Prop::Named { name, value } => {
                widget = quote! { #widget.#name(#value) };
            }
            Prop::Boolean(name) => {
                widget = quote! { #widget.#name(true) };
            }
            Prop::Spread(_) => {
                widget = quote! { compile_error!("spread props are not supported by reactatui v0.3 yet") };
            }
            // Event props are handled at the node level, not passed to the widget.
            Prop::Event { .. } => {}
        }
    }

    widget
}

pub fn default_constructor(_name: &str) -> &'static str {
    "default"
}

pub fn named_prop(props: &[Prop], expected: &str) -> Option<TokenStream2> {
    props.iter().find_map(|prop| match prop {
        Prop::Named { name, value } if name == expected => Some(value.clone()),
        _ => None,
    })
}
