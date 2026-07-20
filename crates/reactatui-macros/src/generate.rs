use proc_macro2::{Ident, TokenStream as TokenStream2};
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
        _ if is_builtin(&element.tag.type_name()) => gen_builtin(element),
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
            if let Some(flex) = named_prop(&element.props, "flex") {
                item = quote! { #item.flex(#flex) };
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
            _ => {}
        }
    }

    quote! { ::reactatui::TuiNode::from(#flex) }
}

fn gen_element_without_flex(element: &Element) -> TokenStream2 {
    let mut clone = element.clone();
    clone.props.retain(|prop| match prop {
        Prop::Named { name, .. } | Prop::Boolean(name) => {
            !matches!(name.to_string().as_str(), "flex" | "min" | "max")
        }
        Prop::Spread(_) | Prop::Event { .. } => true,
    });
    gen_element(&clone)
}

fn gen_builtin(element: &Element) -> TokenStream2 {
    if element.tag.type_name() == "List" && !element.children.is_empty() {
        return gen_list_with_children(element);
    }

    let widget = gen_widget_expr(element, false);
    if element.children.is_empty() {
        return gen_widget_node(widget, element);
    }

    if element.tag.type_name() == "Block" {
        let child = gen_branch(&element.children);
        // Apply mouse-region wrapper around the whole block+child composite if needed.
        maybe_wrap_with_mouse(quote! { #child.block(#widget) }, &element.props)
    } else {
        gen_widget_node(widget, element)
    }
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
            quote! { Some(Box::new(#h) as Box<dyn FnMut(::reactatui::ratatui::crossterm::event::MouseButton)>) }
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
        ::reactatui::TuiNode::Widget(Box::new(move |__area: ::reactatui::ratatui::layout::Rect, __buf: &mut ::reactatui::ratatui::buffer::Buffer| {
            ::reactatui::hooks::register_mouse_region(
                __area,
                #click_tokens,
                #mousein_tokens,
                #mouseout_tokens,
                #scrollx_tokens,
                #scrolly_tokens,
            );
            ::reactatui::ratatui::widgets::Widget::render(__inner_node, __area, __buf);
        }))
    }}
}

fn gen_widget_node(widget: TokenStream2, element: &Element) -> TokenStream2 {
    let node = if let Some(state) = named_prop(&element.props, "state") {
        quote! { ::reactatui::TuiNode::from_stateful_widget(#widget, #state) }
    } else {
        quote! { ::reactatui::TuiNode::from_widget(#widget) }
    };
    maybe_wrap_with_mouse(node, &element.props)
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
        // Legacy behaviour: look up known positional prop names and pull them from named props.
        let positional = positional_props(&ty_name, &constructor);
        let args = positional
            .iter()
            .filter_map(|name| named_prop(&element.props, name));
        if constructor == "default" {
            quote! { #ty::default() }
        } else {
            quote! { #ty::#ctor_ident(#(#args),*) }
        }
    };

    // Collect the set of positional prop names to skip them below (only relevant when
    // NOT using explicit constructor_args, but harmless to compute either way).
    let positional = if element.tag.constructor_args.is_none() {
        positional_props(&ty_name, &constructor)
    } else {
        Vec::new()
    };

    for prop in &element.props {
        match prop {
            Prop::Named { name, value } if name == "state" => {}
            Prop::Named { name, .. } if positional.iter().any(|pos| name == pos) => {}
            Prop::Named { name, .. }
                if omit_flex_props
                    && matches!(name.to_string().as_str(), "flex" | "min" | "max") => {}
            Prop::Named { name, value } => {
                widget = quote! { #widget.#name(#value) };
            }
            Prop::Boolean(name) if ty_name == "Block" && name == "borders" => {
                widget = quote! { #widget.borders(::reactatui::ratatui::widgets::Borders::ALL) };
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

fn gen_list_with_children(element: &Element) -> TokenStream2 {
    let child_items = gen_list_items(&element.children);
    let mut clone = element.clone();
    clone.children.clear();
    clone
        .props
        .retain(|prop| !matches!(prop, Prop::Named { name, .. } if name == "items"));
    clone.props.push(Prop::Named {
        name: Ident::new("items", proc_macro2::Span::call_site()),
        value: quote! { #child_items },
    });
    let widget = gen_widget_expr(&clone, false);
    gen_widget_node(widget, &clone)
}

fn gen_list_items(children: &[Node]) -> TokenStream2 {
    let pushes = children.iter().map(|child| match child {
        Node::Element(element) if element.tag.type_name() == "ListItem" => {
            let item = gen_widget_expr(element, false);
            quote! { __reactatui_items.push(#item); }
        }
        Node::For(node) => {
            let head = &node.head;
            let inner = gen_list_items(&node.body);
            quote! {
                for #head {
                    __reactatui_items.extend(#inner);
                }
            }
        }
        _ => quote! {
            compile_error!("List children must be <ListItem /> elements or for loops producing ListItem elements");
        },
    });

    quote! {{
        let mut __reactatui_items = Vec::new();
        #(#pushes)*
        __reactatui_items
    }}
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

    let call = if let Some(ctor_args) = &element.tag.constructor_args {
        // Positional args supplied explicitly via `Tag(arg1, arg2)` syntax.
        quote! { ::core::convert::Into::<::reactatui::TuiNode<'_>>::into(#tag(#ctor_args)) }
    } else {
        // Legacy: collect all named-prop values as positional arguments.
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

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "Block"
            | "Paragraph"
            | "List"
            | "ListItem"
            | "Tabs"
            | "Table"
            | "Gauge"
            | "Clear"
            | "Input"
    )
}

fn default_constructor(name: &str) -> &'static str {
    match name {
        "Block" | "Gauge" | "Clear" => "default",
        _ => "new",
    }
}

fn positional_props(type_name: &str, constructor: &str) -> Vec<&'static str> {
    match (type_name, constructor) {
        ("Paragraph", "new") => vec!["text"],
        ("Paragraph", "styled") => vec!["text", "style"],
        ("List", "new") => vec!["items"],
        ("ListItem", "new") => vec!["text"],
        ("Input", "new") => vec!["placeholder"],
        ("Tabs", "new") => vec!["titles"],
        ("Table", "new") => vec!["rows", "widths"],
        _ => Vec::new(),
    }
}

fn named_prop(props: &[Prop], expected: &str) -> Option<TokenStream2> {
    props.iter().find_map(|prop| match prop {
        Prop::Named { name, value } if name == expected => Some(value.clone()),
        _ => None,
    })
}
