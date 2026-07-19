use reactatui::prelude::*;

/// A clickable and hoverable action button widget.
/// Emits `event_name` with its label as a payload when clicked.
#[component]
pub fn action_button<'a>(label: &'a str, event_name: &'static str) -> TuiNode<'a> {
    let hovered = use_state(|| false);
    let emit = use_emit::<String>(event_name);

    let style = if hovered.get() {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::Cyan)
    };

    let label_owned = label.to_string();
    let hovered_in = hovered.clone();
    let hovered_out = hovered.clone();

    tui! {
        <Block::default
            title={label}
            borders={Borders::ALL}
            style={style}
            on:click={move |_btn| {
                emit.emit(label_owned.clone());
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
