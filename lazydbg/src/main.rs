use clap::Parser;
use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
};
use reactatui::prelude::*;
use std::{io, time::Duration};
use ui::Home;

use crate::interface::{DbgBackend, DbgSession, gdb::GdbBackend};

mod components;
mod interface;
mod parsers;
mod ui;

#[derive(Parser, Debug, Default, Copy, Clone)]
struct Args {
    #[arg(short, long)]
    gdb: bool,
    #[arg(short, long)]
    lldb: bool,
}

fn init<'a>(args: Args) -> Result<(), &'a str> {
    // Initialize a debug backend based on if the user passes
    // --gdb or --lldb. Default is --gdb.
    let backend: Box<dyn DbgBackend> = {
        if args.lldb && args.gdb {
            return Err(
                "Cannot use two debugger backends at once. Please pass either --lldb or --gdb",
            );
        } else if args.lldb {
            todo!("LLDB Backend isn't implemented yet");
        } else {
            Box::new(GdbBackend::new())
        }
    };

    // Store the debug session in the global state
    use_global_with("dbg-session", || DbgSession::new(backend));
    Ok(())
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    match init(args) {
        Ok(_) => (),
        Err(err) => {
            println!("Error: {}", err);
            return Ok(());
        }
    }

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

        if use_global_or_default("should-quit").get() {
            break;
        }
    }

    Ok(())
}
