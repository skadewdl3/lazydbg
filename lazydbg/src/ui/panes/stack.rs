use ratatui::widgets::{Borders, Paragraph};
use reactatui::{TuiNode, component, tui};
use reactatui_widgets::Block;

use crate::ui::panes::Pane;

#[component]
pub fn Stack<'a>() -> TuiNode<'a> {
    tui! {
        <Block::default title={"Stack"} borders={Borders::ALL}>
            if Pane::Stack.is_active() {
                <Paragraph::new("Active!") />
            }
            <Paragraph::new("Call Stack Pane") />
        </Block>
    }
}
