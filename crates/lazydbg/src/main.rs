use std::{io, time::Duration};

use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
};
use reactatui::prelude::*;

mod components;
mod ui;

use ui::demo_ui;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    // Enable mouse events
    execute!(std::io::stderr(), EnableMouseCapture)?;
    let result = run(&mut terminal);
    execute!(std::io::stderr(), DisableMouseCapture)?;
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> io::Result<()> {
    loop {
        reactatui::hooks::begin_frame();

        terminal.draw(|frame| {
            frame.render_node(demo_ui("lazydbg"), frame.area());
        })?;

        if event::poll(Duration::from_millis(16))? {
            // Drain all pending events to prevent lag when many events are queued (like fast mouse movements)
            loop {
                match event::read()? {
                    Event::Key(key) => {
                        reactatui::hooks::dispatch_key(key);
                    }
                    Event::Mouse(mouse) => {
                        reactatui::hooks::dispatch_mouse(mouse);
                    }
                    _ => {}
                }
                // Check if there are more events available immediately
                if !event::poll(Duration::from_millis(0))? {
                    break;
                }
            }
        }

        if use_state_keyed("should_quit", || false).get() {
            break;
        }
    }

    Ok(())
}
