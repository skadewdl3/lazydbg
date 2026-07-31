// crates/reactatui-widgets/src/button.rs
use ratatui::{
    layout::Alignment,
    widgets::{Borders, Paragraph},
};
use reactatui::{keybindings, prelude::*};

use crate::Block;

/// A clickable, hoverable, keyboard-activatable button widget.
///
/// Calls `on:click` when activated by a left mouse click, or by Enter/Space
/// while focused. Disabled buttons render dimmed and do not call it.
///
/// ```ignore
/// <Button
///     label={"Save"}
///     borders={style!{ borders: all; }}
///     disabled={false}
///     focused={true}
///     on:click={move || save()}
/// />
/// ```
#[component]
pub fn Button<'a>(
    #[prop] label: &'a str,
    #[prop] borders: impl Into<Borders>,
    #[prop] disabled: bool,
    #[prop] focused: bool,
    #[prop] on_click: Action,
) -> TuiNode<'a> {
    let hovered = state(|| false);
    let hovered_in = hovered.clone();
    let hovered_out = hovered.clone();
    let mouse_click = on_click.clone();
    focus(focused);
    if focused && !disabled {
        let keyboard_click = on_click.clone();
        keybindings! {
            "enter" | "space" => move || keyboard_click.call(),
        }
    }

    let appearance = style! {
        if disabled {
            text-style: dim;
        } else if hovered.get() || focused {
            text-style: reversed;
        }
    };
    let button = TuiNode::from_widget(
        Block::default()
            .borders(borders)
            .style(&appearance)
            .children(vec![TuiNode::from_widget(
                Paragraph::new(label).alignment(Alignment::Center),
            )]),
    );

    tui! {
        <{button}
            on:click={move |btn| {
                if !disabled && btn == ratatui::crossterm::event::MouseButton::Left {
                    mouse_click.call();
                }
            }}
            on:mousein={move || {
                if !disabled {
                    hovered_in.set(true);
                }
            }}
            on:mouseout={move || {
                hovered_out.set(false);
            }}
        />
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use ratatui::buffer::Buffer;
    use ratatui::crossterm::event::{
        Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use ratatui::layout::Rect;

    use super::*;

    #[test]
    fn focused_button_activates_from_keyboard_and_mouse() {
        let runtime = Runtime::new();
        let calls = Rc::new(Cell::new(0));
        let area = Rect::new(0, 0, 10, 3);
        let mut buffer = Buffer::empty(area);
        runtime.render_to_buffer(&mut buffer, area, {
            let calls = calls.clone();
            move || {
                Button(
                    "Save",
                    style! { borders: all; },
                    false,
                    true,
                    Action::from(move || calls.set(calls.get() + 1)),
                )
            }
        });

        runtime.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        runtime.handle_event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        }));
        assert_eq!(calls.get(), 2);
    }
}
