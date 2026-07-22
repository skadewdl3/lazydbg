use ratatui::widgets::Paragraph;
use reactatui::{
    TuiNode, component,
    hooks::{use_global_or_default, use_global_with},
    tui,
};

#[component]
pub fn Keybinds<'a>() -> TuiNode<'a> {
    let active_pane_keybinds = use_global_with("pane-keybinds", || "Hi mom");
    let keybinds = active_pane_keybinds.get();

    tui! {
        <Paragraph::new(keybinds) />
    }
}
