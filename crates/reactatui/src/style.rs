//! The universal value produced by [`style!`](crate::style!).
//!
//! [`ReactatuiStyle`] keeps ordinary Ratatui text styling together with typed
//! block configuration. Each builder callback selects the value it needs
//! through `Into<T>`.

use std::ops::Deref;

use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    symbols::{border, merge::MergeStrategy},
    widgets::{BorderType, Borders, Padding, Shadow, TitlePosition},
};

/// A CSS-like style value that can be interpreted by different Ratatui APIs.
///
/// Reuse one value by passing a reference to each callback. For example,
/// `block.style(&style).borders(&style).border_type(&style)`.
#[derive(Debug, Clone, Default)]
pub struct ReactatuiStyle<'a> {
    base: Style,
    block: Vec<BlockProperty<'a>>,
}

impl<'a> ReactatuiStyle<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Extract the ordinary Ratatui style.
    pub fn base(self) -> Style {
        self.base
    }

    #[doc(hidden)]
    pub fn base_style(mut self, style: impl Into<Style>) -> Self {
        self.base = style.into();
        self
    }

    #[doc(hidden)]
    pub fn fg(mut self, color: Color) -> Self {
        self.base = self.base.fg(color);
        self
    }

    #[doc(hidden)]
    pub fn bg(mut self, color: Color) -> Self {
        self.base = self.base.bg(color);
        self
    }

    #[doc(hidden)]
    pub fn underline_color(mut self, color: Color) -> Self {
        self.base = self.base.underline_color(color);
        self
    }

    #[doc(hidden)]
    pub fn add_modifier(mut self, modifier: Modifier) -> Self {
        self.base = self.base.add_modifier(modifier);
        self
    }

    #[doc(hidden)]
    pub fn remove_modifier(mut self, modifier: Modifier) -> Self {
        self.base = self.base.remove_modifier(modifier);
        self
    }

    #[doc(hidden)]
    pub fn patch(mut self, style: impl Into<Style>) -> Self {
        self.base = self.base.patch(style);
        self
    }
}

impl Deref for ReactatuiStyle<'_> {
    type Target = Style;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl From<ReactatuiStyle<'_>> for Style {
    fn from(style: ReactatuiStyle<'_>) -> Self {
        style.base
    }
}

impl From<&ReactatuiStyle<'_>> for Style {
    fn from(style: &ReactatuiStyle<'_>) -> Self {
        style.base.clone()
    }
}

// This registry is the runtime half of adding a block property. It generates
// the stored property, macro-facing setter, and owned/borrowed conversions for
// the concrete type accepted by its Ratatui callback.
macro_rules! block_properties {
    (
        conversions {
            $(
                $variant:ident($ty:ty) => $setter:ident;
            )*
        }
    ) => {
        #[derive(Debug, Clone)]
        enum BlockProperty<'a> {
            $(
                $variant($ty),
            )*
        }

        impl<'a> ReactatuiStyle<'a> {
            $(
                #[doc(hidden)]
                pub fn $setter(mut self, value: impl Into<$ty>) -> Self {
                    self.block.push(BlockProperty::$variant(value.into()));
                    self
                }
            )*
        }

        $(
            impl<'a> From<ReactatuiStyle<'a>> for $ty {
                fn from(style: ReactatuiStyle<'a>) -> Self {
                    (&style).into()
                }
            }

            impl<'a> From<&ReactatuiStyle<'a>> for $ty {
                fn from(style: &ReactatuiStyle<'a>) -> Self {
                    style.block.iter().rev().find_map(|property| match property {
                        BlockProperty::$variant(value) => Some(value.clone()),
                        _ => None,
                    }).unwrap_or_else(|| panic!(
                        "style! value does not configure `{}`",
                        stringify!($setter),
                    ))
                }
            }
        )*
    };
}

block_properties! {
    conversions {
        Borders(Borders) => borders;
        BorderType(BorderType) => border_type;
        BorderSet(border::Set<'a>) => border_set;
        TitleAlignment(Alignment) => title_alignment;
        TitlePosition(TitlePosition) => title_position;
        Padding(Padding) => padding;
        MergeBorders(MergeStrategy) => merge_borders;
        Shadow(Shadow) => shadow;
    }
}

/// Backwards-compatible name for code that used the earlier combined value.
pub type CombinedStyle<'a> = ReactatuiStyle<'a>;
