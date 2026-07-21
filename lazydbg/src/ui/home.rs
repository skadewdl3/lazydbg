use crate::{
    Args,
    interface::{DbgBackend, DbgSession, GdbBackend, LldbBackend},
};
use ratatui::{
    crossterm::event::KeyCode,
    widgets::{Borders, Paragraph},
};
use reactatui::prelude::*;
use reactatui_widgets::Block;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Panels {
    None,
    Variables,
    Stack,
}

/// The root UI component containing a title bar and the main multi-pane panel.
/// Responds to global Esc key to trigger application quit.
#[component]
pub fn Home<'a>() -> TuiNode<'a> {
    let should_quit = use_global_with("should_quit", || false);
    let active_panel = use_global_with("active-panel", move || Panels::None);
    let args = use_global("cli-args");
    let keys = use_key();

    let Args { lldb, .. } = args.get();
    let session = use_global_with("debugger-session", move || {
        let backend: Box<dyn DbgBackend> = if lldb {
            Box::new(LldbBackend::new())
        } else {
            Box::new(GdbBackend::new())
        };
        DbgSession::new(backend)
    });

    let session_is_alive = { session.with_mut(|s| s.is_alive()) };

    if active_panel.get() == Panels::None && !session_is_alive {
        keys.on(KeyCode::Esc, move || should_quit.set(true));
    }

    keys.on(KeyCode::Char('s'), move || {
        session.with_mut(|s| s.stop());
    });

    let stop = if session_is_alive {
        "s -> stop session"
    } else {
        "Esc -> exit"
    };
    let keybinds = format!("{}, o -> Open File, a -> set arguments, r -> run", stop);

    tui! {
        <Flex direction={Direction::Horizontal} gap={5}>
            <Block title_bottom={keybinds} borders={Borders::ALL}>
                <Paragraph::new("Layout demo — Tab/Shift+Tab to focus, Up/Down to scroll, Esc quits") />
            </Block>
            <Block borders={Borders::ALL}>
                <Paragraph::new("right") />
            </Block>
        </Flex>
    }
}
