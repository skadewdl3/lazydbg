use crate::{
    Args,
    interface::{DbgBackend, DbgSession, GdbBackend, LldbBackend},
};
use ratatui::widgets::{Block, Borders, Paragraph};
use reactatui::{prelude::*, ratatui::crossterm::event::KeyCode};

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

    // Initialize the session once, if not already initialized
    let args = use_global("cli-args");
    let Args { lldb, .. } = args.get();
    let session = use_global_with("debugger-session", move || {
        let backend: Box<dyn DbgBackend> = if lldb {
            Box::new(LldbBackend::new())
        } else {
            Box::new(GdbBackend::new())
        };
        DbgSession::new(backend)
    });

    let keys = use_key();

    let session_is_alive = { session.with_mut(|s| s.is_alive()) };

    if active_panel.get() == Panels::None && !session_is_alive {
        keys.on(KeyCode::Esc, move || should_quit.set(true));
    }

    keys.on(KeyCode::Char('s'), move || {
        session.with_mut(|s| s.stop());
    });

    let keybinds = "Esc -> exit, o -> Open File, a -> set arguments, r -> run";

    tui! {
        <Flex direction={Direction::Vertical} gap={1}>
            <Paragraph::new("Layout demo — Tab/Shift+Tab to focus, Up/Down to scroll, Esc quits")
                block={Block::default().title_bottom(keybinds).borders(Borders::ALL)}
            />
        </Flex>
    }
}
