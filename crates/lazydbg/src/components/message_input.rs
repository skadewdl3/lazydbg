use ratatui::crossterm::event::KeyCode;
use reactatui::prelude::*;

/// A message input box widget.
/// Emits `"submitted"` when the user presses Enter with non-empty text,
/// and `"quit"` when the user presses Esc.
#[component]
pub fn message_input<'a>(is_active: bool) -> TuiNode<'a> {
    let state = use_state_keyed("msg_input", InputState::default);

    if is_active {
        let keys = use_key();
        let emit_submitted = use_emit::<String>("submitted");
        let emit_quit = use_emit::<()>("quit");

        {
            let state = state.clone();
            let emit_submitted = emit_submitted.clone();
            keys.on(KeyCode::Enter, move || {
                let value = state.with(|s| s.value().to_string());
                if !value.is_empty() {
                    emit_submitted.emit(value);
                    state.with_mut(|s| *s = InputState::default());
                }
            });
        }

        keys.on(KeyCode::Esc, move || emit_quit.emit(()));

        {
            let state = state.clone();
            keys.on_any(move |event| {
                // don't consume tab/backtab so focus can shift
                if event.code != KeyCode::Tab && event.code != KeyCode::BackTab {
                    state.with_mut(|s| s.handle_key(event));
                }
            });
        }
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
