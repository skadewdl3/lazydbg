use ratatui::{
    buffer::Buffer,
    layout::{Direction, Rect},
    widgets::Widget,
};

use crate::layout::Padding;
use crate::node::TuiNode;

pub struct FlexNode<'a> {
    direction: Direction,
    gap: u16,
    padding: Padding,
    items: Vec<FlexItemNode<'a>>,
}

impl<'a> FlexNode<'a> {
    pub fn new(items: impl Into<Vec<FlexItemNode<'a>>>) -> Self {
        Self {
            direction: Direction::Vertical,
            gap: 0,
            padding: Padding::default(),
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
}

impl<'a> Widget for FlexNode<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = self.padding.apply(area);
        let count = self.items.len() as u16;
        if count == 0 || area.width == 0 || area.height == 0 {
            return;
        }

        let total_gap = self.gap.saturating_mul(count.saturating_sub(1));
        let available = match self.direction {
            Direction::Horizontal => area.width.saturating_sub(total_gap),
            Direction::Vertical => area.height.saturating_sub(total_gap),
        };
        let sizes = flex_sizes(&self.items, available);

        let mut cursor = match self.direction {
            Direction::Horizontal => area.x,
            Direction::Vertical => area.y,
        };

        for (item, size) in self.items.into_iter().zip(sizes) {
            let item_area = match self.direction {
                Direction::Horizontal => Rect::new(cursor, area.y, size, area.height),
                Direction::Vertical => Rect::new(area.x, cursor, area.width, size),
            };
            item.node.render(item_area, buf);
            cursor = cursor.saturating_add(size).saturating_add(self.gap);
        }
    }
}

pub struct FlexItemNode<'a> {
    flex: u16,
    pub(crate) min: u16,
    pub(crate) max: u16,
    node: TuiNode<'a>,
}

impl<'a> FlexItemNode<'a> {
    pub fn new(node: impl Into<TuiNode<'a>>) -> Self {
        Self {
            flex: 1,
            min: 0,
            max: u16::MAX,
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
