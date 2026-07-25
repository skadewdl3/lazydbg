//! Support type for the `style!` macro: a single value carrying both a
//! `ratatui::style::Style` (colors/modifiers) and a `reactatui::layout::Style`
//! (flex/grid alignment) side by side.
//!
//! Consumers don't pick eagerly — whichever builder method a widget exposes
//! (`Block::style` wants colors, `FlexItemNode::style` wants layout),
//! `Into` resolves the matching half automatically as long as that method
//! accepts `impl Into<T>` rather than a concrete `T`.

/// Produced by the `style!` macro. See the module docs.
#[derive(Debug, Clone, Copy, Default)]
pub struct CombinedStyle {
    pub color: ratatui::style::Style,
    pub layout: crate::layout::Style,
}

impl CombinedStyle {
    /// Explicit accessor for contexts where `Into`'s target type can't be
    /// inferred (e.g. storing into a `let` with no type annotation).
    pub fn color(self) -> ratatui::style::Style {
        self.color
    }

    /// Explicit accessor, layout half.
    pub fn layout(self) -> crate::layout::Style {
        self.layout
    }
}

impl From<CombinedStyle> for ratatui::style::Style {
    fn from(c: CombinedStyle) -> Self {
        c.color
    }
}

impl From<CombinedStyle> for crate::layout::Style {
    fn from(c: CombinedStyle) -> Self {
        c.layout
    }
}
