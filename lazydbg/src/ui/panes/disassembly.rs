use ratatui::widgets::{Borders, Paragraph};
use reactatui::{TuiNode, component, tui};
use reactatui_widgets::Block;

use crate::ui::panes::Pane;

#[component]
pub fn Disassembly<'a>() -> TuiNode<'a> {
    tui! {
        <Block::default title={"Disassembly"} borders={Borders::ALL}>
            if Pane::Disassembly.is_active() {
                <Paragraph::new("Active!") />
            }
            <Paragraph::new("Disassembly Pane") />
        </Block>
    }
}
