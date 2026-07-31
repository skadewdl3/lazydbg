use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::template::{
    ast::Element,
    generate::builder::{
        BuilderProps, apply_builder_props, gen_container_item_push, gen_item_without_layout_props,
        named_prop,
    },
};

#[derive(Clone, Copy)]
pub enum ContainerKind {
    Flex,
    Grid,
}

impl ContainerKind {
    fn item_type(self) -> TokenStream2 {
        match self {
            Self::Flex => quote! { ::reactatui::FlexItemNode },
            Self::Grid => quote! { ::reactatui::GridItemNode },
        }
    }

    fn items_ident(self) -> proc_macro2::Ident {
        match self {
            Self::Flex => format_ident!("__reactatui_flex_items"),
            Self::Grid => format_ident!("__reactatui_grid_items"),
        }
    }
}

/// Generates Flex and Grid from the same component-like path. Their tag's
/// constructor and all normal builder props are preserved; only children are
/// wrapped in the container-specific item type required by the runtime.
pub fn gen_container(element: &Element, kind: ContainerKind) -> TokenStream2 {
    let items_ident = kind.items_ident();
    let item_type = kind.item_type();
    let item_pushes = element.children.iter().map(|child| {
        gen_container_item_push(child, &items_ident, item_type.clone(), &|element| {
            gen_container_item(element, item_type.clone())
        })
    });

    let ty = element.tag.type_path_tokens();
    let constructor = element
        .tag
        .constructor
        .as_ref()
        .map(|constructor| quote! { #constructor })
        .unwrap_or_else(|| quote! { new });
    let constructor_args = element
        .tag
        .constructor_args
        .as_ref()
        .filter(|args| !args.is_empty())
        .map(|args| quote! { #args, });
    let builder = apply_builder_props(
        quote! { #ty::#constructor(#constructor_args #items_ident) },
        &element.props,
        BuilderProps {
            skip: &[],
            include_style: true,
            bind_error: "bind props require #[component] metadata support",
        },
    );

    quote! {{
        let mut #items_ident = Vec::new();
        #(#item_pushes)*
        ::reactatui::TuiNode::from(#builder)
    }}
}

fn gen_container_item(element: &Element, item_type: TokenStream2) -> TokenStream2 {
    let node = gen_item_without_layout_props(element);
    let mut item = quote! { #item_type::new(#node) };
    if let Some(style) = named_prop(&element.props, "layout") {
        item = quote! {
            #item.style(::core::convert::Into::<::reactatui::layout::Style>::into(#style))
        };
    }
    item
}
