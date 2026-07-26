use ratatui::{
    layout::Direction,
    widgets::{Borders, Paragraph},
};
use reactatui::{
    TuiNode, component,
    hooks::{use_emit, use_global_with, use_key, use_memo, use_state},
    keybindings, tui,
};
use reactatui_macros::style;
use reactatui_widgets::Block;

use crate::interface::backend::DbgFrame;

/// A single stack frame item component.
#[component]
pub fn FrameItem<'a>(frame: &Box<dyn DbgFrame>, active: bool) -> TuiNode<'a> {
    let level = frame.level().unwrap_or("?".into());
    let addr = frame.addr().unwrap_or("?".into());
    let func = frame.func().unwrap_or("?".into());
    let file = frame.file().unwrap_or("?".into());
    let line = frame.line().unwrap_or("?".into());
    let emitter = use_emit::<()>("frame_selected");

    let thing = move |_| emitter.emit(());

    let st = style! {
        background: if active { green } else if 2 == 2 { blue };
    };

    // Format the frame information
    let text = format!("#{} {} {} ({}:{})", level, addr, func, file, line);

    tui! {
        <Paragraph::new(text)
            style={st}
            on:click={thing}
        />
    }
}

#[component]
pub fn Frame<'a>() -> TuiNode<'a> {
    // TODO: What happens when the frame count changes?
    let frames = use_global_with::<Vec<Box<dyn DbgFrame>>>("frames", || Vec::new());
    let frame_count = use_memo(frames, |f| f.len() as i64);
    let selected_frame = use_state::<Option<i64>>(|| None);
    let keys = use_key();

    keybindings!(keys, {
       "j" | "down" => move || {
           if frame_count.get() <= 0 { return; }
           let current_frame = selected_frame.get().unwrap();
           selected_frame.set(Some((current_frame + 1).rem_euclid(frame_count.get())));
       },
       "k" | "up" => move || {
           if frame_count.get() <= 0 { return; }
           let current_frame = selected_frame.get().unwrap();
           selected_frame.set(Some((current_frame - 1).rem_euclid(frame_count.get())));
       }
    });

    if frame_count.get() > 0 && selected_frame.get().is_none() {
        selected_frame.set(Some(0));
    }

    tui! {
        <Block::default title={"Stack Frame"} borders={Borders::ALL}>
            <Flex direction={Direction::Vertical}>
                for frame in frames.get() {
                    <FrameItem(
                            &frame,
                            selected_frame.get().unwrap() == frame.level().unwrap().parse::<i64>().unwrap()
                        )
                        style={style!{ flex-basis: 1 }}
                    />
                }
            </Flex>
        </Block>
    }
}
