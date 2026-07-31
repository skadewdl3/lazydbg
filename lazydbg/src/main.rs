use clap::Parser;
use ratatui::crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture},
    execute,
};
use reactatui::prelude::*;
use std::{io, time::Duration};
use ui::Home;

use crate::{
    app_state::AppState,
    interface::{DbgBackend, DbgSession, gdb::GdbBackend},
    logger::init_logging,
};

mod app_state;
mod interface;
mod logger;
mod ui;

#[derive(Parser, Debug, Default, Copy, Clone)]
struct Args {
    #[arg(short, long)]
    gdb: bool,
    #[arg(short, long)]
    lldb: bool,
}

fn init<'a>(runtime: &Runtime, args: Args) -> Result<AppState, &'a str> {
    // Initialize a debug backend based on if the user passes
    // Initialize logging

    let logs = init_logging();

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

    Ok(AppState::new(runtime, DbgSession::new(backend), logs))
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let runtime = Runtime::new();

    let app_state = match init(&runtime, args) {
        Ok(app_state) => app_state,
        Err(err) => {
            println!("Error: {}", err);
            return Ok(());
        }
    };

    // Start the ratatui app
    let mut terminal = ratatui::try_init()?;
    // Enable mouse events
    execute!(std::io::stderr(), EnableMouseCapture)?;
    let result = run(&mut terminal, runtime, app_state);
    execute!(std::io::stderr(), DisableMouseCapture)?;
    ratatui::restore();
    result
}

fn run(
    terminal: &mut ratatui::DefaultTerminal,
    runtime: Runtime,
    app_state: AppState,
) -> io::Result<()> {
    let should_quit = app_state.should_quit.clone();
    let mut initial_app_state = Some(app_state);
    loop {
        if runtime.needs_render() {
            terminal.draw(|frame| {
                let app_state = initial_app_state.take();
                runtime.render(frame, frame.area(), || Home(app_state));
            })?;
        }

        if event::poll(Duration::from_millis(16))? {
            // Drain all pending events to prevent lag when many events are queued (like fast mouse movements)
            loop {
                runtime.handle_event(event::read()?);
                // Check if there are more events available immediately
                if !event::poll(Duration::from_millis(0))? {
                    break;
                }
            }
        }

        if should_quit.get() {
            break;
        }
    }

    Ok(())
}
