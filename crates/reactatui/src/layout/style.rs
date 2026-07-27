use crate::layout::size::Size;
use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    // ---- Container-level ----
    /// Flex direction (Horizontal / Vertical).
    pub direction: Option<ratatui::layout::Direction>,
    /// Main axis (flex) / column axis (grid) distribution of leftover space.
    pub justify_content: Justify,
    /// Cross axis (flex, unused without wrap) / row axis (grid) distribution.
    pub align_content: Justify,
    /// Default cross-axis (flex) / row-axis (grid) alignment for children.
    pub align_items: Align,
    /// Default column-axis alignment for children (grid only).
    pub justify_items: Align,
    /// Container-level: gap between items (applies to both axes if gap_x/gap_y not specified).
    pub gap: u16,
    /// Column/Horizontal gap override.
    pub gap_x: Option<u16>,
    /// Row/Vertical gap override.
    pub gap_y: Option<u16>,
    /// Padding around the contents of this container.
    pub padding: Option<crate::layout::Padding>,
    /// Grid container columns specification.
    pub columns: Option<Vec<Size>>,
    /// Grid container rows specification.
    pub rows: Option<Vec<Size>>,

    // ---- Item-level ----
    /// Overrides the container's `align_items` for this item only.
    pub align_self: Option<Align>,
    /// Overrides the container's `justify_items` for this item only (grid only).
    pub justify_self: Option<Align>,
    /// Item-level, Flex only: pulls this item out of normal flex flow
    /// entirely — it renders on top, full area, and positions itself.
    pub ignore: bool,

    /// Size along the primary axis.
    pub size: Size,
    /// Explicit width override.
    pub width: Option<Size>,
    /// Explicit height override.
    pub height: Option<Size>,
    /// Minimum width constraint.
    pub min_width: Option<u16>,
    /// Maximum width constraint.
    pub max_width: Option<u16>,
    /// Minimum height constraint.
    pub min_height: Option<u16>,
    /// Maximum height constraint.
    pub max_height: Option<u16>,
    /// Flex grow factor.
    pub grow: Option<f32>,
    /// Flex shrink factor.
    pub shrink: f32,

    /// Grid placement column start (0-indexed).
    pub column: Option<usize>,
    /// Grid placement row start (0-indexed).
    pub row: Option<usize>,
    /// Grid column span (default 1).
    pub column_span: usize,
    /// Grid row span (default 1).
    pub row_span: usize,
    /// Grid placement column end (0-indexed line).
    pub column_end: Option<usize>,
    /// Grid placement row end (0-indexed line).
    pub row_end: Option<usize>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            direction: None,
            justify_content: Justify::Start,
            align_content: Justify::Start,
            align_items: Align::Stretch,
            justify_items: Align::Stretch,
            gap: 0,
            gap_x: None,
            gap_y: None,
            padding: None,
            columns: None,
            rows: None,
            align_self: None,
            justify_self: None,
            ignore: false,
            size: Size::Auto,
            width: None,
            height: None,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            grow: None,
            shrink: 1.0,
            column: None,
            row: None,
            column_span: 1,
            row_span: 1,
            column_end: None,
            row_end: None,
        }
    }
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn direction(mut self, d: impl Into<ratatui::layout::Direction>) -> Self {
        self.direction = Some(d.into());
        self
    }
    pub fn flex_direction(self, d: impl Into<ratatui::layout::Direction>) -> Self {
        self.direction(d)
    }
    pub fn justify_content(mut self, j: Justify) -> Self {
        self.justify_content = j;
        self
    }
    pub fn align_content(mut self, j: Justify) -> Self {
        self.align_content = j;
        self
    }
    pub fn align_items(mut self, a: Align) -> Self {
        self.align_items = a;
        self
    }
    pub fn justify_items(mut self, a: Align) -> Self {
        self.justify_items = a;
        self
    }
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }
    pub fn gap_x(mut self, gap: u16) -> Self {
        self.gap_x = Some(gap);
        self
    }
    pub fn column_gap(self, gap: u16) -> Self {
        self.gap_x(gap)
    }
    pub fn gap_y(mut self, gap: u16) -> Self {
        self.gap_y = Some(gap);
        self
    }
    pub fn row_gap(self, gap: u16) -> Self {
        self.gap_y(gap)
    }
    pub fn padding(mut self, padding: impl Into<crate::layout::Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }
    pub fn padding_top(mut self, top: u16) -> Self {
        let mut p = self.padding.unwrap_or_default();
        p.top = top;
        self.padding = Some(p);
        self
    }
    pub fn pad_top(self, top: u16) -> Self {
        self.padding_top(top)
    }
    pub fn padding_right(mut self, right: u16) -> Self {
        let mut p = self.padding.unwrap_or_default();
        p.right = right;
        self.padding = Some(p);
        self
    }
    pub fn pad_right(self, right: u16) -> Self {
        self.padding_right(right)
    }
    pub fn padding_bottom(mut self, bottom: u16) -> Self {
        let mut p = self.padding.unwrap_or_default();
        p.bottom = bottom;
        self.padding = Some(p);
        self
    }
    pub fn pad_bottom(self, bottom: u16) -> Self {
        self.padding_bottom(bottom)
    }
    pub fn padding_left(mut self, left: u16) -> Self {
        let mut p = self.padding.unwrap_or_default();
        p.left = left;
        self.padding = Some(p);
        self
    }
    pub fn pad_left(self, left: u16) -> Self {
        self.padding_left(left)
    }
    pub fn columns(mut self, cols: impl crate::layout::size::IntoSizeList) -> Self {
        self.columns = Some(cols.into_size_list());
        self
    }
    pub fn grid_template_columns(self, cols: impl crate::layout::size::IntoSizeList) -> Self {
        self.columns(cols)
    }
    pub fn rows(mut self, rows: impl crate::layout::size::IntoSizeList) -> Self {
        self.rows = Some(rows.into_size_list());
        self
    }
    pub fn grid_template_rows(self, rows: impl crate::layout::size::IntoSizeList) -> Self {
        self.rows(rows)
    }

    pub fn align_self(mut self, a: Align) -> Self {
        self.align_self = Some(a);
        self
    }
    pub fn justify_self(mut self, a: Align) -> Self {
        self.justify_self = Some(a);
        self
    }
    pub fn ignore(mut self) -> Self {
        self.ignore = true;
        self
    }
    pub fn size(mut self, s: impl Into<Size>) -> Self {
        self.size = s.into();
        self
    }
    pub fn width(mut self, w: impl Into<Size>) -> Self {
        self.width = Some(w.into());
        self
    }
    pub fn height(mut self, h: impl Into<Size>) -> Self {
        self.height = Some(h.into());
        self
    }
    pub fn min_width(mut self, w: u16) -> Self {
        self.min_width = Some(w);
        self
    }
    pub fn max_width(mut self, w: u16) -> Self {
        self.max_width = Some(w);
        self
    }
    pub fn min_height(mut self, h: u16) -> Self {
        self.min_height = Some(h);
        self
    }
    pub fn max_height(mut self, h: u16) -> Self {
        self.max_height = Some(h);
        self
    }
    pub fn grow(mut self, g: f32) -> Self {
        self.grow = Some(g.max(0.0));
        self
    }
    pub fn flex_grow(self, g: f32) -> Self {
        self.grow(g)
    }
    pub fn shrink(mut self, s: f32) -> Self {
        self.shrink = s.max(0.0);
        self
    }
    pub fn flex_shrink(self, s: f32) -> Self {
        self.shrink(s)
    }
    pub fn column(mut self, c: usize) -> Self {
        self.column = Some(c);
        self
    }
    pub fn column_start(self, c: usize) -> Self {
        self.column(c)
    }
    pub fn grid_column_start(self, c: usize) -> Self {
        self.column(c)
    }
    pub fn row(mut self, r: usize) -> Self {
        self.row = Some(r);
        self
    }
    pub fn row_start(self, r: usize) -> Self {
        self.row(r)
    }
    pub fn grid_row_start(self, r: usize) -> Self {
        self.row(r)
    }
    pub fn column_span(mut self, s: usize) -> Self {
        self.column_span = s.max(1);
        self
    }
    pub fn grid_column_span(self, s: usize) -> Self {
        self.column_span(s)
    }
    pub fn row_span(mut self, s: usize) -> Self {
        self.row_span = s.max(1);
        self
    }
    pub fn grid_row_span(self, s: usize) -> Self {
        self.row_span(s)
    }
    pub fn column_end(mut self, c: usize) -> Self {
        self.column_end = Some(c);
        self
    }
    pub fn grid_column_end(self, c: usize) -> Self {
        self.column_end(c)
    }
    pub fn row_end(mut self, r: usize) -> Self {
        self.row_end = Some(r);
        self
    }
    pub fn grid_row_end(self, r: usize) -> Self {
        self.row_end(r)
    }

    pub fn resolved_column_span(&self) -> usize {
        if let (Some(start), Some(end)) = (self.column, self.column_end) {
            if end > start {
                return end - start;
            }
        }
        self.column_span
    }

    pub fn resolved_row_span(&self) -> usize {
        if let (Some(start), Some(end)) = (self.row, self.row_end) {
            if end > start {
                return end - start;
            }
        }
        self.row_span
    }

    pub fn resolve_align(container: &Style, item: &Style) -> Align {
        item.align_self.unwrap_or(container.align_items)
    }

    pub fn resolve_justify_self(container: &Style, item: &Style) -> Align {
        item.justify_self.unwrap_or(container.justify_items)
    }
}

pub fn clamp_rect(mut rect: Rect, style: &Style) -> Rect {
    if let Some(min_w) = style.min_width {
        rect.width = rect.width.max(min_w);
    }
    if let Some(max_w) = style.max_width {
        rect.width = rect.width.min(max_w);
    }
    if let Some(min_h) = style.min_height {
        rect.height = rect.height.max(min_h);
    }
    if let Some(max_h) = style.max_height {
        rect.height = rect.height.min(max_h);
    }
    rect
}

/// Position measured content within `cell` on both axes independently
/// (grid — row and column alignment are genuinely independent in CSS grid).
pub fn align_rect(
    cell: Rect,
    align_row: Align,
    align_col: Align,
    content_w: Option<u16>,
    content_h: Option<u16>,
) -> Rect {
    let w = match align_col {
        Align::Stretch => cell.width,
        _ => content_w.unwrap_or(cell.width).min(cell.width),
    };
    let h = match align_row {
        Align::Stretch => cell.height,
        _ => content_h.unwrap_or(cell.height).min(cell.height),
    };
    let x_off = match align_col {
        Align::Start | Align::Stretch => 0,
        Align::Center => (cell.width - w) / 2,
        Align::End => cell.width.saturating_sub(w),
    };
    let y_off = match align_row {
        Align::Start | Align::Stretch => 0,
        Align::Center => (cell.height - h) / 2,
        Align::End => cell.height.saturating_sub(h),
    };
    Rect::new(cell.x + x_off, cell.y + y_off, w, h)
}

/// Compute a leading offset and per-item extra gap for main-axis (or
/// track-axis) distribution, given `available` space, space already
/// consumed (`used`), and item/track `count`.
pub fn distribute(justify: Justify, available: u16, used: u16, count: usize) -> (u16, Vec<u16>) {
    if count == 0 {
        return (0, Vec::new());
    }
    let leftover = available.saturating_sub(used);
    if leftover == 0 {
        return (0, vec![0; count]);
    }

    match justify {
        Justify::Start => (0, vec![0; count]),
        Justify::End => (leftover, vec![0; count]),
        Justify::Center => (leftover / 2, vec![0; count]),
        Justify::SpaceBetween => {
            if count <= 1 {
                return (0, vec![0; count]);
            }
            let gaps = count as u16 - 1;
            let each = leftover / gaps;
            let mut rem = leftover % gaps;
            let mut extra = vec![0u16; count];
            for slot in extra.iter_mut().skip(1) {
                *slot = each + u16::from(rem > 0);
                rem = rem.saturating_sub(1);
            }
            (0, extra)
        }
        Justify::SpaceAround => {
            let unit = leftover / (count as u16 * 2);
            let mut rem = leftover.saturating_sub(unit * count as u16 * 2);
            let leading = unit;
            let mut extra = vec![0u16; count];
            for (i, slot) in extra.iter_mut().enumerate() {
                if i == 0 {
                    continue;
                }
                *slot = unit * 2 + u16::from(rem > 0);
                rem = rem.saturating_sub(1);
            }
            (leading, extra)
        }
        Justify::SpaceEvenly => {
            let gaps = count as u16 + 1;
            let each = leftover / gaps;
            let mut rem = leftover % gaps;
            let leading = each + u16::from(rem > 0);
            rem = rem.saturating_sub(u16::from(rem > 0));
            let mut extra = vec![0u16; count];
            for slot in extra.iter_mut().skip(1) {
                *slot = each + u16::from(rem > 0);
                rem = rem.saturating_sub(1);
            }
            (leading, extra)
        }
    }
}
