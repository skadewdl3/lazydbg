use ratatui::widgets::Paragraph;
use reactatui::{
    TuiNode, component,
    hooks::{resource, state},
    keybindings, lambda, layout, style, tui,
};
use reactatui_widgets::{Block, List};

use crate::app_state::{APP_STATE_KEY, AppState};
use crate::interface::backend::DbgFrame;

/// A single stack frame item component.
#[component]
pub fn FrameItem<'a>(frame: &dyn DbgFrame, #[prop] active: bool) -> TuiNode<'a> {
    let level = frame.level().unwrap_or("?".into());
    let addr = frame.addr().unwrap_or("?".into());
    let func = frame.func().unwrap_or("?".into());
    let file = frame.file().unwrap_or("?".into());
    let line = frame.line().unwrap_or("?".into());
    // Format the frame information
    let text = format!("#{} {} {} ({}:{})", level, addr, func, file, line);
    let row_style = style! {
        if active {
            text-style: reversed;
        }
    };

    TuiNode::from_widget(Paragraph::new(text).style(row_style))
}

#[component]
pub fn Frame<'a>() -> TuiNode<'a> {
    // TODO: What happens when the frame count changes?
    let frames = resource::<AppState>(APP_STATE_KEY).frames.clone();
    let frame_count = frames.with(Vec::len);
    let selected_frame = state(|| 0_usize);
    keybindings! {
       "j" | "down" => lambda!(+selected_frame, || {
           if frame_count == 0 { return; }
           selected_frame.update(|selected| *selected = (*selected + 1) % frame_count);
       }),
       "k" | "up" => lambda!(+selected_frame, || {
           if frame_count == 0 { return; }
           selected_frame.update(|selected| {
               *selected = if *selected == 0 { frame_count - 1 } else { *selected - 1 };
           });
       })
    }

    if frame_count > 0 && selected_frame.get() >= frame_count {
        selected_frame.set(frame_count - 1);
    }
    let block_style = style! { borders: all; };

    tui! {
        <Block::default title="Stack Frame" borders={&block_style}>
            <List virtual={false}>
                for (index, frame) in frames.get().into_iter().enumerate() {
                    <FrameItem(frame.as_ref())
                        layout={layout!{ size: 1 }}
                        key={index}
                        active={selected_frame.get() == index}
                    />
                }
            </List>
        </Block>
    }
}
