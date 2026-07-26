use ratatui::widgets::{Borders, Paragraph};
use reactatui::{
    TuiNode, component,
    hooks::{use_global, use_memo},
    style, tui,
};
use reactatui_widgets::{Block, Button};

use crate::interface::DbgSession;

#[component]
pub fn Status<'a>() -> TuiNode<'a> {
    let session = use_global::<DbgSession>("dbg-session");
    let is_alive = use_memo(session, |s| s.is_alive());

    tui! {
        <Block::default title={"Status"} borders={Borders::ALL}>
            if is_alive.get() {
                <Paragraph::new("Session is alive and well!") />
            } else {
                <Paragraph::new("Session is dead :(") />
            }
        </Block>
    }
}
