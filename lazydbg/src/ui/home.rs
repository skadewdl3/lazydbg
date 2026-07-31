use crate::ui::Keybinds;
use crate::ui::panes::Pane;
use crate::ui::panes::logs::Logs;
use crate::ui::panes::{disassembly::Disassembly, frame::Frame, status::Status};
use reactatui::keybindings;
use reactatui::prelude::*;
use reactatui_widgets::{Dialog, Input};

use crate::app_state::{APP_STATE_KEY, AppState};

#[derive(Copy, Clone)]
enum InputUse {
    None,
    Binary,
    Breakpoint,
}

/// The root UI component containing a title bar and the main multi-pane panel.
/// Owns application-wide key bindings and pane composition.
#[component]
pub fn Home<'a>(initial_app_state: Option<AppState>) -> TuiNode<'a> {
    let app = resource_or(APP_STATE_KEY, || {
        initial_app_state.expect("AppState must be supplied on the first render")
    });
    let session = app.session.clone();
    let open = state(|| false);
    let input_use = state(|| InputUse::None);
    let input_placeholder = memo(|| match input_use.get() {
        InputUse::Binary => "Enter binary path",
        InputUse::Breakpoint => "Enter breakpoint symbol",
        _ => "",
    });
    let text = state(String::new);

    let frames = app.frames.clone();

    keybindings! {
       "q" => move || resource::<AppState>(APP_STATE_KEY).should_quit.set(true),
       "tab" => move || Pane::next(),
       "shift+tab" | "backtab" => move || Pane::prev(),
       "s" => lambda!(+session, || session.with_mut(|s| s.stop())),
       "o" => lambda!(+input_use, +open, || {
           input_use.set(InputUse::Binary);
           open.set(true);
       }),
       "b" => lambda!(+input_use, +open, || {
           input_use.set(InputUse::Breakpoint);
           open.set(true);
       }),
       "l" => lambda!(+session, || session.with_mut(|s| s.list_breakpoints())),
       "r" => lambda!(+session, || session.with_mut(|s| s.run())),
       "t" => lambda!(+session, +frames, || {
           frames.set(session.with_mut(|s| s.frames()));
       }),
    }

    let submit_handler = lambda!(+open, +input_use, +session, |string: Option<String>| {
        open.set(false);
        let Some(string) = string else {
            return;
        };
        match input_use.get() {
            InputUse::Binary => {
                session.with_mut(|s| {
                    s.open_file(string.clone());
                });
            }
            InputUse::Breakpoint => {
                session.with_mut(|s| {
                    s.set_breakpoint(string.clone());
                });
            }
            _ => {}
        }
    });

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
            <Dialog bind:visible={open} width="50%" layout={layout!{ignore: true}}>
                <Input(*input_placeholder, open.get(), true) bind:value={text} on:submit={submit_handler} />
            </Dialog>
        </Flex>
    }
}
