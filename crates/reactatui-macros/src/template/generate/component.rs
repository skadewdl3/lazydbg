use crate::template::{
    ast::{Element, Prop},
    generate::{
        gen_node,
        misc::{gen_widget_expr, named_prop},
        mouse::maybe_wrap_with_mouse,
    },
};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub fn gen_custom_component(element: &Element) -> TokenStream2 {
    let tag = element.tag.type_path_tokens();
    let component_name = element.tag.type_name();

    // Collect `on:event_name={handler}` props — these become `use_on` calls
    // scoped to the child component about to be invoked, so sibling
    // component handlers don't all receive the same bubbled event.
    let event_hooks: Vec<TokenStream2> = element
        .props
        .iter()
        .filter_map(|prop| match prop {
            Prop::Event { kind, handler }
                // Mouse/pointer events are handled separately via register_mouse_region.
                if !matches!(kind.as_str(), "click" | "mousein" | "mouseout" | "scrollx" | "scrolly") =>
            {
                let event_name = kind.as_str();
                Some(quote! {
                    ::reactatui::hooks::use_on_component_id(__reactatui_child_id, #event_name, #handler);
                })
            }
            _ => None,
        })
        .collect();

    let has_children = !element.children.is_empty();

    let call = if has_children && element.tag.constructor.is_some() {
        let widget = gen_widget_expr(element, false);
        let child_nodes = element.children.iter().map(gen_node);
        let children_vec = quote! { vec![#(#child_nodes),*] };
        let widget = quote! { #widget.children(#children_vec) };
        if let Some(state) = named_prop(&element.props, "state") {
            quote! { ::reactatui::TuiNode::from_stateful_widget(#widget, #state) }
        } else {
            quote! { ::reactatui::TuiNode::from_widget(#widget) }
        }
    } else if has_children {
        let child_nodes = element.children.iter().map(gen_node);
        let children_vec = quote! { vec![#(#child_nodes),*] };

        if let Some(ctor_args) = &element.tag.constructor_args {
            quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#ctor_args, #children_vec)) }
        } else {
            let named_args: Vec<_> = element
                .props
                .iter()
                .filter_map(|prop| match prop {
                    Prop::Named { name, value } if name != "children" => Some(quote! { #value }),
                    Prop::Boolean(_) | Prop::Spread(_) | Prop::Event { .. } => None,
                    _ => None,
                })
                .collect();
            if named_args.is_empty() {
                quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#children_vec)) }
            } else {
                quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#(#named_args),*, #children_vec)) }
            }
        }
    } else if element.tag.constructor.is_some() {
        let widget = gen_widget_expr(element, false);
        if let Some(state) = named_prop(&element.props, "state") {
            quote! { ::reactatui::TuiNode::from_stateful_widget(#widget, #state) }
        } else {
            quote! { ::reactatui::TuiNode::from_widget(#widget) }
        }
    } else if let Some(ctor_args) = &element.tag.constructor_args {
        quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#ctor_args)) }
    } else {
        let args = element.props.iter().filter_map(|prop| match prop {
            Prop::Named { value, .. } => Some(quote! { #value }),
            Prop::Boolean(_) | Prop::Spread(_) | Prop::Event { .. } => None,
        });
        quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#(#args),*)) }
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

    match named_prop(&element.props, "style") {
        Some(style_val) => quote! { ::reactatui::TuiNode::style(#with_events, #style_val) },
        None => with_events,
    }
}

pub fn gen_component_is(element: &Element) -> TokenStream2 {
    if let Some(value) = named_prop(&element.props, "is") {
        quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#value) }
    } else {
        quote! { compile_error!("<Component /> requires an `is` prop") }
    }
}
