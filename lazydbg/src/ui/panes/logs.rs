use reactatui::{keybindings, prelude::*};
use reactatui_widgets::*;

use crate::app_state::AppState;

/// A scrollable paragraph widget displaying a log of events.
/// Handles arrow keys for manual panning and mouse wheel events for scrolling.
#[component]
pub fn Logs<'a>(#[prop] is_active: bool) -> TuiNode<'a> {
    // scroll offset: (y, x)
    let app = resource::<AppState>();
    let scroll_offset = app.log_scroll.clone();

    if is_active {
        let down = scroll_offset.clone();
        let up = scroll_offset.clone();
        let left = scroll_offset.clone();
        let right = scroll_offset.clone();
        keybindings!(use_key(), {
            "down" => move || down.with_mut(|o| apply_scroll_delta(&mut o.0, 1)),
            "up" => move || up.with_mut(|o| apply_scroll_delta(&mut o.0, -1)),
            "left" => move || left.with_mut(|o| apply_scroll_delta(&mut o.1, 1)),
            "right" => move || right.with_mut(|o| apply_scroll_delta(&mut o.1, -1))
        });
    }

    let (scroll_y_offset, scroll_x_offset) = scroll_offset.get();

    // Provide lots of dummy lines to test scrolling
    let lines_str = app.logs.snapshot().join("'\n");
    let mouse_y = scroll_offset.clone();
    let mouse_x = scroll_offset.clone();
    let block_style = style! { borders: all; };

    tui! {
        <Block::default borders={&block_style} title={"Logs"}>
            <Paragraph::new(lines_str) scroll={(scroll_y_offset, scroll_x_offset)}
                on:scrolly={move |delta: i16| mouse_y.with_mut(|o| apply_scroll_delta(&mut o.0, delta))}
                on:scrollx={move |delta: i16| mouse_x.with_mut(|o| apply_scroll_delta(&mut o.1, delta))}
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
