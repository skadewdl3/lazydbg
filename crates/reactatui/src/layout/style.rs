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

/// Unified layout style, applied at both container and item level — same
/// shape CSS uses (an element's `align-items` is a sibling concept to its
/// own `align-self`). Irrelevant fields at a given level are just unused.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    // ---- Container-level ----
    /// Main axis (flex) / column axis (grid) distribution of leftover space.
    pub justify_content: Justify,
    /// Cross axis (flex, unused without wrap) / row axis (grid) distribution.
    pub align_content: Justify,
    /// Default cross-axis (flex) / row-axis (grid) alignment for children.
    pub align_items: Align,
    /// Default column-axis alignment for children (grid only).
    pub justify_items: Align,

    // ---- Item-level ----
    /// Overrides the container's `align_items` for this item only.
    pub align_self: Option<Align>,
    /// Overrides the container's `justify_items` for this item only (grid only).
    pub justify_self: Option<Align>,

    /// Size along the primary axis. Shared, single-name property across
    /// Flex and Grid:
    /// - `Auto` (the default, everywhere): measure intrinsic content.
    /// - `Length`/`Percent`: pin an explicit size.
    /// - `Fr(n)`: grow to take a proportional share of leftover space.
    ///   This is the *only* way an item grows — nothing grows implicitly.
    ///   In Grid, `Fr` on an item has no effect (grid tracks, not items,
    ///   carry `Fr` sizing); it's only meaningful on Flex items.
    pub size: Size,
    /// Flex-only: how eagerly this item shrinks below `size` on overflow,
    /// weighted the CSS way (`shrink * basis`). No effect in Grid — grid
    /// tracks never shrink below their resolved size, same as CSS Grid.
    pub shrink: f32,

    pub gap: u16,

    /// Grid placement. `None` triggers CSS-grid-style auto-placement.
    pub column: Option<usize>,
    pub row: Option<usize>,
    pub column_span: usize,
    pub row_span: usize,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            justify_content: Justify::Start,
            align_content: Justify::Start,
            align_items: Align::Stretch,
            justify_items: Align::Stretch,
            align_self: None,
            justify_self: None,
            size: Size::Auto,
            shrink: 1.0,
            gap: 0,
            column: None,
            row: None,
            column_span: 1,
            row_span: 1,
        }
    }
}

impl Style {
    pub fn new() -> Self {
        Self::default()
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
    pub fn align_self(mut self, a: Align) -> Self {
        self.align_self = Some(a);
        self
    }
    pub fn justify_self(mut self, a: Align) -> Self {
        self.justify_self = Some(a);
        self
    }
    pub fn size(mut self, s: impl Into<Size>) -> Self {
        self.size = s.into();
        self
    }
    pub fn shrink(mut self, s: f32) -> Self {
        self.shrink = s.max(0.0);
        self
    }
    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }
    pub fn column(mut self, c: usize) -> Self {
        self.column = Some(c);
        self
    }
    pub fn row(mut self, r: usize) -> Self {
        self.row = Some(r);
        self
    }
    pub fn column_span(mut self, s: usize) -> Self {
        self.column_span = s.max(1);
        self
    }
    pub fn row_span(mut self, s: usize) -> Self {
        self.row_span = s.max(1);
        self
    }

    pub fn resolve_align(container: &Style, item: &Style) -> Align {
        item.align_self.unwrap_or(container.align_items)
    }

    pub fn resolve_justify_self(container: &Style, item: &Style) -> Align {
        item.justify_self.unwrap_or(container.justify_items)
    }
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
