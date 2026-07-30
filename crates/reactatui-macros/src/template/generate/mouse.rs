use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::template::ast::Prop;

pub fn maybe_wrap_with_mouse(node: TokenStream2, props: &[Prop]) -> TokenStream2 {
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
            quote! { Some(Box::new({
                let mut __h = #h;
                move |btn: ratatui::crossterm::event::MouseButton| -> ::reactatui::hooks::Propagation {
                    __h(btn);
                    ::reactatui::hooks::Propagation::Stop
                }
            }) as Box<dyn FnMut(ratatui::crossterm::event::MouseButton) -> ::reactatui::hooks::Propagation>) }
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
        Some(h) => {
            quote! { Some(Box::new({
                let mut __h = #h;
                move |delta: i16| -> ::reactatui::hooks::Propagation {
                    __h(delta);
                    ::reactatui::hooks::Propagation::Stop
                }
            }) as Box<dyn FnMut(i16) -> ::reactatui::hooks::Propagation>) }
        }
        None => quote! { None },
    };
    let scrolly_tokens = match &scrolly {
        Some(h) => {
            quote! { Some(Box::new({
                let mut __h = #h;
                move |delta: i16| -> ::reactatui::hooks::Propagation {
                    __h(delta);
                    ::reactatui::hooks::Propagation::Stop
                }
            }) as Box<dyn FnMut(i16) -> ::reactatui::hooks::Propagation>) }
        }
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
