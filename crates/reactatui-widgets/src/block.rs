use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::Line,
    widgets::{BorderType, Borders, Widget},
};
use reactatui::node::TuiNode;

/// A Block widget analogous to Ratatui's `Block`, capable of rendering child TUI nodes inside its borders.
pub struct Block<'a> {
    inner: ratatui::widgets::Block<'a>,
    children: Vec<TuiNode<'a>>,
}

impl<'a> Default for Block<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Block<'a> {
    pub fn new() -> Self {
        Self {
            inner: ratatui::widgets::Block::new(),
            children: Vec::new(),
        }
    }

    pub fn title<T>(mut self, title: T) -> Self
    where
        T: Into<Line<'a>>,
    {
        self.inner = self.inner.title(title);
        self
    }

    pub fn title_top<T>(mut self, title: T) -> Self
    where
        T: Into<Line<'a>>,
    {
        self.inner = self.inner.title_top(title);
        self
    }

    pub fn title_bottom<T>(mut self, title: T) -> Self
    where
        T: Into<Line<'a>>,
    {
        self.inner = self.inner.title_bottom(title);
        self
    }

    pub fn title_alignment(mut self, alignment: Alignment) -> Self {
        self.inner = self.inner.title_alignment(alignment);
        self
    }

    pub fn borders(mut self, borders: Borders) -> Self {
        self.inner = self.inner.borders(borders);
        self
    }

    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.inner = self.inner.style(style.into());
        self
    }

    pub fn border_style(mut self, style: impl Into<Style>) -> Self {
        self.inner = self.inner.border_style(style.into());
        self
    }

    pub fn border_type(mut self, border_type: impl Into<BorderType>) -> Self {
        self.inner = self.inner.border_type(border_type.into());
        self
    }

    pub fn padding(mut self, padding: ratatui::widgets::Padding) -> Self {
        self.inner = self.inner.padding(padding);
        self
    }

    pub fn children(mut self, children: impl Into<Vec<TuiNode<'a>>>) -> Self {
        self.children = children.into();
        self
    }

    /// Access the underlying `ratatui::widgets::Block`.
    pub fn inner_block(&self) -> &ratatui::widgets::Block<'a> {
        &self.inner
    }

    /// Convert into the underlying `ratatui::widgets::Block`.
    pub fn into_inner(self) -> ratatui::widgets::Block<'a> {
        self.inner
    }
}

impl<'a> From<Block<'a>> for ratatui::widgets::Block<'a> {
    fn from(block: Block<'a>) -> Self {
        block.inner
    }
}

impl<'a> Widget for Block<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner_area = self.inner.inner(area);
        self.inner.render(area, buf);
        if !self.children.is_empty() {
            TuiNode::fragment(self.children).render(inner_area, buf);
        }
    }
}
