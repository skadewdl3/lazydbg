use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use reactatui::prelude::*;

use crate::components::{ActionButton, MessageInput, ScrollableLog};

/// A three-pane layout component managing active pane focus and event routing.
///
/// Focus order (via Tab/Shift+Tab):
/// 0: Message Input
/// 1: Action Buttons (Save / Run / Clear)
/// 2: Event Log
#[component]
pub fn Panel<'a>() -> TuiNode<'a> {
    let log = use_state_keyed("log", Vec::<String>::new);
    let active_pane = use_state_keyed("active_pane", || 0_usize); // 0: Input, 1: Buttons, 2: Log
    let keys = use_key();

    let next_pane = move || active_pane.with_mut(|p| *p = (*p + 1) % 3);
    let previous_pane = move || active_pane.with_mut(|p| *p = (*p + 2) % 3);
    let set_pane = move |pane: usize| active_pane.with_mut(|p| *p = pane);
    let is_active = |pane: usize| pane == active_pane.get();
    let get_style = |pane: usize| match is_active(pane) {
        true => Style::default().fg(Color::Yellow),
        false => Style::default().fg(Color::DarkGray),
    };

    // Focus management keybinds
    keys.on(KeyCode::Tab, next_pane);
    keys.on_modified(KeyCode::BackTab, KeyModifiers::SHIFT, previous_pane);

    let add_log = move |label: &String| {
        log.with_mut(|v| v.push(format!("[click] {label}")));
        Propagation::Continue
    };

    tui! {
        <Flex direction={Direction::Horizontal} gap={1}>
            <Flex direction={Direction::Vertical} gap={1} flex={1}>
                <Block::default
                    title={"Input"}
                    borders={Borders::ALL}
                    style={get_style(0)}
                    on:click={move |_| set_pane(0)}
                    flex={1}
                >
                    <MessageInput(is_active(0))
                        on:submit={move |text: &String| {
                            log.with_mut(|v| v.push(format!("[input] {text}")));
                            Propagation::Continue
                        }}
                        on:quit={|_: &()| Propagation::Stop}
                    />
                </Block>
                <Block::default
                    title={"Buttons"}
                    borders={Borders::ALL}
                    style={get_style(1)}
                    on:click={move |_| set_pane(1)}
                    flex={1}
                >
                    <Flex direction={Direction::Horizontal} gap={1}>
                        <ActionButton("[ Save ]") on:clicked={move |label: &String| add_log(label)} flex={1} />
                        <ActionButton("[ Run  ]") on:clicked={move |label: &String| add_log(label)} flex={1} />
                        <ActionButton("[ Clear]") on:clicked={move |label: &String| add_log(label)} flex={1} />
                    </Flex>
                </Block>
            </Flex>
            <Block::default
                title={"Event log"}
                borders={Borders::ALL}
                style={get_style(2)}
                on:click={move |_| set_pane(2)}
                flex={2}
            >
                <ScrollableLog(is_active(2)) />
            </Block>
        </Flex>
    }
}
