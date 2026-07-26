use ratatui::buffer::Buffer;
use ratatui::layout::{Direction, Rect};
use ratatui::widgets::{Paragraph, Widget};
use reactatui::hooks::{register_mouse_region, use_key, use_state};
use reactatui::keybindings;
use reactatui::measure::{blit_measured, measure_node};
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
        let mut iter = items.into_iter();
        let first_item = iter.next().expect("checked non-empty above");
        let rest: Vec<TuiNode<'a>> = iter.collect(); // original items[1..]

        // Measure the first child once to get a uniform row height —
        // this consumes it (TuiNode::Widget is FnOnce), so we keep the
        // rendered scratch buffer around to blit from if it's in view,
        // rather than trying to `.render()` it a second time.
        let probe_area = Rect::new(area.x, area.y, area.width, area.height);
        let measured_first = measure_node(first_item, probe_area);
        let item_height = measured_first.content_height.max(1);

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
            blit_measured(
                &measured_first,
                Rect::new(area.x, row, area.width, height),
                buf,
            );
            row = row.saturating_add(height);
        }

        // `rest` is items[1..], so skip (first_index - 1) of it to reach
        // the same absolute index `first_index` continues from.
        let skip = first_index.saturating_sub(1);
        for item in rest.into_iter().skip(skip) {
            if row >= bottom {
                break;
            }
            let height = item_height.min(bottom.saturating_sub(row));
            item.render(Rect::new(area.x, row, area.width, height), buf);
            row = row.saturating_add(height);
        }
    }))
}
