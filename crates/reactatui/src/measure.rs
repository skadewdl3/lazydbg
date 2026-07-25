//! Shared "measure this node's tight content extent" primitive. Used by
//! `Dialog` (auto-fit sizing), `Flex` (auto tracks + cross-axis alignment),
//! and `Grid` (auto tracks + alignment).

use ratatui::{buffer::Buffer, layout::Rect};

use crate::node::TuiNode;

/// The tight, non-blank content extent of a node rendered into `probe_area`,
/// plus the scratch buffer it was rendered into. Callers blit from this
/// buffer into the node's real final position rather than invoking the
/// node a second time (`TuiNode::Widget` is `FnOnce`, so it can only be
/// rendered once).
pub struct Measured<'a> {
    pub scratch: Buffer,
    pub probe_area: Rect,
    pub content_width: u16,
    pub content_height: u16,
    _marker: std::marker::PhantomData<&'a ()>,
}

/// Render `node` into a scratch buffer covering `probe_area` and return the
/// tight bounding box of its non-blank cells alongside the buffer.
pub fn measure_node<'a>(node: TuiNode<'a>, probe_area: Rect) -> Measured<'a> {
    use ratatui::widgets::Widget;

    let mut scratch = Buffer::empty(probe_area);
    node.render(probe_area, &mut scratch);

    let mut min_x = probe_area.x + probe_area.width;
    let mut min_y = probe_area.y + probe_area.height;
    let mut max_x = probe_area.x;
    let mut max_y = probe_area.y;
    let mut has_content = false;

    for y in probe_area.y..probe_area.y + probe_area.height {
        for x in probe_area.x..probe_area.x + probe_area.width {
            if scratch[(x, y)].symbol() != " " {
                has_content = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    let (content_width, content_height) = if has_content {
        (max_x - min_x + 1, max_y - min_y + 1)
    } else {
        (0, 0)
    };

    Measured {
        scratch,
        probe_area,
        content_width,
        content_height,
        _marker: std::marker::PhantomData,
    }
}

/// Blit a measured node's content into `target`, clipped to whichever of
/// `target`/measured content is smaller. `target` should already be
/// positioned/sized the way you want the content to land (e.g. via
/// `layout::style::align_rect`) — this function does no alignment itself.
pub fn blit_measured(measured: &Measured<'_>, target: Rect, buf: &mut Buffer) {
    let copy_w = target.width.min(measured.content_width);
    let copy_h = target.height.min(measured.content_height);
    let (src_x, src_y) = (measured.probe_area.x, measured.probe_area.y);

    for y in 0..copy_h {
        for x in 0..copy_w {
            let src = &measured.scratch[(src_x + x, src_y + y)];
            buf[(target.x + x, target.y + y)] = src.clone();
        }
    }
}
