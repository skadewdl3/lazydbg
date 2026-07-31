use ratatui::widgets::Paragraph;
use reactatui::{TuiNode, component, style, tui};
use reactatui_widgets::Block;

use crate::ui::panes::Pane;

#[component]
pub fn Disassembly<'a>() -> TuiNode<'a> {
    let block_style = style! { borders: all; };
    tui! {
        <Block::default title={"Disassembly"} borders={&block_style}>
            if Pane::Disassembly.is_active() {
                <Paragraph::new("Active!") />
            }
            <Paragraph::new("Disassembly Pane") />
        </Block>
    }
}
