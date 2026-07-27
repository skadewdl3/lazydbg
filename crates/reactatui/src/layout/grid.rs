use crate::layout::Padding;
use crate::layout::size::{Size, parse_size_list, resolve_sizes};
use crate::layout::style::{Align, Style, align_rect, distribute};
use crate::measure::{Measured, blit_measured, measure_node};
use crate::node::TuiNode;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use std::collections::HashSet;

pub struct GridNode<'a> {
    columns: Vec<Size>,
    rows: Vec<Size>,
    gap_x: u16,
    gap_y: u16,
    padding: Padding,
    style: Style,
    items: Vec<GridItemNode<'a>>,
}

impl<'a> GridNode<'a> {
    pub fn new(items: impl Into<Vec<GridItemNode<'a>>>) -> Self {
        Self {
            columns: vec![Size::Fr(1)],
            rows: vec![Size::Fr(1)],
            gap_x: 0,
            gap_y: 0,
            padding: Padding::default(),
            style: Style::default(),
            items: items.into(),
        }
    }

    /// e.g. `"auto, 1fr, 20"`.
    pub fn columns(mut self, spec: impl AsRef<str>) -> Self {
        self.columns = parse_size_list(spec.as_ref());
        self
    }

    /// e.g. `"3, 1fr, auto"`.
    pub fn rows(mut self, spec: impl AsRef<str>) -> Self {
        self.rows = parse_size_list(spec.as_ref());
        self
    }

    pub fn gap(mut self, gap: u16) -> Self {
        self.gap_x = gap;
        self.gap_y = gap;
        self
    }
    pub fn gap_x(mut self, gap: u16) -> Self {
        self.gap_x = gap;
        self
    }
    pub fn gap_y(mut self, gap: u16) -> Self {
        self.gap_y = gap;
        self
    }
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// If the style specifies a non-zero `gap`, it's applied uniformly to
    /// both `gap_x` and `gap_y` (use `.gap_x()`/`.gap_y()` directly, called
    /// after `.style(..)`, for asymmetric gaps).
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        let style = style.into();
        if let Some(cols) = &style.columns {
            self.columns = cols.to_vec()
        }
        if let Some(rows) = &style.rows {
            self.rows = rows.to_vec()
        }
        if let Some(gx) = style.gap_x {
            self.gap_x = gx;
        } else if style.gap > 0 {
            self.gap_x = style.gap;
        }
        if let Some(gy) = style.gap_y {
            self.gap_y = gy;
        } else if style.gap > 0 {
            self.gap_y = style.gap;
        }
        if let Some(padding) = style.padding {
            self.padding = padding;
        }
        self.style = style;
        self
    }

    pub fn natural_size(&self, viewport: (u16, u16)) -> (u16, u16) {
        let col_total = Self::track_total(&self.columns, viewport.0);
        let row_total = Self::track_total(&self.rows, viewport.1);

        let gap_x_total = self
            .gap_x
            .saturating_mul((self.columns.len() as u16).saturating_sub(1));
        let gap_y_total = self
            .gap_y
            .saturating_mul((self.rows.len() as u16).saturating_sub(1));

        let (pad_top, pad_right, pad_bottom, pad_left) = self.padding.amounts();

        let width = col_total
            .saturating_add(gap_x_total)
            .saturating_add(pad_left)
            .saturating_add(pad_right);
        let height = row_total
            .saturating_add(gap_y_total)
            .saturating_add(pad_top)
            .saturating_add(pad_bottom);

        (width, height)
    }

    fn track_total(tracks: &[Size], reference: u16) -> u16 {
        let mut total: u32 = 0;
        for track in tracks {
            total += match track {
                Size::Length(n) => u32::from(*n),
                Size::Percent(p) => (u32::from(reference) * u32::from(*p)) / 100,
                Size::Fr(_) | Size::Auto => 0,
            };
        }
        total.min(u32::from(u16::MAX)) as u16
    }
}

pub struct GridItemNode<'a> {
    style: Style,
    node: TuiNode<'a>,
}

impl<'a> GridItemNode<'a> {
    pub fn new(node: impl Into<TuiNode<'a>>) -> Self {
        let (style, node) = node.into().take_style();
        Self { style, node }
    }

    /// Item-level: `column`/`row`/`column_span`/`row_span` (placement,
    /// `None` for auto-flow), `align_self`/`justify_self` (per-cell
    /// alignment overrides), `size` (an explicit hint for the cell's own
    /// dimension when it sits in an `auto` column/row — skips measuring
    /// the same way it would in a Flex item).
    pub fn style(mut self, style: impl Into<crate::layout::Style>) -> Self {
        self.style = style.into();
        self
    }

    pub fn flatten_fragments(self) -> Vec<Self> {
        let Self { style, node } = self;
        match node {
            TuiNode::Styled(inner, s) => Self {
                style: style.merge(&s),
                node: *inner,
            }
            .flatten_fragments(),
            TuiNode::Fragment(children) => children
                .into_iter()
                .flat_map(|child| {
                    let (child_style, child_node) = match child {
                        TuiNode::Styled(inner, s) => (style.merge(&s), *inner),
                        other => (style.clone(), other),
                    };
                    Self {
                        style: child_style,
                        node: child_node,
                    }
                    .flatten_fragments()
                })
                .collect(),
            TuiNode::Empty => Vec::new(),
            node => vec![Self { style, node }],
        }
    }
}

/// CSS-grid-style auto-placement: items with an explicit `column`/`row`
/// occupy their declared cells; everything else flows row-major into the
/// first free cell, skipping cells already taken by explicit placements.
fn auto_place(col_count: usize, styles: &[Style]) -> Vec<(usize, usize)> {
    let mut occupied: HashSet<(usize, usize)> = HashSet::new();

    // First pass: reserve every explicitly-placed item's cells so
    // auto-placed items flow around them.
    for style in styles {
        if let (Some(c), Some(r)) = (style.column, style.row) {
            for dc in 0..style.resolved_column_span().max(1) {
                for dr in 0..style.resolved_row_span().max(1) {
                    occupied.insert((c + dc, r + dr));
                }
            }
        }
    }

    let mut placements = Vec::with_capacity(styles.len());
    let mut auto_cursor = (0usize, 0usize);

    for style in styles {
        let col_span = style.resolved_column_span().max(1);
        let row_span = style.resolved_row_span().max(1);

        let (col, row) = match (style.column, style.row) {
            (Some(c), Some(r)) => (c, r),
            (Some(c), None) => {
                let mut r = 0usize;
                loop {
                    if (0..row_span).all(|dr| !occupied.contains(&(c, r + dr))) {
                        break;
                    }
                    r += 1;
                }
                for dr in 0..row_span {
                    occupied.insert((c, r + dr));
                }
                (c, r)
            }
            (None, Some(r)) => {
                let mut c = 0usize;
                loop {
                    let fits = c + col_span <= col_count.max(col_span);
                    if fits && (0..col_span).all(|dc| !occupied.contains(&(c + dc, r))) {
                        break;
                    }
                    c += 1;
                }
                for dc in 0..col_span {
                    occupied.insert((c + dc, r));
                }
                (c, r)
            }
            (None, None) => {
                let (mut c, mut r) = auto_cursor;
                loop {
                    if c + col_span > col_count.max(col_span) {
                        c = 0;
                        r += 1;
                        continue;
                    }
                    if (0..col_span).all(|dc| !occupied.contains(&(c + dc, r))) {
                        break;
                    }
                    c += 1;
                }
                for dc in 0..col_span {
                    occupied.insert((c + dc, r));
                }
                auto_cursor = (c + col_span, r);
                (c, r)
            }
        };
        placements.push((col, row));
    }

    placements
}

/// Extent covered by a span of tracks, derived from already-computed
/// offsets (so it automatically accounts for `justify_content`/
/// `align_content` extra gaps inserted between tracks, not just the
/// container's fixed `gap`).
fn span_extent_from_offsets(offsets: &[u16], sizes: &[u16], start: usize, span: usize) -> u16 {
    let end = start.saturating_add(span).min(sizes.len());
    if end == 0 || end <= start {
        return 0;
    }
    let last = end - 1;
    let last_end = offsets[last].saturating_add(sizes[last]);
    last_end.saturating_sub(offsets[start])
}

fn track_offsets(sizes: &[u16], gap: u16, extra_gaps: &[u16], leading: u16) -> Vec<u16> {
    let mut offsets = Vec::with_capacity(sizes.len());
    let mut cursor = leading;
    for (i, &s) in sizes.iter().enumerate() {
        cursor = cursor.saturating_add(extra_gaps.get(i).copied().unwrap_or(0));
        offsets.push(cursor);
        cursor = cursor.saturating_add(s).saturating_add(gap);
    }
    offsets
}

/// Resolve an item's own `size` into a concrete cell-count hint for an
/// `auto` track, if `size` gives us one directly (`Length`/`Percent`)
/// without needing to measure content. `Fr`/`Auto` return `None` — `Fr`
/// has no meaning inside a per-cell auto-track hint (grid tracks don't
/// grow the way flex items do), and `Auto` means "go measure it".
fn size_hint(size: Size, reference: u16) -> Option<u16> {
    match size {
        Size::Length(n) => Some(n),
        Size::Percent(p) => Some(((u32::from(reference) * u32::from(p)) / 100) as u16),
        Size::Fr(_) | Size::Auto => None,
    }
}

impl<'a> Widget for GridNode<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = self.padding.apply(area);

        let mut columns = self.columns;
        let mut rows = self.rows;
        let gap_x = self.gap_x;
        let gap_y = self.gap_y;
        let container_style = self.style;
        let items: Vec<_> = self
            .items
            .into_iter()
            .flat_map(GridItemNode::flatten_fragments)
            .collect();

        if columns.is_empty()
            || rows.is_empty()
            || area.width == 0
            || area.height == 0
            || items.is_empty()
        {
            return;
        }

        let styles: Vec<Style> = items.iter().map(|it| it.style.clone()).collect();
        let mut nodes: Vec<Option<TuiNode<'a>>> =
            items.into_iter().map(|it| Some(it.node)).collect();

        let placements = auto_place(columns.len(), &styles);

        // CSS grid's "implicit tracks": if placement (explicit or
        // auto-flowed) needs more columns/rows than declared, extend with
        // auto-sized implicit tracks rather than panicking or clipping.
        let needed_cols = placements
            .iter()
            .zip(&styles)
            .map(|((c, _), s)| c + s.column_span.max(1))
            .max()
            .unwrap_or(0);
        let needed_rows = placements
            .iter()
            .zip(&styles)
            .map(|((_, r), s)| r + s.row_span.max(1))
            .max()
            .unwrap_or(0);
        while columns.len() < needed_cols {
            columns.push(Size::Auto);
        }
        while rows.len() < needed_rows {
            rows.push(Size::Auto);
        }

        let col_count = columns.len();
        let row_count = rows.len();

        let total_gap_x = gap_x.saturating_mul((col_count as u16).saturating_sub(1));
        let total_gap_y = gap_y.saturating_mul((row_count as u16).saturating_sub(1));
        let available_w = area.width.saturating_sub(total_gap_x);
        let available_h = area.height.saturating_sub(total_gap_y);

        // Measure any item in an auto (non-spanning) column/row, or with
        // non-Stretch alignment on either axis — alignment needs intrinsic
        // size regardless of which track type placed the item. An item
        // can skip measuring by giving its own `size` (Length/Percent) —
        // that hint seeds the auto track directly, same idea as a Flex
        // item's `size` skipping the content probe.
        let mut premeasured: Vec<Option<Measured<'a>>> = (0..nodes.len()).map(|_| None).collect();
        let mut auto_col_size = vec![0u16; col_count];
        let mut auto_row_size = vec![0u16; row_count];

        // Same reasoning as Flex: probe auto tracks at a fair-share
        // estimate rather than the whole container, so widgets with no
        // real intrinsic size (bordered Blocks) don't get measured far
        // larger than their eventual cell and clipped when blitted in.
        let auto_col_count = columns
            .iter()
            .filter(|c| matches!(c, Size::Auto))
            .count()
            .max(1) as u16;
        let auto_row_count = rows
            .iter()
            .filter(|c| matches!(c, Size::Auto))
            .count()
            .max(1) as u16;
        let known_col_total: u16 = columns
            .iter()
            .map(|c| match c {
                Size::Length(n) => *n,
                Size::Percent(p) => ((u32::from(available_w) * u32::from(*p)) / 100) as u16,
                Size::Fr(_) | Size::Auto => 0,
            })
            .sum();
        let known_row_total: u16 = rows
            .iter()
            .map(|r| match r {
                Size::Length(n) => *n,
                Size::Percent(p) => ((u32::from(available_h) * u32::from(*p)) / 100) as u16,
                Size::Fr(_) | Size::Auto => 0,
            })
            .sum();
        let fair_col_w = available_w.saturating_sub(known_col_total) / auto_col_count;
        let fair_row_h = available_h.saturating_sub(known_row_total) / auto_row_count;

        for (i, ((col, row), style)) in placements.iter().zip(styles.iter()).enumerate() {
            let col_span = style.column_span.max(1);
            let row_span = style.row_span.max(1);
            let in_auto_col = col_span == 1 && matches!(columns.get(*col), Some(Size::Auto));
            let in_auto_row = row_span == 1 && matches!(rows.get(*row), Some(Size::Auto));

            let col_hint = in_auto_col
                .then(|| size_hint(style.size, available_w))
                .flatten();
            let row_hint = in_auto_row
                .then(|| size_hint(style.size, available_h))
                .flatten();

            if let Some(w) = col_hint {
                auto_col_size[*col] = auto_col_size[*col].max(w);
            }
            if let Some(h) = row_hint {
                auto_row_size[*row] = auto_row_size[*row].max(h);
            }

            let needs_auto_measure =
                (in_auto_col && col_hint.is_none()) || (in_auto_row && row_hint.is_none());

            if needs_auto_measure {
                let node = nodes[i].take().expect("node already taken");
                let probe_w = if in_auto_col && col_hint.is_none() {
                    fair_col_w.max(1)
                } else {
                    available_w
                };
                let probe_h = if in_auto_row && row_hint.is_none() {
                    fair_row_h.max(1)
                } else {
                    available_h
                };
                // Probe always at (0,0) so blit_measured's source offset is consistent.
                let probe = Rect::new(0, 0, probe_w, probe_h);
                let measured = measure_node(node, probe);
                if in_auto_col && col_hint.is_none() {
                    auto_col_size[*col] = auto_col_size[*col].max(measured.content_width);
                }
                if in_auto_row && row_hint.is_none() {
                    auto_row_size[*row] = auto_row_size[*row].max(measured.content_height);
                }
                premeasured[i] = Some(measured);
            }
        }

        let col_sizes = resolve_sizes(&columns, available_w, &auto_col_size);
        let row_sizes = resolve_sizes(&rows, available_h, &auto_row_size);

        let col_used = col_sizes.iter().map(|&s| u16::from(s)).sum::<u16>() as u16 + total_gap_x;
        let row_used = row_sizes.iter().map(|&s| u16::from(s)).sum::<u16>() as u16 + total_gap_y;

        // Real per-axis track distribution: `justify_content` spreads
        // leftover space among *columns*, `align_content` among *rows* —
        // genuinely independent, unlike the single-value approximation
        // from before.
        let (col_leading, col_extra_gaps) = distribute(
            container_style.justify_content,
            area.width,
            col_used,
            col_count,
        );
        let (row_leading, row_extra_gaps) = distribute(
            container_style.align_content,
            area.height,
            row_used,
            row_count,
        );

        let col_offsets = track_offsets(&col_sizes, gap_x, &col_extra_gaps, col_leading);
        let row_offsets = track_offsets(&row_sizes, gap_y, &row_extra_gaps, row_leading);
        //
        // Second pass: items that only need measuring for non-Stretch alignment
        // (not auto-track sizing) are probed now, at their real final cell size —
        // deferred from the pass above because that size wasn't known until the
        // tracks were resolved.
        for (i, ((col, row), style)) in placements.iter().zip(styles.iter()).enumerate() {
            if premeasured[i].is_some() {
                continue;
            }
            let align_row = Style::resolve_align(&container_style, style);
            let align_col = Style::resolve_justify_self(&container_style, style);
            if align_row == Align::Stretch && align_col == Align::Stretch {
                continue;
            }
            let col_span = style.resolved_column_span().max(1);
            let row_span = style.resolved_row_span().max(1);
            let cell_w = span_extent_from_offsets(&col_offsets, &col_sizes, *col, col_span).max(1);
            let cell_h = span_extent_from_offsets(&row_offsets, &row_sizes, *row, row_span).max(1);
            let node = nodes[i].take().expect("node already taken");
            // Probe always at (0,0) for consistent blit origins.
            let probe = Rect::new(0, 0, cell_w, cell_h);
            premeasured[i] = Some(measure_node(node, probe));
        }

        for (i, ((col, row), style)) in placements.into_iter().zip(styles.into_iter()).enumerate() {
            let col_span = style.resolved_column_span().max(1);
            let row_span = style.resolved_row_span().max(1);

            let cell_w = span_extent_from_offsets(&col_offsets, &col_sizes, col, col_span);
            let cell_h = span_extent_from_offsets(&row_offsets, &row_sizes, row, row_span);
            let cell_x = area.x + col_offsets.get(col).copied().unwrap_or(0);
            let cell_y = area.y + row_offsets.get(row).copied().unwrap_or(0);
            let cell = Rect::new(cell_x, cell_y, cell_w, cell_h);

            let align_row = Style::resolve_align(&container_style, &style);
            let align_col = Style::resolve_justify_self(&container_style, &style);

            if let Some(measured) = premeasured[i].take() {
                let target = align_rect(
                    cell,
                    align_row,
                    align_col,
                    Some(measured.content_width),
                    Some(measured.content_height),
                );
                blit_measured(&measured, target, buf);
            } else {
                let node = nodes[i].take().expect("node already taken");
                node.render(cell, buf);
            }
        }
    }
}
