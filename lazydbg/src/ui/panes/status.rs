use ratatui::widgets::{Borders, Paragraph};
use reactatui::{TuiNode, component, hooks::resource, tui};
use reactatui_widgets::Block;

use crate::app_state::AppState;

#[component]
pub fn Status<'a>() -> TuiNode<'a> {
    let session = resource::<AppState>().session.clone();
    let is_alive = session.with_mut_untracked(|s| s.is_alive());

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
