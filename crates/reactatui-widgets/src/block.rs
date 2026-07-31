use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    symbols::{border, merge::MergeStrategy},
    text::Line,
    widgets::{BorderType, Borders, Shadow, TitlePosition, Widget},
};
use reactatui::node::TuiNode;

/// A Block widget analogous to Ratatui's `Block`, capable of rendering child TUI nodes inside its borders.
pub struct Block<'a> {
    inner: ratatui::widgets::Block<'a>,
    children: Vec<TuiNode<'a>>,
}

impl<'a> Default for Block<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Block<'a> {
    pub fn new() -> Self {
        Self {
            inner: ratatui::widgets::Block::new(),
            children: Vec::new(),
        }
    }

    pub fn title<T>(mut self, title: T) -> Self
    where
        T: Into<Line<'a>>,
    {
        self.inner = self.inner.title(title);
        self
    }

    pub fn title_top<T>(mut self, title: T) -> Self
    where
        T: Into<Line<'a>>,
    {
        self.inner = self.inner.title_top(title);
        self
    }

    pub fn title_bottom<T>(mut self, title: T) -> Self
    where
        T: Into<Line<'a>>,
    {
        self.inner = self.inner.title_bottom(title);
        self
    }

    pub fn title_style(mut self, style: impl Into<Style>) -> Self {
        self.inner = self.inner.title_style(style);
        self
    }

    pub fn title_alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.inner = self.inner.title_alignment(alignment.into());
        self
    }

    pub fn title_position(mut self, position: impl Into<TitlePosition>) -> Self {
        self.inner = self.inner.title_position(position.into());
        self
    }

    pub fn borders(mut self, borders: impl Into<Borders>) -> Self {
        self.inner = self.inner.borders(borders.into());
        self
    }

    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.inner = self.inner.style(style.into());
        self
    }

    pub fn border_style(mut self, style: impl Into<Style>) -> Self {
        self.inner = self.inner.border_style(style.into());
        self
    }

    pub fn border_type(mut self, border_type: impl Into<BorderType>) -> Self {
        self.inner = self.inner.border_type(border_type.into());
        self
    }

    pub fn border_set(mut self, border_set: impl Into<border::Set<'a>>) -> Self {
        self.inner = self.inner.border_set(border_set.into());
        self
    }

    pub fn padding(mut self, padding: impl Into<ratatui::widgets::Padding>) -> Self {
        self.inner = self.inner.padding(padding.into());
        self
    }

    pub fn merge_borders(mut self, strategy: impl Into<MergeStrategy>) -> Self {
        self.inner = self.inner.merge_borders(strategy.into());
        self
    }

    pub fn shadow(mut self, shadow: impl Into<Shadow>) -> Self {
        self.inner = self.inner.shadow(shadow.into());
        self
    }

    pub fn children(mut self, children: impl Into<Vec<TuiNode<'a>>>) -> Self {
        self.children = children.into();
        self
    }

    /// Access the underlying `ratatui::widgets::Block`.
    pub fn inner_block(&self) -> &ratatui::widgets::Block<'a> {
        &self.inner
    }

    /// Convert into the underlying `ratatui::widgets::Block`.
    pub fn into_inner(self) -> ratatui::widgets::Block<'a> {
        self.inner
    }
}

impl<'a> From<Block<'a>> for ratatui::widgets::Block<'a> {
    fn from(block: Block<'a>) -> Self {
        block.inner
    }
}

impl<'a> Widget for Block<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner_area = self.inner.inner(area);
        self.inner.render(area, buf);
        if !self.children.is_empty() {
            TuiNode::fragment(self.children).render(inner_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{
        buffer::Buffer,
        layout::{Alignment, Rect},
        style::{Color, Style},
        symbols::{self, merge::MergeStrategy},
        widgets::{
            Block as RatatuiBlock, BorderType, Borders, Padding, Shadow, TitlePosition, Widget,
        },
    };
    use reactatui::{TuiNode, style, tui};

    use super::Block;

    #[test]
    fn reuses_one_style_for_every_block_callback() {
        let style = style! {
            color: cyan;
            background-color: black;
            borders: top left;
            border-type: rounded;
            border-set: {symbols::border::DOUBLE};
            title-alignment: center;
            title-position: bottom;
            padding: 1 2 3 4;
            merge-borders: fuzzy;
            shadow: dark-shade;
        };
        let actual = Block::new()
            .style(&style)
            .border_style(&style)
            .title_style(&style)
            .borders(&style)
            .border_type(&style)
            .border_set(&style)
            .title_alignment(&style)
            .title_position(&style)
            .padding(&style)
            .merge_borders(&style)
            .shadow(&style);
        let expected = RatatuiBlock::new()
            .style(Style::new().fg(Color::Cyan).bg(Color::Black))
            .borders(Borders::TOP | Borders::LEFT)
            .border_type(BorderType::Rounded)
            .border_set(symbols::border::DOUBLE)
            .border_style(Style::new().fg(Color::Cyan).bg(Color::Black))
            .title_style(Style::new().fg(Color::Cyan).bg(Color::Black))
            .title_alignment(Alignment::Center)
            .title_position(TitlePosition::Bottom)
            .padding(Padding::new(4, 2, 1, 3))
            .merge_borders(MergeStrategy::Fuzzy)
            .shadow(Shadow::dark_shade());

        assert_eq!(actual.inner_block(), &expected);
    }

    #[test]
    fn converts_style_values_for_explicit_block_props() {
        let style = style! {
            borders: top right;
            border-type: heavy-double-dashed;
            title-alignment: right;
            title-position: bottom;
            padding: 1 2;
            merge-borders: exact;
            shadow: light-shade;
        };
        let borders: Borders = (&style).into();
        let border_type: BorderType = (&style).into();
        let alignment: Alignment = (&style).into();
        let position: TitlePosition = (&style).into();
        let padding: Padding = (&style).into();
        let merging: MergeStrategy = (&style).into();
        let shadow: Shadow = (&style).into();

        assert_eq!(borders, Borders::TOP | Borders::RIGHT);
        assert_eq!(border_type, BorderType::HeavyDoubleDashed);
        assert_eq!(alignment, Alignment::Right);
        assert_eq!(position, TitlePosition::Bottom);
        assert_eq!(padding, Padding::symmetric(2, 1));
        assert_eq!(merging, MergeStrategy::Exact);
        assert_eq!(shadow, Shadow::light_shade());
    }

    #[test]
    fn supports_all_border_types_and_conditional_block_properties() {
        let cases = [
            (style! { border-type: plain; }, BorderType::Plain),
            (style! { border-type: rounded; }, BorderType::Rounded),
            (style! { border-type: double; }, BorderType::Double),
            (style! { border-type: thick; }, BorderType::Thick),
            (
                style! { border-type: light-double-dashed; },
                BorderType::LightDoubleDashed,
            ),
            (
                style! { border-type: heavy-double-dashed; },
                BorderType::HeavyDoubleDashed,
            ),
            (
                style! { border-type: light-triple-dashed; },
                BorderType::LightTripleDashed,
            ),
            (
                style! { border-type: heavy-triple-dashed; },
                BorderType::HeavyTripleDashed,
            ),
            (
                style! { border-type: light-quadruple-dashed; },
                BorderType::LightQuadrupleDashed,
            ),
            (
                style! { border-type: heavy-quadruple-dashed; },
                BorderType::HeavyQuadrupleDashed,
            ),
            (
                style! { border-type: quadrant-inside; },
                BorderType::QuadrantInside,
            ),
            (
                style! { border-type: quadrant-outside; },
                BorderType::QuadrantOutside,
            ),
        ];
        for (style, expected) in cases {
            assert_eq!(BorderType::from(style), expected);
        }

        let rounded = true;
        let conditional: BorderType = style! {
            border-type: if rounded { rounded } else { plain };
        }
        .into();
        assert_eq!(conditional, BorderType::Rounded);

        let merged: MergeStrategy = style! {
            if rounded {
                merge-borders: fuzzy;
            } else {
                merge-borders: replace;
            }
        }
        .into();
        assert_eq!(merged, MergeStrategy::Fuzzy);
    }

    #[test]
    #[should_panic(expected = "style! value does not configure `borders`")]
    fn mismatched_conversion_panics_clearly() {
        let _: Borders = style! { border-type: rounded; }.into();
    }

    #[test]
    fn jsx_props_borrow_the_same_style_value() {
        let style = style! {
            color: cyan;
            borders: all;
            border-type: rounded;
        };
        let node: TuiNode<'_> = tui! {
            <Block::default
                border_style={&style}
                title_style={&style}
                borders={&style}
                border_type={&style}
            />
        };
        let area = Rect::new(0, 0, 4, 3);
        let mut buffer = Buffer::empty(area);
        node.render(area, &mut buffer);

        assert_eq!(buffer[(0, 0)].symbol(), "╭");
        assert_eq!(buffer[(3, 2)].symbol(), "╯");
        assert_eq!(buffer[(0, 0)].fg, Color::Cyan);
    }
}
