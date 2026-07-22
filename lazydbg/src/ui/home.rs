use crate::interface::DbgSession;
use crate::ui::Keybinds;
use crate::ui::panes::Pane;
use crate::ui::panes::{disassembly::Disassembly, frame::Frame, stack::Stack, status::Status};
use reactatui::keybindings;
use reactatui::prelude::*;

/// The root UI component containing a title bar and the main multi-pane panel.
/// Responds to global Esc key to trigger application quit.
#[component]
pub fn Home<'a>() -> TuiNode<'a> {
    let should_quit = use_global_with("should-quit", || false);
    let session = use_global::<DbgSession>("dbg-session");
    let keys = use_key();

    keybindings!(keys, {
       "q" => move || should_quit.set(true),
       "tab" => move || Pane::next(),
       "shift+tab" | "backtab" => move || Pane::prev(),
       "s" => move || session.with_mut(|s| s.stop())
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
        </Flex>
    }
}
