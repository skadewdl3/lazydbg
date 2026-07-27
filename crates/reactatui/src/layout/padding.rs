use ratatui::layout::Rect;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Padding {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Padding {
    pub fn new(top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn symmetric(vertical: u16, horizontal: u16) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }

    pub fn all(value: u16) -> Self {
        Self::from(value)
    }

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

impl From<(u16, u16)> for Padding {
    fn from((vertical, horizontal): (u16, u16)) -> Self {
        Self::symmetric(vertical, horizontal)
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
