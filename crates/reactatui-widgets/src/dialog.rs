use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{BorderType, Borders, Clear, Padding, Widget},
};
use reactatui::node::TuiNode;

use crate::Block; // your custom Block widget from above

/// Controls how a `Dialog`'s popup area is sized relative to the area it's rendered into.
#[derive(Debug, Clone, Copy)]
pub enum DialogSize {
    /// Percentage of the parent area, e.g. `Percent(60, 40)` = 60% width, 40% height.
    Percent(u16, u16),
    /// Fixed size in cells, clamped to the parent area if it doesn't fit.
    Fixed(u16, u16),
    /// Fixed size in cells, but grows to fill the parent area if the parent is smaller.
    FixedOrFill(u16, u16),
}

impl Default for DialogSize {
    fn default() -> Self {
        DialogSize::Percent(60, 40)
    }
}

/// A centered popup/dialog widget, analogous to a modal `Block`, capable of
/// rendering child TUI nodes inside its borders.
///
/// Internally this composes your `Block` widget, so all border/title/padding
/// behavior is identical — `Dialog` just adds centering, sizing, and an
/// optional background clear/dim so it renders over whatever is already there.
pub struct Dialog<'a> {
    block: Block<'a>,
    size: DialogSize,
    clear_background: bool,
    background_style: Option<Style>,
    visible: bool,
}

impl<'a> Default for Dialog<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Dialog<'a> {
    pub fn new() -> Self {
        Self {
            block: Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .padding(Padding::uniform(1)),
            size: DialogSize::default(),
            clear_background: true,
            background_style: None,
            visible: true,
        }
    }

    // --- sizing / positioning -------------------------------------------------

    pub fn size(mut self, size: DialogSize) -> Self {
        self.size = size;
        self
    }

    pub fn percent(mut self, width: u16, height: u16) -> Self {
        self.size = DialogSize::Percent(width, height);
        self
    }

    pub fn fixed(mut self, width: u16, height: u16) -> Self {
        self.size = DialogSize::Fixed(width, height);
        self
    }

    /// Whether to paint over (clear) whatever was previously in the popup area
    /// before rendering the dialog's block. Defaults to `true`. Disable this
    /// if you're rendering onto a buffer you know is already blank there.
    pub fn clear_background(mut self, clear: bool) -> Self {
        self.clear_background = clear;
        self
    }

    /// Optional style applied to the popup area before the block/border is
    /// drawn (e.g. a dimmed/tinted backdrop for the dialog body).
    pub fn background_style(mut self, style: Style) -> Self {
        self.background_style = Some(style);
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    // --- delegated Block builder methods --------------------------------------

    pub fn title<T>(mut self, title: T) -> Self
    where
        T: Into<Line<'a>>,
    {
        self.block = self.block.title(title);
        self
    }

    pub fn title_top<T>(mut self, title: T) -> Self
    where
        T: Into<Line<'a>>,
    {
        self.block = self.block.title_top(title);
        self
    }

    pub fn title_bottom<T>(mut self, title: T) -> Self
    where
        T: Into<Line<'a>>,
    {
        self.block = self.block.title_bottom(title);
        self
    }

    pub fn title_alignment(mut self, alignment: Alignment) -> Self {
        self.block = self.block.title_alignment(alignment);
        self
    }

    pub fn borders(mut self, borders: Borders) -> Self {
        self.block = self.block.borders(borders);
        self
    }

    pub fn border_style(mut self, style: Style) -> Self {
        self.block = self.block.border_style(style);
        self
    }

    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.block = self.block.border_type(border_type);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.block = self.block.style(style);
        self
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.block = self.block.padding(padding);
        self
    }

    pub fn children(mut self, children: impl Into<Vec<TuiNode<'a>>>) -> Self {
        self.block = self.block.children(children);
        self
    }

    /// Access the underlying custom `Block`.
    pub fn inner_block(&self) -> &Block<'a> {
        &self.block
    }

    /// Compute the centered popup rect for a given parent `area`, according
    /// to this dialog's `DialogSize`.
    fn popup_area(&self, area: Rect) -> Rect {
        let (width_constraint, height_constraint) = match self.size {
            DialogSize::Percent(w, h) => (Constraint::Percentage(w), Constraint::Percentage(h)),
            DialogSize::Fixed(w, h) => (
                Constraint::Length(w.min(area.width)),
                Constraint::Length(h.min(area.height)),
            ),
            DialogSize::FixedOrFill(w, h) => (
                Constraint::Length(w.min(area.width)),
                Constraint::Length(h.min(area.height)),
            ),
        };

        let [horizontal] = Layout::horizontal([width_constraint])
            .flex(Flex::Center)
            .areas(area);
        let [vertical] = Layout::vertical([height_constraint])
            .flex(Flex::Center)
            .areas(horizontal);
        vertical
    }
}

impl<'a> Widget for Dialog<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }
        let popup_area = self.popup_area(area);

        if self.clear_background {
            Clear.render(popup_area, buf);
        }

        if let Some(style) = self.background_style {
            buf.set_style(popup_area, style);
        }

        self.block.render(popup_area, buf);
    }
}
