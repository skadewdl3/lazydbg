use ratatui::crossterm::event::KeyCode;
use reactatui::prelude::*;
use reactatui_widgets::*;

/// A scrollable paragraph widget displaying a log of events.
/// Handles arrow keys for manual panning and mouse wheel events for scrolling.
#[component]
pub fn ScrollableLog<'a>(is_active: bool) -> TuiNode<'a> {
    // scroll offset: (y, x)
    let scroll_offset: State<(u16, u16)> = use_global_or_default("scroll_offset");
    let log: State<Vec<String>> = use_global_or_default("log");

    let scroll_x = move |delta: i16| {
        scroll_offset.with_mut(|o| apply_scroll_delta(&mut o.1, delta));
    };

    let scroll_y = move |delta: i16| {
        scroll_offset.with_mut(|o| apply_scroll_delta(&mut o.0, delta));
    };

    if is_active {
        let keys = use_key();
        keys.on(KeyCode::Down, move || scroll_y(1));
        keys.on(KeyCode::Up, move || scroll_y(-1));
        keys.on(KeyCode::Right, move || scroll_x(1));
        keys.on(KeyCode::Left, move || scroll_x(-1));
    }

    let (scroll_y_offset, scroll_x_offset) = scroll_offset.get();

    // Provide lots of dummy lines to test scrolling
    let mut all_lines = vec![
        "Welcome to lazydbg!".to_string(),
        "Press Tab / Shift+Tab to change focus.".to_string(),
        "Up/Down/Left/Right to scroll the log when it's active.".to_string(),
        "Use mouse wheel or trackpad to scroll horizontally and vertically.".to_string(),
        "-----------------------------------------".to_string(),
    ];
    let user_lines = log.with(|v| v.clone());
    all_lines.extend(user_lines);
    for i in 0..50 {
        all_lines.push(format!("Dummy line {} with a reaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaally long trailing text so we can test horizontal scrolling properly without wrapping", i + 1));
    }

    let lines_str = all_lines.join("\n");

    tui! {
        <Paragraph text={lines_str} scroll={(scroll_y_offset, scroll_x_offset)}
            on:scrolly={move |delta: i16| scroll_y(delta)}
            on:scrollx={move |delta: i16| scroll_x(delta)}
        />
    }
}

fn apply_scroll_delta(offset: &mut u16, delta: i16) {
    if delta >= 0 {
        *offset = offset.saturating_add(delta as u16);
    } else {
        *offset = offset.saturating_sub(delta.unsigned_abs());
    }
}
