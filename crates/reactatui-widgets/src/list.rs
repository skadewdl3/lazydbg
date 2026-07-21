use ratatui::layout::Direction;
use ratatui::widgets::Paragraph;
use reactatui::prelude::*;

/// A vertical list component that accepts children using `#[children]`.
#[component]
pub fn List<'a>(#[children] children: Vec<TuiNode<'a>>) -> TuiNode<'a> {
    tui! {
        <Flex direction={Direction::Vertical}>
        {
            for child in children {
                tui! { child }
            }
        }
        </Flex>
    }
}

/// An individual list item component accepting a text string.
#[component]
pub fn ListItem<'a>(text: &'a str) -> TuiNode<'a> {
    tui! {
        <Paragraph text={text} />
    }
}
