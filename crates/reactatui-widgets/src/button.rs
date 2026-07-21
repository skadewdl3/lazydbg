use ratatui::widgets::{Block, Borders};
use reactatui::prelude::*;

/// A clickable and hoverable action button widget.
/// Emits `"clicked"` with its label as a payload when clicked.
#[component]
pub fn Button<'a>(label: &'a str) -> TuiNode<'a> {
    let hovered = use_state(|| false);
    let emit = use_emit::<String>("click");
    let click_label = label.to_string();

    let style = if hovered.get() {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::Cyan)
    };

    let hovered_in = hovered.clone();
    let hovered_out = hovered.clone();

    tui! {
        <Block::default
            title={label.to_string()}
            borders={Borders::ALL}
            style={style}
            on:click={move |_btn| {
                emit.emit(click_label.clone());
            }}
            on:mousein={move || {
                hovered_in.set(true);
            }}
            on:mouseout={move || {
                hovered_out.set(false);
            }}
        />
    }
}
