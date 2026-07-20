use crate::node::TuiNode;
use ratatui::layout::Rect;

pub trait FrameExt {
    fn render_node(&mut self, node: TuiNode<'_>, area: Rect);
}

impl FrameExt for ratatui::Frame<'_> {
    fn render_node(&mut self, node: TuiNode<'_>, area: Rect) {
        self.render_widget(node, area);
    }
}
