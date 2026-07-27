use ratatui::{
    buffer::Buffer,
    layout::{Direction, Rect},
    widgets::Widget,
};

use crate::layout::Padding;
use crate::layout::size::Size;
use crate::layout::style::{Align, Style, distribute};
use crate::measure::{Measured, blit_measured, measure_node};
use crate::node::TuiNode;

pub struct FlexNode<'a> {
    direction: Direction,
    gap: u16,
    padding: Padding,
    style: Style,
    items: Vec<FlexItemNode<'a>>,
}

impl<'a> FlexNode<'a> {
    pub fn new(items: impl Into<Vec<FlexItemNode<'a>>>) -> Self {
        Self {
            direction: Direction::Vertical,
            gap: 0,
            padding: Padding::default(),
            style: Style::default(),
            items: items.into(),
        }
    }

    pub fn horizontal(items: impl Into<Vec<FlexItemNode<'a>>>) -> Self {
        Self::new(items).direction(Direction::Horizontal)
    }

    pub fn vertical(items: impl Into<Vec<FlexItemNode<'a>>>) -> Self {
        Self::new(items).direction(Direction::Vertical)
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Container-level: `justify_content` (main-axis distribution),
    /// `align_items` (default cross-axis alignment for children), `direction`, `gap`, `padding`.
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        let style = style.into();
        if let Some(dir) = style.direction {
            self.direction = dir;
        }
        if let Some(gx) = style.gap_x {
            if self.direction == Direction::Horizontal {
                self.gap = gx;
            }
        }
        if let Some(gy) = style.gap_y {
            if self.direction == Direction::Vertical {
                self.gap = gy;
            }
        }
        if style.gap > 0 && style.gap_x.is_none() && style.gap_y.is_none() {
            self.gap = style.gap;
        }
        if let Some(padding) = style.padding {
            self.padding = padding;
        }
        self.style = style;
        self
    }

    /// Returns the statically-known minimum size: only `Length`/`Percent`
    /// items contribute; `Auto` and `Fr` items contribute 0 (their sizes
    /// are not known without rendering). Use this for a cheap lower-bound
    /// estimate (e.g. in `Scroll`'s fast path).
    ///
    /// Returns `(width, height)`.
    pub fn natural_size(&self, cross_axis_hint: u16) -> (u16, u16) {
        let (_ignored, participating): (Vec<_>, Vec<_>) =
            self.items.iter().partition(|item| item.style.ignore);

        let gap_total = self
            .gap
            .saturating_mul((participating.len() as u16).saturating_sub(1));
        let mut main: u32 = u32::from(gap_total);

        for item in &participating {
            main += match item.style.size {
                Size::Length(n) => u32::from(n),
                Size::Percent(_) | Size::Fr(_) | Size::Auto => 0,
            };
        }

        let (pad_top, pad_right, pad_bottom, pad_left) = self.padding.amounts();
        let main_pad = match self.direction {
            Direction::Horizontal => pad_left.saturating_add(pad_right),
            Direction::Vertical => pad_top.saturating_add(pad_bottom),
        };
        let main = (main.saturating_add(u32::from(main_pad))).min(u32::from(u16::MAX)) as u16;

        match self.direction {
            Direction::Horizontal => (main, cross_axis_hint),
            Direction::Vertical => (cross_axis_hint, main),
        }
    }
}

pub struct FlexItemNode<'a> {
    style: Style,
    node: TuiNode<'a>,
}

impl<'a> FlexItemNode<'a> {
    pub fn new(node: impl Into<TuiNode<'a>>) -> Self {
        let (style, node) = node.into().take_style();
        Self { style, node }
    }

    fn flatten_fragments(self) -> Vec<Self> {
        let Self { style, node } = self;
        match node {
            TuiNode::Styled(inner, s) => Self {
                style: s,
                node: *inner,
            }
            .flatten_fragments(),
            TuiNode::Fragment(children) => children
                .into_iter()
                .flat_map(|child| {
                    let (child_style, child_node) = match child {
                        TuiNode::Styled(inner, s) => (s, *inner),
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

    /// Item-level: `size` (main-axis sizing — `auto` measures content,
    /// `Length`/`Percent` pin it, `"Nfr"` grows to take a share of
    /// leftover space — this is the *only* way an item grows), `shrink`
    /// (how eagerly it gives back space below `size` on overflow — the
    /// one property that's genuinely flex-only, since grid tracks never
    /// shrink), and `align_self` (cross-axis override).
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.style = style.into();
        self
    }
}

fn align_cross(
    item_area: Rect,
    align: Align,
    direction: Direction,
    content_w: u16,
    content_h: u16,
) -> Rect {
    if align == Align::Stretch {
        return item_area;
    }
    match direction {
        Direction::Horizontal => {
            let h = content_h.min(item_area.height);
            let y_off = match align {
                Align::Start => 0,
                Align::Center => (item_area.height - h) / 2,
                Align::End => item_area.height - h,
                Align::Stretch => unreachable!(),
            };
            Rect::new(item_area.x, item_area.y + y_off, item_area.width, h)
        }
        Direction::Vertical => {
            let w = content_w.min(item_area.width);
            let x_off = match align {
                Align::Start => 0,
                Align::Center => (item_area.width - w) / 2,
                Align::End => item_area.width - w,
                Align::Stretch => unreachable!(),
            };
            Rect::new(item_area.x + x_off, item_area.y, w, item_area.height)
        }
    }
}

impl<'a> Widget for FlexNode<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = self.padding.apply(area);
        let direction = self.direction;
        let gap = self.gap;
        let container_style = self.style;
        let items: Vec<_> = self
            .items
            .into_iter()
            .flat_map(FlexItemNode::flatten_fragments)
            .collect();

        let (ignored, participating): (Vec<_>, Vec<_>) =
            items.into_iter().partition(|item| item.style.ignore);

        let count = participating.len();

        if count != 0 && area.width != 0 && area.height != 0 {
            let styles: Vec<Style> = participating.iter().map(|it| it.style.clone()).collect();
            let mut nodes: Vec<Option<TuiNode<'a>>> =
                participating.into_iter().map(|it| Some(it.node)).collect();

            let total_gap = gap.saturating_mul((count as u16).saturating_sub(1));
            let available = match direction {
                Direction::Horizontal => area.width.saturating_sub(total_gap),
                Direction::Vertical => area.height.saturating_sub(total_gap),
            };
            let cross_available = match direction {
                Direction::Horizontal => area.height,
                Direction::Vertical => area.width,
            };

            // Resolve each item's main-axis contribution:
            // - `Length`/`Percent` pin a concrete basis directly.
            // - `Fr(f)` contributes zero basis but registers a grow weight —
            //   `Fr` is the *only* way an item grows; there is no implicit
            //   "Auto grows by default" anymore.
            // - `Auto` (or any item with non-Stretch cross-axis alignment)
            //   needs measuring before we know its basis.
            let mut premeasured: Vec<Option<Measured<'a>>> = (0..count).map(|_| None).collect();
            let mut basis = vec![0u16; count];
            let mut grow = vec![0f32; count];
            let mut is_auto = vec![false; count];

            // Estimate a "fair share" of the remaining budget for Auto
            // items *before* measuring any of them. Widgets with no real
            // intrinsic size (e.g. bordered Blocks) always fill whatever
            // rect they're probed with — measuring them against the full
            // remaining budget makes every Auto item report a basis equal
            // to that whole budget, forcing a shrink pass later whose
            // blit then clips their trailing border off. Probing at a fair
            // share instead means the measured size already matches (or
            // is very close to) the eventual post-shrink size.
            let mut known_basis_sum: u32 = 0;
            let mut auto_count: u16 = 0;
            for i in 0..count {
                let item_size = match direction {
                    Direction::Horizontal => styles[i].width.unwrap_or(styles[i].size),
                    Direction::Vertical => styles[i].height.unwrap_or(styles[i].size),
                };
                match item_size {
                    Size::Length(n) => {
                        basis[i] = n;
                        known_basis_sum += u32::from(n);
                    }
                    Size::Percent(p) => {
                        let n = ((u32::from(available) * u32::from(p)) / 100) as u16;
                        basis[i] = n;
                        known_basis_sum += u32::from(n);
                    }
                    Size::Fr(f) => {
                        if styles[i].grow.is_none() {
                            grow[i] = f as f32;
                        }
                    }
                    Size::Auto => {
                        is_auto[i] = true;
                        auto_count += 1;
                    }
                }
                if let Some(g) = styles[i].grow {
                    grow[i] = g;
                }
            }
            let fair_share = (u32::from(available).saturating_sub(known_basis_sum)
                / u32::from(auto_count.max(1)))
            .min(u32::from(u16::MAX)) as u16;

            // First pass: measure only `Auto` items — their content size
            // directly determines their basis. Probe rects always start at
            // (0, 0) so the measured scratch buffer origin is consistent
            // with `blit_measured`'s expectations.
            for i in 0..count {
                if !is_auto[i] {
                    continue;
                }
                let node = nodes[i].take().expect("node already taken");
                let probe = match direction {
                    Direction::Horizontal => Rect::new(0, 0, fair_share.max(1), cross_available),
                    Direction::Vertical => Rect::new(0, 0, cross_available, fair_share.max(1)),
                };
                let measured = measure_node(node, probe);
                basis[i] = match direction {
                    Direction::Horizontal => measured.content_width,
                    Direction::Vertical => measured.content_height,
                };
                premeasured[i] = Some(measured);
            }

            let total_basis: u32 = basis.iter().map(|&b| u32::from(b)).sum();
            let free_space =
                i64::from(available) - i64::from(total_basis.min(u32::from(u16::MAX)) as u16);

            let mut sizes = basis.clone();

            if free_space > 0 {
                // Grow phase: distribute leftover space proportionally by
                // each item's `Fr` weight. If nothing has a weight, sizes
                // stay at basis and leftover is handled by
                // justify_content below.
                let free_space = free_space as u16;
                let total_grow: f32 = grow.iter().sum();
                if total_grow > 0.0 {
                    let mut used_extra = 0u16;
                    for i in 0..count {
                        let share = grow[i] / total_grow;
                        let extra = (f32::from(free_space) * share).floor() as u16;
                        let extra = extra.min(free_space.saturating_sub(used_extra));
                        sizes[i] = sizes[i].saturating_add(extra);
                        used_extra = used_extra.saturating_add(extra);
                    }
                    let mut leftover = free_space.saturating_sub(used_extra);
                    for i in 0..count {
                        if leftover == 0 {
                            break;
                        }
                        if grow[i] > 0.0 {
                            sizes[i] = sizes[i].saturating_add(1);
                            leftover -= 1;
                        }
                    }
                }
            } else if free_space < 0 {
                // Shrink phase: over budget, reduce proportionally to
                // shrink * basis (CSS's actual weighting).
                let overflow = (-free_space) as u16;
                let total_shrink_weighted: f32 = styles
                    .iter()
                    .zip(basis.iter())
                    .map(|(s, &b)| s.shrink * f32::from(b))
                    .sum();
                if total_shrink_weighted > 0.0 {
                    let mut used_reduction = 0u16;
                    for i in 0..count {
                        let weight = styles[i].shrink * f32::from(basis[i]);
                        let share = weight / total_shrink_weighted;
                        let reduce = (f32::from(overflow) * share).floor() as u16;
                        let reduce = reduce
                            .min(basis[i])
                            .min(overflow.saturating_sub(used_reduction));
                        sizes[i] = basis[i].saturating_sub(reduce);
                        used_reduction = used_reduction.saturating_add(reduce);
                    }
                }
                // else: nothing can shrink — items overflow, matching
                // CSS's behavior when shrink is 0 everywhere.
            }

            // Second pass: any remaining item whose cross-axis alignment
            // is not `Stretch` needs its content measured to know how to
            // position it within its slot — but only *now*, once `sizes`
            // holds each item's real, final main-axis size (post
            // grow/shrink). Auto items were already measured above and
            // are skipped here via the `premeasured[i].is_some()` check.
            // Probe rects always start at (0, 0).
            for i in 0..count {
                if premeasured[i].is_some() {
                    continue;
                }
                let align = Style::resolve_align(&container_style, &styles[i]);
                if align == Align::Stretch {
                    continue;
                }
                let node = nodes[i].take().expect("node already taken");
                let main_probe = sizes[i].max(1);
                let probe = match direction {
                    Direction::Horizontal => Rect::new(0, 0, main_probe, cross_available),
                    Direction::Vertical => Rect::new(0, 0, cross_available, main_probe),
                };
                premeasured[i] = Some(measure_node(node, probe));
            }

            let used = sizes
                .iter()
                .map(|&s| u32::from(s))
                .sum::<u32>()
                .saturating_add(u32::from(total_gap))
                .min(u32::from(u16::MAX)) as u16;
            let full_available = match direction {
                Direction::Horizontal => area.width,
                Direction::Vertical => area.height,
            };
            let (leading, extra_gaps) =
                distribute(container_style.justify_content, full_available, used, count);

            let mut cursor = match direction {
                Direction::Horizontal => area.x.saturating_add(leading),
                Direction::Vertical => area.y.saturating_add(leading),
            };

            for i in 0..count {
                cursor = cursor.saturating_add(extra_gaps.get(i).copied().unwrap_or(0));
                let size = sizes[i];

                let item_area = match direction {
                    Direction::Horizontal => Rect::new(cursor, area.y, size, area.height),
                    Direction::Vertical => Rect::new(area.x, cursor, area.width, size),
                };

                let item_area = item_area.intersection(area);

                let align = Style::resolve_align(&container_style, &styles[i]);

                if let Some(measured) = premeasured[i].take() {
                    let target = align_cross(
                        item_area,
                        align,
                        direction,
                        measured.content_width,
                        measured.content_height,
                    )
                    .intersection(area);
                    blit_measured(&measured, target, buf);
                } else {
                    let node = nodes[i].take().expect("node already taken");
                    node.render(item_area, buf);
                }

                cursor = cursor.saturating_add(size).saturating_add(gap);
            }
        }

        for item in ignored {
            item.node.render(area, buf);
        }
    }
}
