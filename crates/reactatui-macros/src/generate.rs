use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::ast::{Element, ElseBranch, ForNode, IfNode, Node, Prop};

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

fn gen_if(node: &IfNode) -> TokenStream2 {
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

fn gen_branch(nodes: &[Node]) -> TokenStream2 {
    if nodes.len() == 1 {
        gen_node(&nodes[0])
    } else {
        gen_fragment(nodes)
    }
}

fn gen_for_fragment(node: &ForNode) -> TokenStream2 {
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

fn gen_element(element: &Element) -> TokenStream2 {
    match element.tag.simple_name().as_deref() {
        Some("Component") => gen_component_is(element),
        Some("Flex") => gen_flex(element),
        _ => gen_custom_component(element),
    }
}

fn gen_component_is(element: &Element) -> TokenStream2 {
    if let Some(value) = named_prop(&element.props, "is") {
        quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#value) }
    } else {
        quote! { compile_error!("<Component /> requires an `is` prop") }
    }
}

fn gen_flex(element: &Element) -> TokenStream2 {
    let items = element.children.iter().map(|child| match child {
        Node::Element(element) => {
            let node = gen_element_without_flex(element);
            let mut item = quote! { ::reactatui::FlexItemNode::new(#node) };
            if has_boolean_prop(&element.props, "flex_ignore") {
                item = quote! { #item.flex_ignore() };
            }
            if let Some(min) = named_prop(&element.props, "min") {
                item = quote! { #item.min(#min) };
            }
            if let Some(max) = named_prop(&element.props, "max") {
                item = quote! { #item.max(#max) };
            }
            item
        }
        child => {
            let node = gen_node(child);
            quote! { ::reactatui::FlexItemNode::new(#node) }
        }
    });

    let mut flex = quote! { ::reactatui::FlexNode::new(vec![#(#items),*]) };
    for prop in &element.props {
        match prop {
            Prop::Named { name, value } if name == "direction" => {
                flex = quote! { #flex.direction(#value) };
            }
            Prop::Named { name, value } if name == "gap" => {
                flex = quote! { #flex.gap(#value) };
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
            _ => {}
        }
    }

    quote! { ::reactatui::TuiNode::from(#flex) }
}

fn gen_element_without_flex(element: &Element) -> TokenStream2 {
    let mut clone = element.clone();
    clone.props.retain(|prop| match prop {
        Prop::Named { name, .. } | Prop::Boolean(name) => {
            !matches!(name.to_string().as_str(), "flex_ignore" | "min" | "max")
        }
        Prop::Spread(_) | Prop::Event { .. } => true,
    });
    gen_element(&clone)
}

/// Wrap a `TuiNode`-producing expression with mouse-region registration if
/// any mouse/scroll event props were present on the element.
fn maybe_wrap_with_mouse(node: TokenStream2, props: &[Prop]) -> TokenStream2 {
    let click = props.iter().find_map(|p| match p {
        Prop::Event { kind, handler } if kind == "click" => Some(handler.clone()),
        _ => None,
    });
    let mousein = props.iter().find_map(|p| match p {
        Prop::Event { kind, handler } if kind == "mousein" => Some(handler.clone()),
        _ => None,
    });
    let mouseout = props.iter().find_map(|p| match p {
        Prop::Event { kind, handler } if kind == "mouseout" => Some(handler.clone()),
        _ => None,
    });
    let scrollx = props.iter().find_map(|p| match p {
        Prop::Event { kind, handler } if kind == "scrollx" => Some(handler.clone()),
        _ => None,
    });
    let scrolly = props.iter().find_map(|p| match p {
        Prop::Event { kind, handler } if kind == "scrolly" => Some(handler.clone()),
        _ => None,
    });

    if click.is_none()
        && mousein.is_none()
        && mouseout.is_none()
        && scrollx.is_none()
        && scrolly.is_none()
    {
        return node;
    }

    let click_tokens = match &click {
        Some(h) => {
            quote! { Some(Box::new(#h) as Box<dyn FnMut(ratatui::crossterm::event::MouseButton)>) }
        }
        None => quote! { None },
    };
    let mousein_tokens = match &mousein {
        Some(h) => quote! { Some(Box::new(#h) as Box<dyn FnMut()>) },
        None => quote! { None },
    };
    let mouseout_tokens = match &mouseout {
        Some(h) => quote! { Some(Box::new(#h) as Box<dyn FnMut()>) },
        None => quote! { None },
    };
    let scrollx_tokens = match &scrollx {
        Some(h) => quote! { Some(Box::new(#h) as Box<dyn FnMut(i16)>) },
        None => quote! { None },
    };
    let scrolly_tokens = match &scrolly {
        Some(h) => quote! { Some(Box::new(#h) as Box<dyn FnMut(i16)>) },
        None => quote! { None },
    };

    quote! {{
        let __inner_node = #node;
        ::reactatui::TuiNode::Widget(Box::new(move |__area: ::ratatui::layout::Rect, __buf: &mut ::ratatui::buffer::Buffer| {
            ::reactatui::hooks::register_mouse_region(
                __area,
                #click_tokens,
                #mousein_tokens,
                #mouseout_tokens,
                #scrollx_tokens,
                #scrolly_tokens,
            );
            ::ratatui::widgets::Widget::render(__inner_node, __area, __buf);
        }))
    }}
}

fn gen_widget_expr(element: &Element, omit_flex_props: bool) -> TokenStream2 {
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
            Prop::Boolean(name) if name == "flex_ignore" => {}
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

fn gen_custom_component(element: &Element) -> TokenStream2 {
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
                    Prop::Named { name, value } if name != "children" && name != "flex_ignore" => {
                        Some(quote! { #value })
                    }
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

    if event_hooks.is_empty() {
        wrapped
    } else {
        quote! {{
            let __reactatui_child_id = ::reactatui::hooks::__next_component_id(#component_name);
            #(#event_hooks)*
            #wrapped
        }}
    }
}

fn default_constructor(_name: &str) -> &'static str {
    "default"
}

fn named_prop(props: &[Prop], expected: &str) -> Option<TokenStream2> {
    props.iter().find_map(|prop| match prop {
        Prop::Named { name, value } if name == expected => Some(value.clone()),
        _ => None,
    })
}

fn has_boolean_prop(props: &[Prop], expected: &str) -> bool {
    props
        .iter()
        .any(|prop| matches!(prop, Prop::Boolean(name) if name == expected))
}
