use ratatui::crossterm::event::KeyCode;
use reactatui::prelude::*;

/// A message input box widget.
/// Emits `"submitted"` when the user presses Enter with non-empty text,
/// and `"quit"` when the user presses Esc.
#[component]
pub fn MessageInput<'a>(is_active: bool) -> TuiNode<'a> {
    let state = use_state_keyed("msg_input", InputState::default);

    if is_active {
        let keys = use_key();
        let emit_submit = use_emit::<String>("submit");
        let emit_quit = use_emit::<()>("quit");

        keys.on(KeyCode::Enter, move || {
            let value = state.with(|s| s.value().to_string());
            if !value.is_empty() {
                emit_submit.emit(value);
                state.with_mut(|s| *s = InputState::default());
            }
        });

        keys.on(KeyCode::Esc, move || emit_quit.emit(()));

        keys.on_any(move |event| {
            // don't consume tab/backtab so focus can shift
            if event.code != KeyCode::Tab && event.code != KeyCode::BackTab {
                state.with_mut(|s| s.handle_key(event));
            }
        });
    }

    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    tui! {
        <Input
            placeholder={"Type a message — Enter to send, Esc to quit"}
            state={state}
            style={border_style}
        />
    }
}
