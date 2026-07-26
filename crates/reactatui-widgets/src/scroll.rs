use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};
use reactatui::layout::Size;
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
    let child = match children.len() {
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
            })),
            Some(Box::new(move |delta: i16| {
                offset.with_mut(|o| apply_scroll_delta(&mut o.1, delta));
            })),
        );

        // --- Fast, exact path: Flex child with statically-known basis. ---
        let flex_natural = match &child {
            TuiNode::Flex(flex) => Some(flex.natural_size(area.width)),
            TuiNode::Grid(grid) => Some(grid.natural_size((area.width, area.height))),
            _ => None,
        };
        if let Some((natural_w, natural_h)) = flex_natural {
            let canvas_w = natural_w.max(area.width);
            let canvas_h = natural_h.max(area.height);
            let clamped = clamp_offset(raw_offset, (canvas_w, canvas_h), (area.width, area.height));
            if clamped != raw_offset {
                offset.set(clamped);
            }
            let canvas = Rect::new(0, 0, canvas_w, canvas_h);
            let mut scratch = Buffer::empty(canvas);
            child.render(canvas, &mut scratch);
            blit_window(&scratch, canvas, clamped, area, buf);
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
        let content_size = (measured.content_width, measured.content_height);

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
    }))
}
