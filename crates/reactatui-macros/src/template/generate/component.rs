use crate::template::{
    ast::{Element, Prop},
    generate::{
        builder::{component_prop_args, has_bind_prop, named_prop},
        gen_node,
        misc::gen_widget_expr,
        mouse::maybe_wrap_with_mouse,
    },
};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn gen_custom_component(element: &Element) -> TokenStream2 {
    let tag = element.tag.type_path_tokens();

    if has_bind_prop(&element.props) {
        return quote! { compile_error!("bind props require the #[component] props metadata design to be finalized") };
    }

    let call = if element.tag.constructor.is_some() {
        gen_widget_component_call(element)
    } else {
        gen_function_component_call(element, tag)
    };
    let call = if element.tag.constructor.is_none() {
        match named_prop(&element.props, "key") {
            Some(key) => quote! {{
                let __reactatui_key = #key;
                ::reactatui::hooks::__with_component_key(
                    &(file!(), line!(), column!(), &__reactatui_key),
                    || #call,
                )
            }},
            None => quote! {
                ::reactatui::hooks::__with_component_key(
                    &(file!(), line!(), column!()),
                    || #call,
                )
            },
        }
    } else {
        call
    };

    let wrapped = if element.tag.constructor.is_some() {
        maybe_wrap_with_mouse(call, &element.props)
    } else {
        call
    };

    let with_layout = match named_prop(&element.props, "layout") {
        Some(layout_val) => quote! { ::reactatui::TuiNode::style(#wrapped, #layout_val) },
        None => wrapped,
    };

    match named_prop(&element.props, "style") {
        Some(style_val) => quote! { ::reactatui::TuiNode::style(#with_layout, #style_val) },
        None => with_layout,
    }
}

/// Applies node-level behavior to an opaque `TuiNode` expression. Widget
/// builder props cannot be forwarded because the underlying widget is already
/// erased when it becomes a `TuiNode`.
pub fn gen_dynamic_component(element: &Element) -> TokenStream2 {
    if !element.children.is_empty() {
        return quote! { compile_error!("dynamic components cannot have children") };
    }

    if !element.slots.is_empty() {
        return quote! { compile_error!("dynamic components cannot have named slots") };
    }

    for prop in &element.props {
        match prop {
            Prop::Named { name, .. } if name == "layout" || name == "style" => {}
            Prop::Event { kind, .. }
                if matches!(
                    kind.as_str(),
                    "click" | "mousein" | "mouseout" | "scrollx" | "scrolly"
                ) => {}
            Prop::Named { name, .. } => {
                let message = format!(
                    "`{name}` cannot be forwarded to a dynamic component; dynamic components are opaque TuiNode values and only support `layout`, `style`, and mouse event props"
                );
                return quote! { compile_error!(#message) };
            }
            Prop::Boolean(name) => {
                let message = format!(
                    "`{name}` cannot be forwarded to a dynamic component; dynamic components are opaque TuiNode values and only support `layout`, `style`, and mouse event props"
                );
                return quote! { compile_error!(#message) };
            }
            Prop::Event { kind, .. } => {
                let message = format!(
                    "`on:{kind}` cannot be forwarded to a dynamic component; only mouse event props are supported"
                );
                return quote! { compile_error!(#message) };
            }
            Prop::Spread(_) => {
                return quote! { compile_error!("spread props cannot be forwarded to a dynamic component") };
            }
            Prop::Bind { .. } => {
                return quote! { compile_error!("bind props cannot be forwarded to a dynamic component") };
            }
        }
    }

    let node = element
        .tag
        .dynamic
        .as_ref()
        .expect("dynamic tag was checked");
    let node = quote! {{
        let __reactatui_dynamic = ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#node);
        ::reactatui::TuiNode::into_single_top_level(__reactatui_dynamic)
    }};
    let node = maybe_wrap_with_mouse(node, &element.props);
    let node = match named_prop(&element.props, "layout") {
        Some(layout) => quote! { ::reactatui::TuiNode::style(#node, #layout) },
        None => node,
    };

    match named_prop(&element.props, "style") {
        Some(style) => quote! { ::reactatui::TuiNode::style(#node, #style) },
        None => node,
    }
}

fn gen_widget_component_call(element: &Element) -> TokenStream2 {
    let widget = gen_widget_expr(element, false);
    let widget = if element.children.is_empty() {
        widget
    } else {
        let children = gen_children_vec(element);
        quote! { #widget.children(#children) }
    };

    match named_prop(&element.props, "state") {
        Some(state) => quote! { ::reactatui::TuiNode::from_stateful_widget(#widget, #state) },
        None => quote! { ::reactatui::TuiNode::from_widget(#widget) },
    }
}

fn gen_function_component_call(element: &Element, tag: TokenStream2) -> TokenStream2 {
    let slots = gen_children_vec(element);
    let positional = element.tag.constructor_args.as_ref();
    let positional = positional
        .filter(|args| !args.is_empty())
        .map(|args| quote! { #args, });
    let props = component_prop_args(
        &element.props,
        &["children", "key", "style", "layout", "slot"],
    );
    let prop_checks = props.iter().map(|(name, _)| {
        let marker = crate::component_prop_marker(name);
        quote! { #tag::#marker(); }
    });
    let prop_values = props.iter().map(|(_, value)| value);
    let prop_values = (!props.is_empty()).then(|| quote! { #(#prop_values),*, });

    quote! {{
        #(#prop_checks)*
        ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(
            #tag::__reactatui_render(
                #positional
                #prop_values
                ::reactatui::Slot::new(#slots)
            )
        )
    }}
}

fn gen_children_vec(element: &Element) -> TokenStream2 {
    let children = element.children.iter().map(gen_node);
    quote! { vec![#(#children),*] }
}
