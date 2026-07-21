use clap::Parser;
use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
};
use reactatui::prelude::*;
use std::{io, time::Duration};
use ui::Home;

mod components;
mod interface;
mod ui;

#[derive(Parser, Debug, Default, Copy, Clone)]
struct Args {
    #[arg(short, long)]
    gdb: bool,
    #[arg(short, long)]
    lldb: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    if args.gdb && args.lldb {
        println!("Please select either gdb or lldb backend, not both.");
        return Ok(());
    }

    // Transfer args to global state
    use_global_with("cli-args", move || args);

    // Start the ratatui app
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
            frame.render_node(Home(), frame.area());
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

        if use_global_or_default("should_quit").get() {
            break;
        }
    }

    Ok(())
}
