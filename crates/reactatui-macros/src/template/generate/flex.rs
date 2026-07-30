use crate::template::{
    ast::{Element, ElseBranch, IfNode, Node, Prop},
    generate::{gen_element, gen_node, misc::named_prop},
};

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
pub fn gen_flex(element: &Element) -> TokenStream2 {
    let items_ident = format_ident!("__reactatui_flex_items");
    let item_pushes = element
        .children
        .iter()
        .map(|child| gen_flex_item_push(child, &items_ident));

    let mut flex = quote! { ::reactatui::FlexNode::new(#items_ident) };
    for prop in &element.props {
        match prop {
            Prop::Named { name, value } if name == "direction" => {
                flex = quote! { #flex.direction(#value) };
            }
            Prop::Named { name, value } if name == "padding" => {
                flex = quote! { #flex.padding(#value) };
            }
            Prop::Spread(value) => {
                let _ = value;
                flex = quote! { compile_error!("spread props are not supported by reactatui v0.3 yet") };
            }
            Prop::Named { name, value } if name == "layout" => {
                flex = quote! { #flex.layout(#value) };
            }
            Prop::Named { name, value } if name == "style" => {
                flex = quote! { #flex.style(#value) };
            }
            _ => {}
        }
    }

    quote! {{
        let mut #items_ident = Vec::new();
        #(#item_pushes)*
        ::reactatui::TuiNode::from(#flex)
    }}
}

pub fn gen_flex_item(element: &Element) -> TokenStream2 {
    let node = gen_element_without_flex(element);
    let mut item = quote! { ::reactatui::FlexItemNode::new(#node) };
    if let Some(style) = named_prop(&element.props, "style") {
        item = quote! {
            #item.style(::core::convert::Into::<::reactatui::layout::Style>::into(#style))
        };
    }
    item
}

pub fn gen_flex_item_push(node: &Node, items_ident: &proc_macro2::Ident) -> TokenStream2 {
    match node {
        Node::Element(element) => {
            let item = gen_flex_item(element);
            quote! { #items_ident.push(#item); }
        }
        Node::Fragment(children) => {
            let pushes = children
                .iter()
                .map(|child| gen_flex_item_push(child, items_ident));
            quote! { #(#pushes)* }
        }
        Node::For(node) => {
            let head = &node.head;
            let pushes = node
                .body
                .iter()
                .map(|child| gen_flex_item_push(child, items_ident));
            quote! {
                for #head {
                    #(#pushes)*
                }
            }
        }
        Node::If(node) => gen_flex_if_push(node, items_ident),
        child => {
            let node = gen_node(child);
            quote! { #items_ident.push(::reactatui::FlexItemNode::new(#node)); }
        }
    }
}

pub fn gen_flex_if_push(node: &IfNode, items_ident: &proc_macro2::Ident) -> TokenStream2 {
    let condition = &node.condition;
    let then_pushes = node
        .then_branch
        .iter()
        .map(|child| gen_flex_item_push(child, items_ident));
    let else_pushes = match &node.else_branch {
        Some(ElseBranch::If(node)) => gen_flex_if_push(node, items_ident),
        Some(ElseBranch::Nodes(nodes)) => {
            let pushes = nodes
                .iter()
                .map(|child| gen_flex_item_push(child, items_ident));
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

pub fn gen_element_without_flex(element: &Element) -> TokenStream2 {
    let is_nested_container = matches!(
        element.tag.simple_name().as_deref(),
        Some("Flex") | Some("Grid")
    );

    let mut clone = element.clone();
    clone.props.retain(|prop| match prop {
        Prop::Named { name, .. } | Prop::Boolean(name) => {
            let name = name.to_string();
            let strip_item_only = matches!(name.as_str(), "min" | "max");
            let strip_style = name == "style" && !is_nested_container;
            !(strip_item_only || strip_style)
        }
        Prop::Spread(_) | Prop::Event { .. } => true,
    });
    gen_element(&clone)
}
