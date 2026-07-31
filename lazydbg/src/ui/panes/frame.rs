use ratatui::widgets::{Borders, Paragraph};
use reactatui::{
    TuiNode, component,
    hooks::{Propagation, emitter, global_or, memo, state, use_key},
    keybindings, tui,
};
use reactatui_widgets::{Block, List};

use crate::interface::backend::DbgFrame;

/// A single stack frame item component.
#[component]
pub fn FrameItem<'a>(frame: &Box<dyn DbgFrame>, #[prop] active: bool) -> TuiNode<'a> {
    let level = frame.level().unwrap_or("?".into());
    let addr = frame.addr().unwrap_or("?".into());
    let func = frame.func().unwrap_or("?".into());
    let file = frame.file().unwrap_or("?".into());
    let line = frame.line().unwrap_or("?".into());
    let emitter = emitter::<()>("frame_selected");

    let thing = move |_| {
        emitter.emit(());
        Propagation::Stop
    };

    // Format the frame information
    let text = format!("#{} {} {} ({}:{})", level, addr, func, file, line);

    tui! {
        <Paragraph::new(text)
            on:click={thing}
        />
    }
}

#[component]
pub fn Frame<'a>() -> TuiNode<'a> {
    // TODO: What happens when the frame count changes?
    let frames = global_or::<Vec<Box<dyn DbgFrame>>>("frames", || Vec::new());
    let frame_count = memo(frames, |f| f.len() as i64);
    let selected_frame = state::<Option<i64>>(|| None);
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
        <Block::default title="Stack Frame" borders={Borders::ALL}>
            <List virtual={false}>
                for frame in frames.get() {
                    <FrameItem(&frame)
                        active={
                            selected_frame.get().unwrap() == frame.level().unwrap().parse::<i64>().unwrap()
                        }
                    />
                }
            </List>
        </Block>
    }
}
