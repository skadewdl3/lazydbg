use crate::ui::Keybinds;
use crate::ui::panes::Pane;
use crate::ui::panes::logs::Logs;
use crate::ui::panes::{disassembly::Disassembly, frame::Frame, status::Status};
use reactatui::keybindings;
use reactatui::prelude::*;
use reactatui_widgets::{Dialog, Input};

use crate::app_state::AppState;

#[derive(Copy, Clone)]
enum InputUse {
    None,
    Binary,
    Breakpoint,
}

/// The root UI component containing a title bar and the main multi-pane panel.
/// Owns application-wide key bindings and pane composition.
#[component]
pub fn Home<'a>() -> TuiNode<'a> {
    let app = resource::<AppState>();
    let session = app.session.clone();
    let keys = use_key();
    let open = state(|| false);
    let input_use = state(|| InputUse::None);

    let binary_open = open.clone();
    let binary_use = input_use.clone();
    let breakpoint_open = open.clone();
    let breakpoint_use = input_use.clone();
    let escape_open = open.clone();
    let stop_session = session.clone();
    let list_session = session.clone();
    let run_session = session.clone();
    let frames_session = session.clone();
    let frames = app.frames.clone();

    keybindings!(keys, {
       "q" => move || resource::<AppState>().should_quit.set(true),
       "tab" => move || Pane::next(),
       "shift+tab" | "backtab" => move || Pane::prev(),
       "s" => move || stop_session.with_mut(|s| s.stop()),
       "o" => move || { binary_use.set(InputUse::Binary); binary_open.set(true); },
       "b" => move || { breakpoint_use.set(InputUse::Breakpoint); breakpoint_open.set(true); },
       "esc" => move || escape_open.set(false),
       "l" => move || list_session.with_mut(|s| s.list_breakpoints()),
       "r" => move || run_session.with_mut(|s| s.run()),
       "t" => move || frames.set(frames_session.with_mut(|s| s.frames())),
    });

    let submit_open = open.clone();
    let submit_use = input_use.clone();
    let submit_session = session.clone();
    let submit_handler = move |string: Option<String>| {
        submit_open.set(false);
        let Some(string) = string else {
            return;
        };
        match submit_use.get() {
            InputUse::Binary => {
                submit_session.with_mut(|s| {
                    s.open_file(string.clone());
                });
            }
            InputUse::Breakpoint => {
                submit_session.with_mut(|s| {
                    s.set_breakpoint(string.clone());
                });
            }
            _ => {}
        }
    };

    let input = tui! {
        <Input("Enter binary", open.get(), true) on:submit={submit_handler} />
    };

    tui! {
        <Flex::horizontal>
            <Flex::horizontal>
                <Flex::vertical>
                    <Flex::vertical>
                        <Status layout={layout!{size: 3 }} />
                        <Frame   layout={layout!{size: 1fr }} />
                        <Disassembly   layout={layout!{size: 1fr }} />
                    </Flex>
                </Flex>
                <Logs is_active={false} />
            </Flex>
            <Keybinds layout={layout!{size: 1}} />
            <Dialog visible={open.get()} width="50%" layout={layout!{ignore: true}}>
                <{input} />
            </Dialog>
        </Flex>
    }
}
