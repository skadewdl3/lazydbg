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
        let mut flex = Self::new(items);
        flex.direction = Direction::Horizontal;
        flex
    }

    pub fn vertical(items: impl Into<Vec<FlexItemNode<'a>>>) -> Self {
        Self::new(items)
    }

    /// Container-level: `justify_content` (main-axis distribution),
    /// `align_items` (default cross-axis alignment for children), `direction`, `gap`, `padding`.
    pub fn style(mut self, style: impl Into<Style>) -> Self {
        let style = style.into();
        if let Some(dir) = style.direction {
            self.direction = dir;
        }
        if let Some(gx) = style.gap_x
            && self.direction == Direction::Horizontal
        {
            self.gap = gx;
        }
        if let Some(gy) = style.gap_y
            && self.direction == Direction::Vertical
        {
            self.gap = gy;
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

    /// Returns exact geometry for layouts that do not require intrinsic
    /// measurement. An explicit `Auto` item returns `None`.
    pub fn natural_size(&self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        let participating = self.items.iter().filter(|item| !item.style.ignore);
        let count = participating.clone().count();
        let gap_total = self.gap.saturating_mul((count as u16).saturating_sub(1));
        let mut main: u32 = u32::from(gap_total);
        let main_available = match self.direction {
            Direction::Horizontal => viewport.0,
            Direction::Vertical => viewport.1,
        };

        for item in participating {
            let item_size = match self.direction {
                Direction::Horizontal => item.style.width.unwrap_or(item.style.size),
                Direction::Vertical => item.style.height.unwrap_or(item.style.size),
            };
            main += match item_size {
                Size::Length(n) => u32::from(n),
                Size::Percent(p) => (u32::from(main_available) * u32::from(p)) / 100,
                Size::Fr(_) => 0,
                Size::Auto => return None,
            };
        }

        let (pad_top, pad_right, pad_bottom, pad_left) = self.padding.amounts();
        let main_pad = match self.direction {
            Direction::Horizontal => pad_left.saturating_add(pad_right),
            Direction::Vertical => pad_top.saturating_add(pad_bottom),
        };
        let main = (main.saturating_add(u32::from(main_pad))).min(u32::from(u16::MAX)) as u16;

        Some(match self.direction {
            Direction::Horizontal => (main, viewport.1),
            Direction::Vertical => (viewport.0, main),
        })
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

    /// Item-level: `size` (main-axis sizing — `auto` measures content,
    /// `Length`/`Percent` pin it, `"Nfr"` grows to take a share of
    /// leftover space — this is the only way an item grows), and
    /// `align_self` (cross-axis override).
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
            struct Prepared<'a> {
                style: Style,
                node: Option<TuiNode<'a>>,
                measured: Option<Measured<'a>>,
                basis: u16,
                grow: u16,
                is_auto: bool,
            }

            let mut prepared: Vec<_> = participating
                .into_iter()
                .map(|item| Prepared {
                    style: item.style,
                    node: Some(item.node),
                    measured: None,
                    basis: 0,
                    grow: 0,
                    is_auto: false,
                })
                .collect();

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
            // - `Fr(f)` contributes zero basis but registers a grow weight.
            //   `Fr` is the only way an item grows; there is no implicit
            //   "Auto grows by default".
            // - `Auto` (or any item with non-Stretch cross-axis alignment)
            //   needs measuring before we know its basis.
            // Estimate a "fair share" of the remaining budget for Auto
            // items *before* measuring any of them. Widgets with no real
            // intrinsic size (e.g. bordered Blocks) always fill whatever
            // rect they're probed with — measuring them against the full
            // remaining budget makes every Auto item report a basis equal
            // to that whole budget and causes its trailing border to be
            // clipped. Probing at a fair share keeps the measured basis
            // close to the space the item can actually use.
            let mut known_basis_sum: u32 = 0;
            let mut auto_count: u16 = 0;
            for item in &mut prepared {
                let item_size = match direction {
                    Direction::Horizontal => item.style.width.unwrap_or(item.style.size),
                    Direction::Vertical => item.style.height.unwrap_or(item.style.size),
                };
                match item_size {
                    Size::Length(n) => {
                        item.basis = n;
                        known_basis_sum += u32::from(n);
                    }
                    Size::Percent(p) => {
                        let n = ((u32::from(available) * u32::from(p)) / 100) as u16;
                        item.basis = n;
                        known_basis_sum += u32::from(n);
                    }
                    Size::Fr(f) => item.grow = f,
                    Size::Auto => {
                        item.is_auto = true;
                        auto_count += 1;
                    }
                }
            }
            let fair_share = (u32::from(available).saturating_sub(known_basis_sum)
                / u32::from(auto_count.max(1)))
            .min(u32::from(u16::MAX)) as u16;

            // First pass: measure only `Auto` items — their content size
            // directly determines their basis. Probe rects always start at
            // (0, 0) so the measured scratch buffer origin is consistent
            // with `blit_measured`'s expectations.
            for item in &mut prepared {
                if !item.is_auto {
                    continue;
                }
                let node = item.node.take().expect("node already taken");
                let probe = match direction {
                    Direction::Horizontal => Rect::new(0, 0, fair_share.max(1), cross_available),
                    Direction::Vertical => Rect::new(0, 0, cross_available, fair_share.max(1)),
                };
                let measured = measure_node(node, probe);
                item.basis = match direction {
                    Direction::Horizontal => measured.content_width,
                    Direction::Vertical => measured.content_height,
                };
                item.measured = Some(measured);
            }

            let total_basis: u32 = prepared.iter().map(|item| u32::from(item.basis)).sum();
            let free_space =
                i64::from(available) - i64::from(total_basis.min(u32::from(u16::MAX)) as u16);

            let mut sizes: Vec<u16> = prepared.iter().map(|item| item.basis).collect();

            if free_space > 0 {
                // Grow phase: distribute leftover space proportionally by
                // each item's `Fr` weight. If nothing has a weight, sizes
                // stay at basis and leftover is handled by
                // justify_content below.
                let free_space = free_space as u16;
                let total_grow: u32 = prepared.iter().map(|item| u32::from(item.grow)).sum();
                if let Some(total_grow) = std::num::NonZeroU32::new(total_grow) {
                    let mut used_extra = 0u16;
                    for (i, item) in prepared.iter().enumerate() {
                        let extra = ((u32::from(free_space) * u32::from(item.grow))
                            / total_grow.get()) as u16;
                        let extra = extra.min(free_space.saturating_sub(used_extra));
                        sizes[i] = sizes[i].saturating_add(extra);
                        used_extra = used_extra.saturating_add(extra);
                    }
                    let mut leftover = free_space.saturating_sub(used_extra);
                    for i in 0..count {
                        if leftover == 0 {
                            break;
                        }
                        if prepared[i].grow > 0 {
                            sizes[i] = sizes[i].saturating_add(1);
                            leftover -= 1;
                        }
                    }
                }
            }

            // Second pass: any remaining item whose cross-axis alignment
            // is not `Stretch` needs its content measured to know how to
            // position it within its slot — but only *now*, once `sizes`
            // holds each item's real, final main-axis size. Auto items were already measured above and
            // are skipped here when the prepared item already has a measurement.
            // Probe rects always start at (0, 0).
            for i in 0..count {
                if prepared[i].measured.is_some() {
                    continue;
                }
                let align = Style::resolve_align(&container_style, &prepared[i].style);
                if align == Align::Stretch {
                    continue;
                }
                let node = prepared[i].node.take().expect("node already taken");
                let main_probe = sizes[i].max(1);
                let probe = match direction {
                    Direction::Horizontal => Rect::new(0, 0, main_probe, cross_available),
                    Direction::Vertical => Rect::new(0, 0, cross_available, main_probe),
                };
                prepared[i].measured = Some(measure_node(node, probe));
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

                let align = Style::resolve_align(&container_style, &prepared[i].style);

                if let Some(measured) = prepared[i].measured.take() {
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
                    let node = prepared[i].node.take().expect("node already taken");
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
