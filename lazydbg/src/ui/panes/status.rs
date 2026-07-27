use ratatui::widgets::{Borders, Paragraph};
use reactatui::{
    TuiNode, component,
    hooks::{use_global, use_memo},
    tui,
};
use reactatui_widgets::Block;

use crate::interface::DbgSession;

#[component]
pub fn Status<'a>() -> TuiNode<'a> {
    let session = use_global::<DbgSession>("dbg-session");
    let is_alive = session.with_mut(|s| s.is_alive());

    tui! {
        <Block::default title={"Status"} borders={Borders::ALL}>
            if is_alive {
                <Paragraph::new("Session is alive and well!") />
            } else {
                <Paragraph::new("Session is dead :(") />
            }
        </Block>
    }
}
