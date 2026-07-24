use ratatui::{
    style::Style,
    widgets::{Borders, Paragraph},
};
use reactatui::{
    TuiNode, component,
    hooks::{use_emit, use_global_with},
    tui,
};
use reactatui_widgets::Block;

use crate::{interface::backend::DbgFrame, ui::panes::Pane};

/// A single stack frame item component.
#[component]
pub fn FrameItem<'a>(index: usize, frame: &Box<dyn DbgFrame>) -> TuiNode<'a> {
    let level = frame.level().unwrap_or("?".into());
    let addr = frame.addr().unwrap_or("?".into());
    let func = frame.func().unwrap_or("?".into());
    let file = frame.file().unwrap_or("?".into());
    let line = frame.line().unwrap_or("?".into());
    let emitter = use_emit::<()>("frame_selected");

    let thing = move |_| emitter.emit(());

    // Format the frame information
    let text = format!("#{} {} {} {} ({}:{})", index, level, addr, func, file, line);

    let style = Style::default();

    tui! {
        <Paragraph::new(text)
            style={style}
            on:click={thing}
        />
    }
}

#[component]
pub fn Frame<'a>() -> TuiNode<'a> {
    let frames = use_global_with::<Vec<Box<dyn DbgFrame>>>("frames", || Vec::new());

    tui! {
        <Block::default title={"Stack Frame"} borders={Borders::ALL}>
            for (index, frame) in frames.get().iter().enumerate() {
                // <Paragraph::new(format!("Frame {:#?}, {:#?}", index, frame.addr())) />
                <FrameItem(index, frame) />
            }
        <Paragraph::new("Stack Frame Pane") />
        </Block>
    }
}
