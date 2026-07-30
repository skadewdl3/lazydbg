// crates/reactatui-widgets/src/button.rs
use ratatui::{
    layout::Alignment,
    widgets::{Borders, Paragraph},
};
use reactatui::prelude::*;

use crate::Block;

/// A clickable, hoverable, keyboard-activatable button widget.
///
/// Emits `"click"` with its label as a payload when activated — either by
/// a left mouse click, or (when `focused` is `true`) by pressing Enter or
/// Space. Disabled buttons render dimmed and never emit.
///
/// `style` is whatever you'd build with the `style!` macro — Button uses
/// its color half as the idle appearance and derives hover (reversed) and
/// disabled (dimmed) looks from it automatically:
///
/// ```ignore
/// <Button
///     label={"Save"}
///     style={style!{ color: cyan; bold; }}
///     borders={Borders::ALL}
///     disabled={false}
///     focused={true}
/// />
/// ```
///
/// Note: if `Button` is placed directly inside a `<Flex>`/`<Grid>`, a
/// `style={..}` prop there is intercepted by the layout macro for that
/// item's flex/grid placement instead of reaching this component — wrap
/// it in a plain fragment or an intermediate component if you need both.
#[component]
pub fn Button<'a>(label: &'a str, borders: Borders, disabled: bool) -> TuiNode<'a> {
    let hovered = use_state(|| false);
    let emit = use_emit::<()>("click");

    tui! {
        <Block::default
            borders={borders}
            on:click={move |btn| {
                if !disabled && btn == ratatui::crossterm::event::MouseButton::Left {
                    emit.emit(());
                }
            }}
            on:mousein={move || {
                if !disabled {
                    hovered.set(true);
                }
            }}
            on:mouseout={move || {
                hovered.set(false);
            }}
        >
            <Paragraph::new(label) alignment={Alignment::Center} />
        </Block>
    }
}
