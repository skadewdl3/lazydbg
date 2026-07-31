use crate::template::{
    ast::{Element, Node},
    generate::{
        component::{gen_custom_component, gen_dynamic_component},
        container::{ContainerKind, gen_container},
        misc::{gen_for_fragment, gen_if, gen_match},
    },
};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

mod builder;
mod component;
mod container;
mod misc;
mod mouse;

pub fn gen_node(node: &Node) -> TokenStream2 {
    match node {
        Node::Element(element) => gen_element(element),
        Node::Fragment(children) => gen_fragment(children),
        Node::Expr(expr) => {
            quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#expr) }
        }
        Node::If(node) => gen_if(node),
        Node::For(node) => gen_for_fragment(node),
        Node::Match(node) => gen_match(node),
    }
}

pub fn gen_fragment(children: &[Node]) -> TokenStream2 {
    let children = children.iter().map(gen_node);
    quote! {
        ::reactatui::TuiNode::fragment(vec![#(#children),*])
    }
}

fn gen_element(element: &Element) -> TokenStream2 {
    let node = if element.tag.dynamic.is_some() {
        gen_dynamic_component(element)
    } else {
        match element.tag.root_name().as_deref() {
            Some("Flex") => gen_container(element, ContainerKind::Flex),
            Some("Grid") => gen_container(element, ContainerKind::Grid),
            _ => gen_custom_component(element),
        }
    };

    match crate::template::generate::builder::named_prop(&element.props, "slot") {
        Some(slot) => quote! { ::reactatui::TuiNode::slot(#node, #slot) },
        None => node,
    }
}
