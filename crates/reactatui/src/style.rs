//! Support type for the `style!` macro: a single value carrying both a
//! `ratatui::style::Style` (colors/modifiers) and a `reactatui::layout::Style`
//! (flex/grid alignment) side by side.
//!
//! Consumers don't pick eagerly — whichever builder method a widget exposes
//! (`Block::style` wants colors, `FlexItemNode::style` wants layout),
//! `Into` resolves the matching half automatically as long as that method
//! accepts `impl Into<T>` rather than a concrete `T`.

/// Produced by the `style!` macro. See the module docs.
#[derive(Debug, Clone, Default)]
pub struct CombinedStyle {
    pub base: ratatui::style::Style,
    pub reactatui: crate::layout::Style,
    pub border_type: Option<ratatui::widgets::BorderType>,
}

impl CombinedStyle {
    /// Explicit accessor for contexts where `Into`'s target type can't be
    /// inferred (e.g. storing into a `let` with no type annotation).
    pub fn base(self) -> ratatui::style::Style {
        self.base
    }

    /// Explicit accessor, layout half.
    pub fn reactatui(self) -> crate::layout::Style {
        self.reactatui
    }

    pub fn border_type(&self) -> ratatui::widgets::BorderType {
        self.border_type
            .unwrap_or(ratatui::widgets::BorderType::Plain)
    }

    pub fn split(self) -> (ratatui::style::Style, crate::layout::Style) {
        (self.base, self.reactatui)
    }
}

impl From<CombinedStyle> for ratatui::style::Style {
    fn from(c: CombinedStyle) -> Self {
        c.base
    }
}

impl From<CombinedStyle> for crate::layout::Style {
    fn from(c: CombinedStyle) -> Self {
        c.reactatui
    }
}

impl From<CombinedStyle> for ratatui::widgets::BorderType {
    fn from(c: CombinedStyle) -> Self {
        c.border_type.unwrap_or(ratatui::widgets::BorderType::Plain)
    }
}
