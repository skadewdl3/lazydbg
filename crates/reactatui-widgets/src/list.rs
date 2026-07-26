use ratatui::buffer::Buffer;
use ratatui::layout::{Direction, Rect};
use ratatui::widgets::{Paragraph, Widget};
use reactatui::hooks::{register_mouse_region, use_key, use_state};
use reactatui::keybindings;
use reactatui::layout::Size;
use reactatui::measure::{Measured, blit_measured, measure_node};
use reactatui::prelude::*;

use crate::Scroll;

#[component]
pub fn List<'a>(r#virtual: bool, #[children] children: Vec<TuiNode<'a>>) -> TuiNode<'a> {
    let items = flatten_items(children);

    if r#virtual {
        render_virtualized(items)
    } else {
        let flex = FlexNode::new(items.into_iter().map(FlexItemNode::new).collect::<Vec<_>>())
            .direction(Direction::Vertical);

        // is_active=true: List is always scrollable by default.
        Scroll(false, vec![TuiNode::from(flex)])
    }
}

/// An individual list item component accepting a text string.
#[component]
pub fn ListItem<'a>(text: &'a str) -> TuiNode<'a> {
    tui! {
        <Paragraph::new(text) />
    }
}

/// Recursively flattens `Fragment` nodes (what a `for`/`if` inside `tui!`
/// expands to) into a flat run of items, so a looped-in collection of N
/// rows counts as N items — same as N literal `<ListItem>` children would.
/// `Empty` nodes (e.g. from a falsy `if` with no `else`) are dropped.
fn flatten_items<'a>(nodes: Vec<TuiNode<'a>>) -> Vec<TuiNode<'a>> {
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            TuiNode::Fragment(children) => out.extend(flatten_items(children)),
            TuiNode::Empty => {}
            other => out.push(other),
        }
    }
    out
}

fn apply_scroll_delta(offset: &mut u16, delta: i16) {
    if delta >= 0 {
        *offset = offset.saturating_add(delta as u16);
    } else {
        *offset = offset.saturating_sub(delta.unsigned_abs());
    }
}

/// Renders only the items inside the visible window. Scroll `offset` is
/// tracked in *items*, not rows — every row is assumed to be exactly as
/// tall as the first item, so no per-row-height bookkeeping is needed
/// beyond that one measurement.

fn render_virtualized<'a>(items: Vec<TuiNode<'a>>) -> TuiNode<'a> {
    let offset = use_state::<u16>(|| 0);
    let keys = use_key();

    keybindings!(keys, {
        "down" | "j" => move || offset.with_mut(|o| *o = o.saturating_add(1)),
        "up" | "k" => move || offset.with_mut(|o| *o = o.saturating_sub(1)),
    });

    let raw_offset = offset.get();

    TuiNode::Widget(Box::new(move |area: Rect, buf: &mut Buffer| {
        if area.width == 0 || area.height == 0 || items.is_empty() {
            return;
        }

        register_mouse_region(
            area,
            None,
            None,
            None,
            None,
            Some(Box::new(move |delta: i16| {
                offset.with_mut(|o| apply_scroll_delta(o, delta));
            })),
        );

        let item_count = items.len() as u16;
        let mut iter = items.into_iter().map(TuiNode::take_style);

        let (first_style, first_node) = iter.next().expect("checked non-empty above");
        let rest: Vec<(reactatui::layout::Style, TuiNode<'a>)> = iter.collect();
        let probe_area = Rect::new(area.x, area.y, area.width, area.height);

        // Explicit `size` on the first item *is* the row height for every
        // row — no measuring, matching what a non-virtual <Flex> with the
        // same style would do. `Percent` resolves against the viewport
        // height, same convention Flex's basis-resolution pass uses.
        // `Fr` has no "leftover space" to share in a virtualized,
        // partially off-screen list, so — like `Auto` — it falls back to
        // measuring the first row's content.
        let (item_height, mut first_node, first_prerendered): (
            u16,
            Option<TuiNode<'a>>,
            Option<Measured<'a>>,
        ) = match first_style.size {
            Size::Length(n) => (n.max(1), Some(first_node), None),
            Size::Percent(p) => (
                (((u32::from(area.height) * u32::from(p)) / 100) as u16).max(1),
                Some(first_node),
                None,
            ),
            Size::Auto | Size::Fr(_) => {
                let measured = measure_node(first_node, probe_area);
                (measured.content_height.max(1), None, Some(measured))
            }
        };

        let visible_items = (area.height / item_height).max(1);
        let max_offset = item_count.saturating_sub(visible_items);
        let clamped = raw_offset.min(max_offset);
        if clamped != raw_offset {
            offset.set(clamped);
        }

        let first_index = clamped as usize;
        let bottom = area.y.saturating_add(area.height);
        let mut row = area.y;

        if first_index == 0 && row < bottom {
            let height = item_height.min(bottom.saturating_sub(row));
            let target = Rect::new(area.x, row, area.width, height);
            if let Some(measured) = &first_prerendered {
                blit_measured(measured, target, buf);
            } else if let Some(node) = first_node.take() {
                node.render(target, buf);
            }
            row = row.saturating_add(height);
        }

        let skip = first_index.saturating_sub(1);
        for (_, item) in rest.into_iter().skip(skip) {
            if row >= bottom {
                break;
            }
            let height = item_height.min(bottom.saturating_sub(row));
            item.render(Rect::new(area.x, row, area.width, height), buf);
            row = row.saturating_add(height);
        }
    }))
}
