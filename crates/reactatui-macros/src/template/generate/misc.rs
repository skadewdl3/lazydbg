use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::template::{
    ast::{Element, ElseBranch, ForNode, IfNode, MatchNode, Node},
    generate::{
        builder::{BuilderProps, apply_builder_props},
        gen_fragment, gen_node,
    },
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

pub fn gen_match(node: &MatchNode) -> TokenStream2 {
    let scrutinee = &node.scrutinee;
    let arms = node.arms.iter().map(|arm| {
        let pattern = &arm.pattern;
        let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
        let body = gen_branch(&arm.body);
        quote! { #pattern #guard => #body, }
    });
    quote! {
        match #scrutinee {
            #(#arms)*
        }
    }
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

    let widget = if let Some(ctor_args) = &element.tag.constructor_args {
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

    let skip = if omit_flex_props {
        &["state", "layout", "slot", "flex", "min", "max"][..]
    } else {
        &["state", "layout", "slot"][..]
    };

    apply_builder_props(
        widget,
        &element.props,
        BuilderProps {
            skip,
            include_style: true,
            bind_error: "bind props require #[component] metadata support",
        },
    )
}

pub fn default_constructor(_name: &str) -> &'static str {
    "default"
}
