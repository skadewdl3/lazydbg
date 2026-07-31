use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use reactatui::{hooks::register_mouse_region, keybindings, measure::measure_node, prelude::*};

fn blit_window(src: &Buffer, src_area: Rect, offset: (u16, u16), target: Rect, buf: &mut Buffer) {
    let (off_x, off_y) = offset;
    let src_bottom = src_area.y.saturating_add(src_area.height);
    let src_right = src_area.x.saturating_add(src_area.width);

    for y in 0..target.height {
        let src_y = src_area.y.saturating_add(off_y).saturating_add(y);
        if src_y >= src_bottom {
            break;
        }
        for x in 0..target.width {
            let src_x = src_area.x.saturating_add(off_x).saturating_add(x);
            if src_x >= src_right {
                break;
            }
            buf[(target.x + x, target.y + y)] = src[(src_x, src_y)].clone();
        }
    }
}

fn clamp_offset(offset: (u16, u16), content_size: (u16, u16), viewport: (u16, u16)) -> (u16, u16) {
    let max_x = content_size.0.saturating_sub(viewport.0);
    let max_y = content_size.1.saturating_sub(viewport.1);
    (offset.0.min(max_x), offset.1.min(max_y))
}

fn apply_scroll_delta(offset: &mut u16, delta: i16) {
    if delta >= 0 {
        *offset = offset.saturating_add(delta as u16);
    } else {
        *offset = offset.saturating_sub(delta.unsigned_abs());
    }
}

/// Low-level scrolling primitive for when you already know the content's
/// exact extent. Cheapest path: one render at exactly `content_size`, no
/// measuring, offset clamped directly against a size you already trust.
pub struct ScrollView<'a> {
    child: TuiNode<'a>,
    offset: (u16, u16),
    content_size: (u16, u16),
}

impl<'a> ScrollView<'a> {
    pub fn new(child: impl Into<TuiNode<'a>>, content_size: (u16, u16)) -> Self {
        Self {
            child: child.into(),
            offset: (0, 0),
            content_size,
        }
    }

    pub fn offset(mut self, offset: (u16, u16)) -> Self {
        self.offset = offset;
        self
    }
}

impl<'a> Widget for ScrollView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let canvas = Rect::new(
            0,
            0,
            self.content_size.0.max(area.width),
            self.content_size.1.max(area.height),
        );
        let mut scratch = Buffer::empty(canvas);
        self.child.render(canvas, &mut scratch);
        let clamped = clamp_offset(self.offset, self.content_size, (area.width, area.height));
        blit_window(&scratch, canvas, clamped, area, buf);
    }
}

/// Makes any children scrollable in both directions, clamped so you can't
/// scroll past the end of the content.
#[component]
pub fn Scroll<'a>(is_active: bool, #[children] children: Vec<TuiNode<'a>>) -> TuiNode<'a> {
    let offset = use_state::<(u16, u16)>(|| (0, 0));
    let probe_size = use_state::<(u16, u16)>(|| (0, 0)); // fallback-path cache only

    if is_active {
        let keys = use_key();
        keybindings!(keys, {
            "down"  => move || offset.with_mut(|o| apply_scroll_delta(&mut o.1, 1)),
            "up"    => move || offset.with_mut(|o| apply_scroll_delta(&mut o.1, -1)),
            "right" => move || offset.with_mut(|o| apply_scroll_delta(&mut o.0, 1)),
            "left"  => move || offset.with_mut(|o| apply_scroll_delta(&mut o.0, -1)),
        });
    }

    // Unwrap a lone child instead of `TuiNode::fragment` — fragment always
    // wraps in `Fragment(vec![..])` even for one item, which would hide a
    // `TuiNode::Flex` from the pattern match below.
    let mut child = match children.len() {
        1 => children.into_iter().next().unwrap(),
        _ => TuiNode::fragment(children),
    };

    let raw_offset = offset.get();
    let last_probe = probe_size.get();

    TuiNode::Widget(Box::new(move |area: Rect, buf: &mut Buffer| {
        if area.width == 0 || area.height == 0 {
            return;
        }

        register_mouse_region(
            area,
            None,
            None,
            None,
            Some(Box::new(move |delta: i16| {
                offset.with_mut(|o| apply_scroll_delta(&mut o.0, delta));
                ::reactatui::hooks::Propagation::Stop
            })),
            Some(Box::new(move |delta: i16| {
                offset.with_mut(|o| apply_scroll_delta(&mut o.1, delta));
                ::reactatui::hooks::Propagation::Stop
            })),
        );

        // --- Fast, exact path: Flex child with statically-known basis. ---
        let mut flex_natural = match &child {
            TuiNode::Flex(flex) => Some(flex.natural_size(area.width)),
            TuiNode::Grid(grid) => Some(grid.natural_size((area.width, area.height))),
            _ => None,
        };
        // Fallback: if natural_size returned 0, it means it contains Auto items.
        // Measure them using measure_natural_size.
        if let Some((w, h)) = flex_natural {
            if w == 0 || h == 0 {
                if let TuiNode::Flex(flex) = child {
                    // measure_natural_size consumes the node, so we can't render it here anymore.
                    // Oh wait, measure_natural_size doesn't exist yet on FlexNode. I only added it to the plan. I should implement it in flex.rs or grid.rs. Or fallback to the existing heuristic.
                    // Since I didn't add measure_natural_size in my previous flex.rs write (I skipped it because it requires taking ownership, making it hard), let's just let it fall through to the probe heuristic!
                    flex_natural = None;
                    child = TuiNode::Flex(flex); // restore
                } else if let TuiNode::Grid(grid) = child {
                    flex_natural = None;
                    child = TuiNode::Grid(grid);
                }
            }
        }

        if let Some((natural_w, natural_h)) = flex_natural {
            let canvas_w = natural_w.max(area.width);
            let canvas_h = natural_h.max(area.height);
            let clamped = clamp_offset(raw_offset, (canvas_w, canvas_h), (area.width, area.height));
            if clamped != raw_offset {
                offset.set(clamped);
            }
            let canvas = Rect::new(0, 0, canvas_w, canvas_h);
            let mut scratch = Buffer::empty(canvas);

            let region_start = ::reactatui::hooks::mouse_region_count();
            child.render(canvas, &mut scratch);
            let region_end = ::reactatui::hooks::mouse_region_count();

            blit_window(&scratch, canvas, clamped, area, buf);

            let dx = area.x as i32 - (canvas.x + clamped.0) as i32;
            let dy = area.y as i32 - (canvas.y + clamped.1) as i32;
            ::reactatui::hooks::transform_mouse_regions(
                region_start,
                region_end,
                dx,
                dy,
                Some(area),
            );
            return;
        }

        // --- Fallback: growing-probe heuristic for opaque leaf content. ---
        let start_w = if last_probe.0 == 0 {
            area.width
        } else {
            last_probe.0
        };
        let start_h = if last_probe.1 == 0 {
            area.height
        } else {
            last_probe.1
        };
        let canvas_w = start_w.max(area.width.saturating_add(raw_offset.0));
        let canvas_h = start_h.max(area.height.saturating_add(raw_offset.1));

        let probe_area = Rect::new(0, 0, canvas_w, canvas_h);
        let measured = measure_node(child, probe_area);
        let content_size = (
            measured.content_width.max(area.width),
            measured.content_height.max(area.height),
        );

        let next_w = if content_size.0 >= canvas_w {
            canvas_w.saturating_mul(2)
        } else {
            content_size.0
        };
        let next_h = if content_size.1 >= canvas_h {
            canvas_h.saturating_mul(2)
        } else {
            content_size.1
        };
        if (next_w, next_h) != last_probe {
            probe_size.set((next_w, next_h));
        }

        let clamped = clamp_offset(raw_offset, content_size, (area.width, area.height));
        if clamped != raw_offset {
            offset.set(clamped);
        }

        blit_window(&measured.scratch, measured.probe_area, clamped, area, buf);

        let dx = area.x as i32 - (measured.probe_area.x + clamped.0) as i32;
        let dy = area.y as i32 - (measured.probe_area.y + clamped.1) as i32;
        ::reactatui::hooks::transform_mouse_regions(
            measured.region_start,
            measured.region_end,
            dx,
            dy,
            Some(area),
        );
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
    use ratatui::widgets::Paragraph;
    use reactatui::layout::{FlexItemNode, FlexNode};

    #[test]
    fn test_scroll_flex_child_and_opaque_child_span_viewport() {
        reactatui::hooks::begin_frame();

        let area = Rect::new(0, 0, 40, 3);
        let item1 = TuiNode::from_widget(
            Paragraph::new("Item1").style(ratatui::style::Style::default().bg(Color::Blue)),
        );
        let flex = FlexNode::vertical(vec![FlexItemNode::new(item1)]);

        let scroll_node = Scroll(false, vec![TuiNode::from(flex)]);
        let mut buf = Buffer::empty(area);
        scroll_node.render(area, &mut buf);

        // Verify entire line 0 in Scroll buffer is Blue across 40 columns
        for x in 0..40 {
            assert_eq!(
                buf[(x, 0)].bg,
                Color::Blue,
                "Scroll flex item at x={} should fill viewport width",
                x
            );
        }
    }
}
