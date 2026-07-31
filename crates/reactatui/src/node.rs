use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, StatefulWidget, Widget},
};

use crate::layout::{FlexNode, GridNode};
use crate::{hooks, layout::Style};

/// Universal render node produced by the `tui!` macro.
pub enum TuiNode<'a> {
    Widget(Box<dyn FnOnce(Rect, &mut Buffer) + 'a>),
    Fragment(Vec<TuiNode<'a>>),
    Flex(FlexNode<'a>),
    Grid(GridNode<'a>),
    Styled(Box<TuiNode<'a>>, Style),
    Slotted(Box<TuiNode<'a>>, String),
    Empty,
}

/// Something that can hand out a `&mut S` for exactly one render call.
pub trait StateHandle<'a, S> {
    fn with_state<R>(self, f: impl FnOnce(&mut S) -> R) -> R;
}

impl<'a, S> StateHandle<'a, S> for &'a mut S {
    fn with_state<R>(self, f: impl FnOnce(&mut S) -> R) -> R {
        f(self)
    }
}

impl<'a, S: 'static> StateHandle<'a, S> for hooks::State<S> {
    fn with_state<R>(self, f: impl FnOnce(&mut S) -> R) -> R {
        self.with_mut(f)
    }
}

impl<'a> TuiNode<'a> {
    pub fn from_widget<W>(widget: W) -> Self
    where
        W: Widget + 'a,
    {
        Self::Widget(Box::new(move |area, buf| widget.render(area, buf)))
    }

    pub fn from_stateful_widget<W, S, H>(widget: W, state: H) -> Self
    where
        W: StatefulWidget<State = S> + 'a,
        H: StateHandle<'a, S> + 'a,
    {
        Self::Widget(Box::new(move |area, buf| {
            state.with_state(|state| widget.render(area, buf, state));
        }))
    }

    pub fn fragment(children: impl Into<Vec<TuiNode<'a>>>) -> Self {
        let children = children.into();
        if children.is_empty() {
            Self::Empty
        } else {
            Self::Fragment(children)
        }
    }

    pub fn empty() -> Self {
        Self::Empty
    }

    /// Unwraps the sole root of an opaque node tree before applying props to it.
    ///
    /// `tui!` cannot determine the shape of an arbitrary `TuiNode` expression
    /// at compile time, so dynamic component tags use this runtime guard.
    pub fn into_single_top_level(self) -> Self {
        match self {
            Self::Fragment(mut children) if children.len() == 1 => {
                children.pop().expect("fragment length was checked")
            }
            Self::Fragment(children) => panic!(
                "dynamic component requires exactly one top-level component, but received {}",
                children.len()
            ),
            Self::Empty => Self::Empty,
            node => node,
        }
    }

    pub fn style(self, style: impl Into<Style>) -> Self {
        Self::Styled(Box::new(self), style.into())
    }

    #[doc(hidden)]
    pub fn slot(self, name: impl Into<String>) -> Self {
        Self::Slotted(Box::new(self), name.into())
    }

    pub(crate) fn opaque_fragment(children: Vec<TuiNode<'a>>) -> Self {
        match children.len() {
            0 => Self::Empty,
            1 => children.into_iter().next().expect("length was checked"),
            _ => Self::Widget(Box::new(move |area, buf| {
                render_fragment(children, area, buf)
            })),
        }
    }

    pub fn take_style(self) -> (Style, Self) {
        match self {
            Self::Styled(inner, style) => {
                let (inner_style, inner_node) = inner.take_style();
                (style.merge(&inner_style), inner_node)
            }
            other => (Style::default(), other),
        }
    }

    pub fn block(self, block: Block<'a>) -> Self {
        Self::Widget(Box::new(move |area, buf| {
            let inner = block.inner(area);
            block.render(area, buf);
            self.render(inner, buf);
        }))
    }
}

impl<'a> From<FlexNode<'a>> for TuiNode<'a> {
    fn from(value: FlexNode<'a>) -> Self {
        Self::Flex(value)
    }
}

impl<'a> From<GridNode<'a>> for TuiNode<'a> {
    fn from(value: GridNode<'a>) -> Self {
        Self::Grid(value)
    }
}

impl<'a> Widget for TuiNode<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            TuiNode::Widget(render) => render(area, buf),
            TuiNode::Fragment(children) => render_fragment(children, area, buf),
            TuiNode::Flex(flex) => flex.render(area, buf),
            TuiNode::Grid(grid) => grid.render(area, buf),
            TuiNode::Styled(inner, style) => match *inner {
                TuiNode::Flex(flex) => flex.style(style).render(area, buf),
                TuiNode::Grid(grid) => grid.style(style).render(area, buf),
                TuiNode::Styled(child, inner_style) => {
                    TuiNode::Styled(child, style.merge(&inner_style)).render(area, buf)
                }
                other => other.render(area, buf),
            },
            TuiNode::Slotted(node, _) => node.render(area, buf),
            TuiNode::Empty => {}
        }
    }
}

impl<'a> From<Option<TuiNode<'a>>> for TuiNode<'a> {
    fn from(node: Option<TuiNode<'a>>) -> Self {
        node.unwrap_or(TuiNode::Empty)
    }
}

/// Render each fragment child into the full area (overlay semantics).
///
/// Fragments are produced by `if`/`for` control flow inside containers —
/// their children are already individually sized by the parent container
/// (Flex/Grid) that drives the layout. Splitting the area equally among
/// fragment children has no meaningful semantic and produces broken output;
/// rendering each into the full allocated area is correct.
pub(crate) fn render_fragment(children: Vec<TuiNode<'_>>, area: Rect, buf: &mut Buffer) {
    for child in children {
        child.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::style::Align;
    use crate::layout::{Size, Style};

    #[test]
    fn test_additive_nested_styles() {
        let inner = TuiNode::Empty.style(Style::default().align_self(Align::Center));
        let outer = inner.style(Style::default().size(Size::Length(1)));

        let (merged, _core) = outer.take_style();
        assert_eq!(merged.size, Size::Length(1));
        assert_eq!(merged.align_self, Some(Align::Center));
    }
}
