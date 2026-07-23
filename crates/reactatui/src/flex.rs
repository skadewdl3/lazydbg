use ratatui::{
    buffer::Buffer,
    layout::{Direction, Rect},
    widgets::Widget,
};

use crate::layout::Padding;
use crate::node::TuiNode;

#[derive(Debug, Clone, Copy)]
pub(crate) enum LayoutUnit {
    Length(u16),
    Percent(u16),
    Fr(u16),
}

fn parse_layout_unit(s: &str) -> LayoutUnit {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        LayoutUnit::Percent(pct.trim().parse().unwrap_or(0))
    } else if let Some(fr) = s.strip_suffix("fr") {
        LayoutUnit::Fr(fr.trim().parse().unwrap_or(1).max(1))
    } else {
        LayoutUnit::Length(s.trim().parse().unwrap_or(0))
    }
}

fn parse_layout(spec: &str) -> Vec<LayoutUnit> {
    spec.split(',').map(parse_layout_unit).collect()
}

pub struct FlexNode<'a> {
    direction: Direction,
    gap: u16,
    padding: Padding,
    layout: Option<Vec<LayoutUnit>>,
    items: Vec<FlexItemNode<'a>>,
}

impl<'a> FlexNode<'a> {
    pub fn new(items: impl Into<Vec<FlexItemNode<'a>>>) -> Self {
        Self {
            direction: Direction::Vertical,
            gap: 0,
            padding: Padding::default(),
            layout: None,
            items: items.into(),
        }
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }
    pub fn layout(mut self, spec: impl AsRef<str>) -> Self {
        self.layout = Some(parse_layout(spec.as_ref()));
        self
    }
}

impl<'a> Widget for FlexNode<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (ignored, participating): (Vec<_>, Vec<_>) =
            self.items.into_iter().partition(|item| item.ignore);

        let count = participating.len() as u16;
        if count != 0 && area.width != 0 && area.height != 0 {
            let total_gap = self.gap.saturating_mul(count.saturating_sub(1));
            let available = match self.direction {
                Direction::Horizontal => area.width.saturating_sub(total_gap),
                Direction::Vertical => area.height.saturating_sub(total_gap),
            };
            let sizes = match &self.layout {
                Some(units) => layout_sizes(units, available),
                None => flex_sizes(&participating, available),
            };

            let mut cursor = match self.direction {
                Direction::Horizontal => area.x,
                Direction::Vertical => area.y,
            };

            for (item, size) in participating.into_iter().zip(sizes) {
                let item_area = match self.direction {
                    Direction::Horizontal => Rect::new(cursor, area.y, size, area.height),
                    Direction::Vertical => Rect::new(area.x, cursor, area.width, size),
                };
                item.node.render(item_area, buf);
                cursor = cursor.saturating_add(size).saturating_add(self.gap);
            }
        }

        // Ignored items render last (on top), full padded area — they position
        // themselves (e.g. a dialog centering within the given area).
        for item in ignored {
            item.node.render(area, buf);
        }
    }
}

pub struct FlexItemNode<'a> {
    flex: u16,
    pub(crate) min: u16,
    pub(crate) max: u16,
    pub(crate) ignore: bool,
    node: TuiNode<'a>,
}

impl<'a> FlexItemNode<'a> {
    pub fn new(node: impl Into<TuiNode<'a>>) -> Self {
        Self {
            flex: 1,
            min: 0,
            max: u16::MAX,
            ignore: false,
            node: node.into(),
        }
    }

    pub fn flex(mut self, flex: u16) -> Self {
        self.flex = flex.max(1);
        self
    }

    pub fn min(mut self, min: u16) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: u16) -> Self {
        self.max = max;
        self
    }

    pub fn flex_ignore(mut self) -> Self {
        self.ignore = true;
        self
    }
}

pub(crate) fn layout_sizes(units: &[LayoutUnit], available: u16) -> Vec<u16> {
    let mut sizes = vec![0u16; units.len()];
    let mut used = 0u16;

    // Pass 1: fixed lengths and percentages consume space first.
    let mut fr_total: u32 = 0;
    for (i, unit) in units.iter().enumerate() {
        let size = match unit {
            LayoutUnit::Length(n) => *n,
            LayoutUnit::Percent(p) => ((u32::from(available) * u32::from(*p)) / 100) as u16,
            LayoutUnit::Fr(f) => {
                fr_total += u32::from(*f);
                0
            }
        };
        let clamped = size.min(available.saturating_sub(used));
        sizes[i] = clamped;
        used = used.saturating_add(clamped);
    }

    // Pass 2: whatever's left is divided among `fr` units proportionally.
    if fr_total > 0 {
        let remaining = available.saturating_sub(used);
        let mut fr_used = 0u16;
        for (i, unit) in units.iter().enumerate() {
            if let LayoutUnit::Fr(f) = unit {
                let raw = (u32::from(remaining) * u32::from(*f)) / fr_total;
                let size = (raw as u16).min(remaining.saturating_sub(fr_used));
                sizes[i] = size;
                fr_used = fr_used.saturating_add(size);
            }
        }
        // Hand out leftover rounding units one at a time to `fr` tracks.
        let mut leftover = remaining.saturating_sub(fr_used);
        for (i, unit) in units.iter().enumerate() {
            if leftover == 0 {
                break;
            }
            if matches!(unit, LayoutUnit::Fr(_)) {
                sizes[i] = sizes[i].saturating_add(1);
                leftover -= 1;
            }
        }
    }

    sizes
}

pub(crate) fn flex_sizes(items: &[FlexItemNode<'_>], available: u16) -> Vec<u16> {
    let total_flex: u32 = items.iter().map(|item| u32::from(item.flex.max(1))).sum();
    if total_flex == 0 {
        return vec![0; items.len()];
    }

    let mut used = 0u16;
    let mut sizes = Vec::with_capacity(items.len());
    for item in items {
        let raw = (u32::from(available) * u32::from(item.flex.max(1))) / total_flex;
        let size = (raw as u16)
            .clamp(item.min, item.max)
            .min(available.saturating_sub(used));
        used = used.saturating_add(size);
        sizes.push(size);
    }

    let mut remaining = available.saturating_sub(used);
    for size in &mut sizes {
        if remaining == 0 {
            break;
        }
        *size = size.saturating_add(1);
        remaining -= 1;
    }

    sizes
}
