use ratatui::widgets::Paragraph;
use reactatui::{TuiNode, component, tui};

#[component]
pub fn Keybinds<'a>() -> TuiNode<'a> {
    let keybinds = "o -> select binary, r -> run, b -> set breakpoint, q -> quit";

    tui! {
        <Paragraph::new(keybinds) />
    }
}
