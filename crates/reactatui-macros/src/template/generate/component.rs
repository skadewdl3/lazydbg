use crate::template::{
    ast::{Element, Prop},
    generate::{
        builder::{has_bind_prop, named_prop, normal_component_args},
        gen_node,
        misc::gen_widget_expr,
        mouse::maybe_wrap_with_mouse,
    },
};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn gen_custom_component(element: &Element) -> TokenStream2 {
    let tag = element.tag.type_path_tokens();
    let component_name = element.tag.type_name();

    if has_bind_prop(&element.props) {
        return quote! { compile_error!("bind props require the #[component] props metadata design to be finalized") };
    }

    if !element.slots.is_empty() {
        return quote! { compile_error!("named slots require the #[component] props metadata design to be finalized") };
    }

    let event_hooks = component_event_hooks(element);
    let call = if element.tag.constructor.is_some() {
        gen_widget_component_call(element)
    } else {
        gen_function_component_call(element, tag)
    };

    let wrapped = maybe_wrap_with_mouse(call, &element.props);

    let with_events = if event_hooks.is_empty() {
        wrapped
    } else {
        quote! {{
            let __reactatui_child_id = ::reactatui::hooks::__next_component_id(#component_name);
            #(#event_hooks)*
            #wrapped
        }}
    };

    let with_layout = match named_prop(&element.props, "layout") {
        Some(layout_val) => quote! { ::reactatui::TuiNode::style(#with_events, #layout_val) },
        None => with_events,
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

fn component_event_hooks(element: &Element) -> Vec<TokenStream2> {
    element
        .props
        .iter()
        .filter_map(|prop| match prop {
            Prop::Event { kind, handler } if !is_mouse_event(kind) => {
                let event_name = kind.as_str();
                Some(quote! {
                    ::reactatui::hooks::use_on_component_id(__reactatui_child_id, #event_name, #handler);
                })
            }
            _ => None,
        })
        .collect()
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
    let children = (!element.children.is_empty()).then(|| gen_children_vec(element));

    if let Some(ctor_args) = &element.tag.constructor_args {
        return match children {
            Some(children) => {
                quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#ctor_args, #children)) }
            }
            None => {
                quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#ctor_args)) }
            }
        };
    }

    let args: Vec<_> =
        normal_component_args(&element.props, &["children", "style", "layout"]).collect();
    match (args.is_empty(), children) {
        (true, Some(children)) => {
            quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#children)) }
        }
        (false, Some(children)) => {
            quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#(#args),*, #children)) }
        }
        (true, None) => quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag()) },
        (false, None) => {
            quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#(#args),*)) }
        }
    }
}

fn gen_children_vec(element: &Element) -> TokenStream2 {
    let children = element.children.iter().map(gen_node);
    quote! { vec![#(#children),*] }
}

fn is_mouse_event(kind: &str) -> bool {
    matches!(
        kind,
        "click" | "mousein" | "mouseout" | "scrollx" | "scrolly"
    )
}
