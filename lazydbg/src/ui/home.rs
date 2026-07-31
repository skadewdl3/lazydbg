use crate::interface::DbgSession;
use crate::ui::Keybinds;
use crate::ui::panes::Pane;
use crate::ui::panes::logs::Logs;
use crate::ui::panes::{disassembly::Disassembly, frame::Frame, status::Status};
use reactatui::keybindings;
use reactatui::prelude::*;
use reactatui_widgets::{Dialog, Input};

#[derive(Copy, Clone)]
enum InputUse {
    None,
    Binary,
    Breakpoint,
}

/// The root UI component containing a title bar and the main multi-pane panel.
/// Responds to global Esc key to trigger application quit.
#[component]
pub fn Home<'a>() -> TuiNode<'a> {
    let should_quit = global_or("should-quit", || false);
    let session = global::<DbgSession>("dbg-session");
    let keys = use_key();
    let open = state(|| false);
    let input_use = state(|| InputUse::None);

    let open_input = move |iu: InputUse| {
        input_use.set(iu);
        open.set(true)
    };

    keybindings!(keys, {
       "q" => move || should_quit.set(true),
       "tab" => move || Pane::next(),
       "shift+tab" | "backtab" => move || Pane::prev(),
       "s" => move || session.with_mut(|s| s.stop()),
       "o" => move || open_input(InputUse::Binary),
       "b" => move || open_input(InputUse::Breakpoint),
       "esc" => move || open.set(false),
       "l" => move || session.with_mut(|s| s.list_breakpoints()),
       "r" => move || session.with_mut(|s| s.run()),
       "t" => move || session.with_mut(|s| s.frames()),
    });

    let submit_handler = move |string: &Option<String>| {
        open.set(false);
        if string.is_none() {
            return Propagation::Stop;
        }
        let string = string.clone().unwrap();
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
        Propagation::Stop
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
