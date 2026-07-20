use ratatui::crossterm::event::KeyCode;
use reactatui::prelude::*;

use crate::ui::Panel;

/// The root UI component containing a title bar and the main multi-pane panel.
/// Responds to global Esc key to trigger application quit.
#[component]
pub fn DemoUi<'a>(name: &'a str) -> TuiNode<'a> {
    let should_quit = use_state_keyed("should_quit", || false);

    let keys = use_key();
    keys.on(KeyCode::Esc, move || should_quit.set(true));

    tui! {
        <Flex direction={Direction::Vertical} gap={1}>
            <Block::default title={name} borders={Borders::ALL} flex={0}>
                <Paragraph text={"Layout demo — Tab/Shift+Tab to focus, Up/Down to scroll, Esc quits"} />
            </Block>
            <Panel flex={10} />
        </Flex>
    }
}
