use ratatui::layout::Rect;

#[derive(Clone, Copy, Default)]
pub struct Padding {
    top: u16,
    right: u16,
    bottom: u16,
    left: u16,
}

impl Padding {
    pub(crate) fn apply(self, area: Rect) -> Rect {
        let horizontal = self.left.saturating_add(self.right);
        let vertical = self.top.saturating_add(self.bottom);
        Rect::new(
            area.x.saturating_add(self.left),
            area.y.saturating_add(self.top),
            area.width.saturating_sub(horizontal),
            area.height.saturating_sub(vertical),
        )
    }
}

impl From<u16> for Padding {
    fn from(value: u16) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

impl From<(u16, u16, u16, u16)> for Padding {
    fn from((top, right, bottom, left): (u16, u16, u16, u16)) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}
