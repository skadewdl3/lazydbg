use ratatui::widgets::Paragraph;
use reactatui::{TuiNode, component, hooks::global_or, tui};

#[component]
pub fn Keybinds<'a>() -> TuiNode<'a> {
    let active_pane_keybinds = global_or(
        "pane-keybinds",
        || "o -> select binary, r -> run, b -> set breakpoint, q -> quit",
    );
    let keybinds = active_pane_keybinds.get();

    tui! {
        <Paragraph::new(keybinds) />
    }
}
