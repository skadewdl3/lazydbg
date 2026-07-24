use ratatui::crossterm::event::KeyCode;
use reactatui::{keybindings, prelude::*};
use reactatui_widgets::*;

use crate::logger::SharedLogStore;

/// A scrollable paragraph widget displaying a log of events.
/// Handles arrow keys for manual panning and mouse wheel events for scrolling.
#[component]
pub fn Logs<'a>(is_active: bool) -> TuiNode<'a> {
    // scroll offset: (y, x)
    let scroll_offset: State<(u16, u16)> = use_global_or_default("scroll-offset");
    let logs: State<SharedLogStore> = use_global("logs");

    let scroll_x = move |delta: i16| {
        scroll_offset.with_mut(|o| apply_scroll_delta(&mut o.1, delta));
    };

    let scroll_y = move |delta: i16| {
        scroll_offset.with_mut(|o| apply_scroll_delta(&mut o.0, delta));
    };

    if is_active {
        keybindings!(use_key(), {
            "down" => move || scroll_y(1),
            "up" => move || scroll_y(-1),
            "left" => move || scroll_x(1),
            "right" => move || scroll_x(-1)
        });
    }

    let (scroll_y_offset, scroll_x_offset) = scroll_offset.get();

    // Provide lots of dummy lines to test scrolling
    let lines_str = logs.get().snapshot().join("'\n");

    tui! {
        <Block::default borders={Borders::ALL} title={"Logs"}>
            <Paragraph::new(lines_str) scroll={(scroll_y_offset, scroll_x_offset)}
                on:scrolly={move |delta: i16| scroll_y(delta)}
                on:scrollx={move |delta: i16| scroll_x(delta)}
            />
        </Block>
    }
}

fn apply_scroll_delta(offset: &mut u16, delta: i16) {
    if delta >= 0 {
        *offset = offset.saturating_add(delta as u16);
    } else {
        *offset = offset.saturating_sub(delta.unsigned_abs());
    }
}
