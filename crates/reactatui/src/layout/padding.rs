use ratatui::layout::Rect;

#[derive(Clone, Copy, Default)]
#[allow(unused)]
pub struct Padding {
    top: u16,
    right: u16,
    bottom: u16,
    left: u16,
}

#[allow(unused)]
impl Padding {
    /// (top, right, bottom, left) — for internal use by natural-size
    /// computations that need to add padding back after computing the
    /// container's inner content extent.
    pub(crate) fn amounts(&self) -> (u16, u16, u16, u16) {
        (self.top, self.right, self.bottom, self.left)
    }

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
