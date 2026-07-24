use ratatui::{
    buffer::Buffer,
    layout::{Flex, Layout, Rect},
    style::Style,
    widgets::{BorderType, Borders, Clear, Padding, Widget},
};
use reactatui::node::TuiNode;

use crate::Block;

/// A dialog dimension: an exact cell count, or a percentage of the parent area.
/// `None` (the default) means "shrink to fit content".
#[derive(Debug, Clone, Copy)]
pub enum DialogDimension {
    Cells(u16),
    Percent(u16),
}

impl From<u16> for DialogDimension {
    fn from(v: u16) -> Self {
        DialogDimension::Cells(v)
    }
}
impl From<i32> for DialogDimension {
    fn from(v: i32) -> Self {
        DialogDimension::Cells(v.max(0) as u16)
    }
}
impl From<&str> for DialogDimension {
    fn from(s: &str) -> Self {
        if let Some(pct) = s.strip_suffix('%') {
            DialogDimension::Percent(pct.trim().parse().unwrap_or(0))
        } else {
            DialogDimension::Cells(s.trim().parse().unwrap_or(0))
        }
    }
}
impl From<String> for DialogDimension {
    fn from(s: String) -> Self {
        DialogDimension::from(s.as_str())
    }
}

pub struct Dialog<'a> {
    block: Block<'a>,
    children: Vec<TuiNode<'a>>,
    width: Option<DialogDimension>,
    height: Option<DialogDimension>,
    flex_ignore: bool,
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
            children: Vec::new(),
            width: None,  // auto-fit by default
            height: None, // auto-fit by default
            flex_ignore: false,
            clear_background: true,
            visible: true,
            background_style: None,
        }
    }

    pub fn width(mut self, width: impl Into<DialogDimension>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<DialogDimension>) -> Self {
        self.height = Some(height.into());
        self
    }

    pub fn flex_ignore(mut self, ignore: bool) -> Self {
        self.flex_ignore = ignore;
        self
    }

    pub fn children(mut self, children: impl Into<Vec<TuiNode<'a>>>) -> Self {
        self.children = children.into();
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    // title/borders/border_style/border_type/style/padding/clear_background/
    // background_style setters unchanged from before...

    /// Render children into a scratch buffer covering `area` and return the
    /// tight bounding box (width, height) of non-blank cells.
    fn measure_content(children: Vec<TuiNode<'a>>, area: Rect) -> (u16, u16, Buffer) {
        let mut scratch = Buffer::empty(area);
        if !children.is_empty() {
            TuiNode::fragment(children).render(area, &mut scratch);
        }

        let mut min_x = area.x + area.width;
        let mut min_y = area.y + area.height;
        let mut max_x = area.x;
        let mut max_y = area.y;

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if scratch[(x, y)].symbol() != " " {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }

        if max_x < min_x || max_y < min_y {
            // no content at all
            (0, 0, scratch)
        } else {
            (max_x - min_x + 1, max_y - min_y + 1, scratch)
        }
    }

    fn resolve_dim(
        dim: Option<DialogDimension>,
        content: u16,
        border: u16,
        parent_len: u16,
        flex_ignore: bool,
    ) -> u16 {
        match dim {
            None => (content + border).min(parent_len.max(border)),
            Some(DialogDimension::Percent(p)) => ((parent_len as u32 * p as u32) / 100) as u16,
            Some(DialogDimension::Cells(c)) => {
                if flex_ignore {
                    c
                } else {
                    c.min(parent_len)
                }
            }
        }
    }
}

impl<'a> Widget for Dialog<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }
        let borders = self.block.inner_block().inner(Rect::new(0, 0, 100, 100));
        let border_w = 100 - borders.width; // horizontal space consumed by borders/padding
        let border_h = 100 - borders.height; // vertical space consumed by borders/padding

        let (content_w, content_h, scratch) = Self::measure_content(self.children, area);

        let total_w = Self::resolve_dim(
            self.width,
            content_w,
            border_w,
            area.width,
            self.flex_ignore,
        );
        let total_h = Self::resolve_dim(
            self.height,
            content_h,
            border_h,
            area.height,
            self.flex_ignore,
        );

        let [horizontal] = Layout::horizontal([ratatui::layout::Constraint::Length(total_w)])
            .flex(Flex::Center)
            .areas(area);
        let [popup_area] = Layout::vertical([ratatui::layout::Constraint::Length(total_h)])
            .flex(Flex::Center)
            .areas(horizontal);

        if self.clear_background {
            Clear.render(popup_area, buf);
        }
        if let Some(style) = self.background_style {
            buf.set_style(popup_area, style);
        }

        let inner_area = self.block.inner_block().inner(popup_area);
        self.block.into_inner().render(popup_area, buf);

        // Blit the measured content from the scratch buffer into inner_area,
        // clipped to whichever is smaller.
        let copy_w = inner_area.width.min(content_w);
        let copy_h = inner_area.height.min(content_h);
        for y in 0..copy_h {
            for x in 0..copy_w {
                let src = &scratch[(area.x + x, area.y + y)];
                buf[(inner_area.x + x, inner_area.y + y)] = src.clone();
            }
        }
    }
}
