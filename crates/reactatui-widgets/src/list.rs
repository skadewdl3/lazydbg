use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Widget};
use reactatui::hooks::{register_mouse_region, state};
use reactatui::layout::Size;
use reactatui::measure::{Measured, blit_measured, measure_node};
use reactatui::prelude::*;

use crate::Scroll;

#[component]
pub fn List<'a>(#[prop] r#virtual: bool, #[children] children: Vec<TuiNode<'a>>) -> TuiNode<'a> {
    let items = flatten_items(children);

    if r#virtual {
        render_virtualized(items)
    } else {
        let flex = FlexNode::vertical(items.into_iter().map(FlexItemNode::new).collect::<Vec<_>>());

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
    let offset = state::<u16>(|| 0);
    let mouse_owner = reactatui::hooks::__current_component_id();

    let raw_offset = offset.get();
    let scroll_offset = offset.clone();

    TuiNode::Widget(Box::new(move |area: Rect, buf: &mut Buffer| {
        if area.width == 0 || area.height == 0 || items.is_empty() {
            return;
        }

        let _mouse_guard = register_mouse_region(
            mouse_owner,
            area,
            None,
            None,
            None,
            None,
            Some(Box::new(move |delta: i16| {
                scroll_offset.with_mut(|o| apply_scroll_delta(o, delta));
                ::reactatui::hooks::Propagation::Stop
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
        } else if let Some(measured) = &first_prerendered {
            ::reactatui::measure::cull_measured(measured);
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_list_non_virtualized_and_virtualized_items_fill_full_width() {
        let area = Rect::new(0, 0, 50, 4);

        // Styled item short text "Short" but background green across 50 cols
        let create_item = || {
            let p =
                Paragraph::new("Short").style(ratatui::style::Style::default().bg(Color::Green));
            TuiNode::from_widget(p).style(reactatui::layout::Style::default().size(Size::Length(1)))
        };

        // 1) Non-virtualized list
        let runtime = Runtime::new();
        let mut buf_non_virt = Buffer::empty(area);
        runtime.render_to_buffer(&mut buf_non_virt, area, || {
            List(false, vec![create_item(), create_item()])
        });

        // Every column in line 0 should be styled Green
        for x in 0..50 {
            assert_eq!(
                buf_non_virt[(x, 0)].bg,
                Color::Green,
                "Non-virtualized List item cell at x={} should be Green",
                x
            );
        }

        // 2) Virtualized list
        let mut buf_virt = Buffer::empty(area);
        runtime.render_to_buffer(&mut buf_virt, area, || {
            List(true, vec![create_item(), create_item()])
        });

        // Every column in line 0 should be styled Green
        for x in 0..50 {
            assert_eq!(
                buf_virt[(x, 0)].bg,
                Color::Green,
                "Virtualized List item cell at x={} should be Green",
                x
            );
        }
    }
}
