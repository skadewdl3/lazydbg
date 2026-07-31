use reactatui::{keybindings, prelude::*};
use reactatui_widgets::*;

use crate::app_state::{APP_STATE_KEY, AppState};

/// A scrollable paragraph widget displaying a log of events.
/// Handles arrow keys for manual panning and mouse wheel events for scrolling.
#[component]
pub fn Logs<'a>(#[prop] is_active: bool) -> TuiNode<'a> {
    // scroll offset: (y, x)
    let app = resource::<AppState>(APP_STATE_KEY);
    let scroll_offset = app.log_scroll.clone();

    if is_active {
        keybindings! {
            "down" => lambda!(+scroll_offset, || scroll_offset.with_mut(|o| apply_scroll_delta(&mut o.0, 1))),
            "up" => lambda!(+scroll_offset, || scroll_offset.with_mut(|o| apply_scroll_delta(&mut o.0, -1))),
            "left" => lambda!(+scroll_offset, || scroll_offset.with_mut(|o| apply_scroll_delta(&mut o.1, 1))),
            "right" => lambda!(+scroll_offset, || scroll_offset.with_mut(|o| apply_scroll_delta(&mut o.1, -1)))
        }
    }

    let (scroll_y_offset, scroll_x_offset) = scroll_offset.get();

    // Provide lots of dummy lines to test scrolling
    let lines_str = app.logs.snapshot().join("'\n");
    let block_style = style! { borders: all; };

    tui! {
        <Block::default borders={&block_style} title={"Logs"}>
            <Paragraph::new(lines_str) scroll={(scroll_y_offset, scroll_x_offset)}
                on:scrolly={lambda!(+scroll_offset, |delta: i16| scroll_offset.with_mut(|o| apply_scroll_delta(&mut o.0, delta)))}
                on:scrollx={lambda!(+scroll_offset, |delta: i16| scroll_offset.with_mut(|o| apply_scroll_delta(&mut o.1, delta)))}
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
