use crate::interface::DbgSession;
use crate::ui::Keybinds;
use crate::ui::panes::Pane;
use crate::ui::panes::{disassembly::Disassembly, frame::Frame, stack::Stack, status::Status};
use ratatui::widgets::Paragraph;
use reactatui::keybindings;
use reactatui::prelude::*;
use reactatui_widgets::{Button, Dialog, Input};

/// The root UI component containing a title bar and the main multi-pane panel.
/// Responds to global Esc key to trigger application quit.
#[component]
pub fn Home<'a>() -> TuiNode<'a> {
    let should_quit = use_global_with("should-quit", || false);
    let session = use_global::<DbgSession>("dbg-session");
    let keys = use_key();
    let open = use_state(|| false);

    keybindings!(keys, {
       "q" => move || should_quit.set(true),
       "tab" => move || Pane::next(),
       "shift+tab" | "backtab" => move || Pane::prev(),
       "s" => move || session.with_mut(|s| s.stop()),
       "o" => move || open.set(true),
       "esc" => move || open.set(false)
    });

    tui! {
        <Flex direction={Direction::Vertical} layout={"1fr, 1"}>
            <Flex direction={Direction::Horizontal} gap={1}>
                <Flex direction={Direction::Vertical} layout={"3, 1fr, 1fr"}>
                    <Status />
                    <Frame />
                    <Disassembly />
                </Flex>
                <Stack />
            </Flex>
            <Keybinds />
            <Dialog::default flex_ignore visible={open.get()}>
                <Button("Hi mom") />
            </Dialog>
        </Flex>
    }
}
