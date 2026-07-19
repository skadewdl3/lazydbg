use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use reactatui::prelude::*;

use crate::components::{action_button, message_input, scrollable_log};

/// A three-pane layout component managing active pane focus and event routing.
///
/// Focus order (via Tab/Shift+Tab):
/// 0: Message Input
/// 1: Action Buttons (Save / Run / Clear)
/// 2: Event Log
#[component]
pub fn panel<'a>() -> TuiNode<'a> {
    let log = use_state_keyed("log", Vec::<String>::new);
    let active_pane = use_state_keyed("active_pane", || 0_usize); // 0: Input, 1: Buttons, 2: Log
    let keys = use_key();

    // Focus management
    {
        let active_pane = active_pane.clone();
        keys.on(KeyCode::Tab, move || {
            active_pane.with_mut(|p| *p = (*p + 1) % 3);
        });
    }
    {
        let active_pane = active_pane.clone();
        keys.on_modified(KeyCode::BackTab, KeyModifiers::SHIFT, move || {
            active_pane.with_mut(|p| *p = (*p + 2) % 3);
        });
    }

    // Receive "submitted" text events from the input child
    use_on::<String>("submitted", {
        let log = log.clone();
        move |text: &String| {
            log.with_mut(|v| v.push(format!("[input] {text}")));
            Propagation::Continue
        }
    });

    // Receive "clicked" events from action_button children
    use_on::<String>("clicked", {
        let log = log.clone();
        move |label: &String| {
            log.with_mut(|v| v.push(format!("[click] {label}")));
            Propagation::Continue
        }
    });

    // Receive "quit" — stop it here so the root never sees it
    use_on::<()>("quit", |_| Propagation::Stop);

    let current_pane = active_pane.get();

    let input_active = current_pane == 0;
    let buttons_active = current_pane == 1;
    let log_active = current_pane == 2;

    let buttons_border_style = if buttons_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let log_border_style = if log_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let set_pane_0 = active_pane.clone();
    let set_pane_1 = active_pane.clone();
    let set_pane_2 = active_pane.clone();

    tui! {
        <Flex direction={Direction::Horizontal} gap={1}>
            <FlexItem flex={1}>
                <Flex direction={Direction::Vertical} gap={1}>
                    <FlexItem flex={1}>
                        <Block::default
                            title={"Input"}
                            borders={Borders::ALL}
                            style={if input_active { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) }}
                            on:click={move |_| set_pane_0.with_mut(|p| *p = 0)}
                        >
                            <message_input is_active={input_active} />
                        </Block>
                    </FlexItem>
                    <FlexItem flex={1}>
                        <Block::default
                            title={"Buttons"}
                            borders={Borders::ALL}
                            style={buttons_border_style}
                            on:click={move |_| set_pane_1.with_mut(|p| *p = 1)}
                        >
                            <Flex direction={Direction::Horizontal} gap={1}>
                                <FlexItem flex={1}>
                                    <action_button label={"[ Save ]"} event_name={"clicked"} />
                                </FlexItem>
                                <FlexItem flex={1}>
                                    <action_button label={"[ Run  ]"} event_name={"clicked"} />
                                </FlexItem>
                                <FlexItem flex={1}>
                                    <action_button label={"[ Clear]"} event_name={"clicked"} />
                                </FlexItem>
                            </Flex>
                        </Block>
                    </FlexItem>
                </Flex>
            </FlexItem>
            <FlexItem flex={2}>
                <Block::default
                    title={"Event log"}
                    borders={Borders::ALL}
                    style={log_border_style}
                    on:click={move |_| set_pane_2.with_mut(|p| *p = 2)}
                >
                    <scrollable_log is_active={log_active} />
                </Block>
            </FlexItem>
        </Flex>
    }
}
