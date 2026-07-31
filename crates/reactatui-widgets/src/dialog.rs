use ratatui::{
    buffer::Buffer,
    layout::{Flex, Layout, Rect},
    style::Style,
    widgets::{Clear, Widget},
};
use reactatui::{
    TuiNode, component,
    hooks::{State, bind},
    keybindings, lambda,
    measure::{Measured, blit_measured, measure_node},
    style, tui,
};

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

struct DialogWidget<'a> {
    block: Block<'a>,
    children: Vec<TuiNode<'a>>,
    width: Option<DialogDimension>,
    height: Option<DialogDimension>,
    clear_background: bool,
    background_style: Option<Style>,
    visible: bool,
}

impl<'a> Default for DialogWidget<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DialogWidget<'a> {
    pub fn new() -> Self {
        let style = style! {
            borders: all;
            border-type: rounded;
            padding: 1;
        };
        Self {
            block: Block::new()
                .borders(&style)
                .border_type(&style)
                .padding(&style),
            children: Vec::new(),
            width: None,  // auto-fit by default
            height: None, // auto-fit by default
            clear_background: true,
            visible: true,
            background_style: None,
        }
    }

    pub fn width(mut self, width: impl Into<DialogDimension>) -> Self {
        self.width = Some(width.into());
        self
    }

    #[allow(dead_code)]
    pub fn height(mut self, height: impl Into<DialogDimension>) -> Self {
        self.height = Some(height.into());
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
    /// tight bounding box (width, height) of non-blank cells, via the shared
    /// `reactatui::measure` primitive (also used by `Flex`).
    fn measure_content(children: Vec<TuiNode<'a>>, area: Rect) -> Measured<'a> {
        measure_node(TuiNode::fragment(children), area)
    }

    fn resolve_dim(
        dim: Option<DialogDimension>,
        content: u16,
        border: u16,
        parent_len: u16,
    ) -> u16 {
        match dim {
            None => (content + border).min(parent_len.max(border)),
            Some(DialogDimension::Percent(p)) => ((parent_len as u32 * p as u32) / 100) as u16,
            Some(DialogDimension::Cells(c)) => c.min(parent_len),
        }
    }
}

impl<'a> Widget for DialogWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }
        let borders = self.block.inner_block().inner(Rect::new(0, 0, 100, 100));
        let border_w = 100 - borders.width;
        let border_h = 100 - borders.height;

        let measured = Self::measure_content(self.children, area);

        let total_w = Self::resolve_dim(self.width, measured.content_width, border_w, area.width);
        let total_h =
            Self::resolve_dim(self.height, measured.content_height, border_h, area.height);

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

        blit_measured(&measured, inner_area, buf);
    }
}

/// Centers a dialog and projects its default slot into the dialog body.
#[component]
pub fn Dialog<'a>(
    #[bind] visible: State<bool>,
    #[prop] width: impl Into<DialogDimension>,
    #[slot(default)] dialog_content: TuiNode<'a>,
) -> TuiNode<'a> {
    let open = bind(visible);
    keybindings! {
        "esc" => lambda!(+open, || open.set(false)),
    }

    tui! {
        <DialogWidget::new visible={open.get()} width={width}>
            <{dialog_content} />
        </DialogWidget::new>
    }
}
