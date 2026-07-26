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

    pub fn style(self, style: impl Into<Style>) -> Self {
        Self::Styled(Box::new(self), style.into())
    }

    pub fn take_style(self) -> (Style, Self) {
        match self {
            Self::Styled(inner, style) => (style, *inner),
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
            TuiNode::Styled(inner, _style) => inner.render(area, buf),
            TuiNode::Empty => {}
        }
    }
}

pub(crate) fn render_fragment(children: Vec<TuiNode<'_>>, area: Rect, buf: &mut Buffer) {
    let count = children.len() as u16;
    if count == 0 {
        return;
    }

    let base = area.height / count;
    let mut remainder = area.height % count;
    let mut y = area.y;
    for child in children {
        let extra = u16::from(remainder > 0);
        remainder = remainder.saturating_sub(extra);
        let height = base.saturating_add(extra);
        child.render(Rect::new(area.x, y, area.width, height), buf);
        y = y.saturating_add(height);
    }
}
