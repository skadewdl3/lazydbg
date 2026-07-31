use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

/// A one-frame render value. Unlike `TuiNode`, implementations can retain
/// their concrete widget type and avoid heap allocation and dynamic dispatch.
pub trait View {
    fn render(self, area: Rect, buffer: &mut Buffer);
}

pub struct WidgetView<W>(W);

pub fn view<W: Widget>(widget: W) -> WidgetView<W> {
    WidgetView(widget)
}

impl<W: Widget> View for WidgetView<W> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        self.0.render(area, buffer);
    }
}

trait ErasedView {
    fn render_box(self: Box<Self>, area: Rect, buffer: &mut Buffer);
}

impl<V: View> ErasedView for V {
    fn render_box(self: Box<Self>, area: Rect, buffer: &mut Buffer) {
        (*self).render(area, buffer);
    }
}

/// Explicit type-erasure for heterogeneous runtime-selected views.
pub struct AnyView<'a>(Box<dyn ErasedView + 'a>);

impl<'a> AnyView<'a> {
    pub fn new(view: impl View + 'a) -> Self {
        Self(Box::new(view))
    }
}

impl View for AnyView<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        self.0.render_box(area, buffer);
    }
}

pub(crate) struct ViewWidget<V>(pub V);

impl<V: View> Widget for ViewWidget<V> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        self.0.render(area, buffer);
    }
}
