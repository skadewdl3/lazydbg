use crate::template::{
    ast::{Element, Node},
    generate::{
        component::{gen_component_is, gen_custom_component},
        flex::gen_flex,
        grid::gen_grid,
        misc::{gen_for_fragment, gen_if},
    },
};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

mod component;
mod flex;
mod grid;
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
    }
}

pub fn gen_fragment(children: &[Node]) -> TokenStream2 {
    let children = children.iter().map(gen_node);
    quote! {
        ::reactatui::TuiNode::fragment(vec![#(#children),*])
    }
}

fn gen_element(element: &Element) -> TokenStream2 {
    match element.tag.simple_name().as_deref() {
        Some("Component") => gen_component_is(element),
        Some("Flex") => gen_flex(element),
        Some("Grid") => gen_grid(element),
        _ => gen_custom_component(element),
    }
}
