use ratatui::widgets::{Borders, Paragraph};
use reactatui::{TuiNode, component, tui};
use reactatui_widgets::Block;

use crate::ui::panes::Pane;

#[component]
pub fn Frame<'a>() -> TuiNode<'a> {
    tui! {
        <Block title={"Stack Frame"} borders={Borders::ALL}>
            if Pane::Frame.is_active() {
                <Paragraph::new("Active!") />
            }
            <Paragraph::new("Stack Frame Pane") />
        </Block>
    }
}
