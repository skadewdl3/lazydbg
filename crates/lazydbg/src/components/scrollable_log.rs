use ratatui::crossterm::event::KeyCode;
use reactatui::prelude::*;

/// A scrollable paragraph widget displaying a log of events.
/// Handles arrow keys for manual panning and mouse wheel events for scrolling.
#[component]
pub fn scrollable_log<'a>(is_active: bool) -> TuiNode<'a> {
    // scroll offset: (y, x)
    let scroll_offset = use_state_keyed("scroll_offset", || (0_usize, 0_usize));
    let log = use_state_keyed("log", Vec::<String>::new);

    if is_active {
        let keys = use_key();
        {
            let offset = scroll_offset.clone();
            keys.on(KeyCode::Down, move || {
                offset.with_mut(|o| o.0 = o.0.saturating_add(1))
            });
        }
        {
            let offset = scroll_offset.clone();
            keys.on(KeyCode::Up, move || {
                offset.with_mut(|o| o.0 = o.0.saturating_sub(1))
            });
        }
        {
            let offset = scroll_offset.clone();
            keys.on(KeyCode::Right, move || {
                offset.with_mut(|o| o.1 = o.1.saturating_add(1))
            });
        }
        {
            let offset = scroll_offset.clone();
            keys.on(KeyCode::Left, move || {
                offset.with_mut(|o| o.1 = o.1.saturating_sub(1))
            });
        }
    }

    let offset_val = scroll_offset.get();
    let scroll_y = offset_val.0 as u16;
    let scroll_x = offset_val.1 as u16;

    // Provide lots of dummy lines to test scrolling
    let mut all_lines = vec![
        "Welcome to lazydbg!".to_string(),
        "Press Tab / Shift+Tab to change focus.".to_string(),
        "Up/Down/Left/Right to scroll the log when it's active.".to_string(),
        "Use mouse wheel or trackpad to scroll horizontally and vertically.".to_string(),
        "-----------------------------------------".to_string(),
    ];
    let user_lines = log.with(|v| v.clone());
    for i in 0..50 {
        all_lines.push(format!("Dummy line {} with a reaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaally long trailing text so we can test horizontal scrolling properly without wrapping", i + 1));
    }
    all_lines.extend(user_lines);

    let lines_str = all_lines.join("\n");

    let offset_y_mut = scroll_offset.clone();
    let offset_x_mut = scroll_offset.clone();

    tui! {
        <Paragraph text={lines_str} scroll={(scroll_y, scroll_x)}
            on:scrolly={move |delta: i16| {
                offset_y_mut.with_mut(|o| {
                    if delta > 0 {
                        o.0 = o.0.saturating_add(delta as usize);
                    } else {
                        o.0 = o.0.saturating_sub((-delta) as usize);
                    }
                });
            }}
            on:scrollx={move |delta: i16| {
                offset_x_mut.with_mut(|o| {
                    if delta > 0 {
                        o.1 = o.1.saturating_add(delta as usize);
                    } else {
                        o.1 = o.1.saturating_sub((-delta) as usize);
                    }
                });
            }}
        />
    }
}
