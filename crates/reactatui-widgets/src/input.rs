use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, StatefulWidget, Widget},
};
use reactatui::prelude::*;
use reactatui::ratatui::widgets::Paragraph;

/// State for managing text input value and cursor position.
#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct InputState {
    pub value: String,
    pub cursor: usize,
}

impl InputState {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self { value, cursor }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn handle_key(&mut self, key: ratatui::crossterm::event::KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.insert(ch);
                true
            }
            KeyCode::Backspace => {
                self.backspace();
                true
            }
            KeyCode::Delete => {
                self.delete();
                true
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.char_len());
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.char_len();
                true
            }
            _ => false,
        }
    }

    pub fn insert(&mut self, ch: char) {
        let byte = self.byte_index(self.cursor);
        self.value.insert(byte, ch);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.byte_index(self.cursor - 1);
        let end = self.byte_index(self.cursor);
        self.value.replace_range(start..end, "");
        self.cursor -= 1;
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.char_len() {
            return;
        }
        let start = self.byte_index(self.cursor);
        let end = self.byte_index(self.cursor + 1);
        self.value.replace_range(start..end, "");
    }

    pub fn value_with_cursor(&self) -> String {
        let mut value = self.value.clone();
        value.insert(self.byte_index(self.cursor), '|');
        value
    }

    pub fn clamp_cursor(&mut self) {
        self.cursor = self.cursor.min(self.char_len());
    }

    pub fn char_len(&self) -> usize {
        self.value.chars().count()
    }

    fn byte_index(&self, char_index: usize) -> usize {
        self.value
            .char_indices()
            .map(|(index, _)| index)
            .nth(char_index)
            .unwrap_or(self.value.len())
    }
}

/// A text input widget supporting showing/hiding a cursor and text editing.
pub struct Input<'a> {
    placeholder: &'a str,
    block: Option<Block<'a>>,
    style: Style,
    focused: bool,
    show_cursor: bool,
}

impl<'a> Input<'a> {
    pub fn new(placeholder: &'a str) -> Self {
        Self {
            placeholder,
            block: None,
            style: Style::default(),
            focused: true,
            show_cursor: true,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn show_cursor(mut self, show_cursor: bool) -> Self {
        self.show_cursor = show_cursor;
        self
    }
}

impl StatefulWidget for Input<'_> {
    type State = InputState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let area = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        state.clamp_cursor();
        let text = if state.value.is_empty() {
            self.placeholder.to_string()
        } else if self.focused && self.show_cursor {
            state.value_with_cursor()
        } else {
            state.value.clone()
        };

        Line::from(text).style(self.style).render(area, buf);
    }
}

/// A component input widget that tracks internal state, supports showing/hiding cursor,
/// handles normal text editing keybinds, and emits `"submit"` when Enter is pressed.
#[component]
pub fn SimpleInput<'a>(placeholder: &'a str, focused: bool, show_cursor: bool) -> TuiNode<'a> {
    let state = use_state(InputState::default);
    let submit_emitter = use_emit::<String>("submit");

    if focused {
        let keys = use_key();
        let submit_emitter = submit_emitter.clone();
        let state = state.clone();

        keys.on(KeyCode::Enter, move || {
            let val = state.with(|s| s.value.clone());
            submit_emitter.emit(val);
        });

        keys.on_any(move |event| {
            if event.code != KeyCode::Tab
                && event.code != KeyCode::BackTab
                && event.code != KeyCode::Enter
            {
                state.with_mut(|s| {
                    s.handle_key(event);
                });
            }
        });
    }

    let text = state.with(|s| {
        if s.value.is_empty() {
            placeholder.to_string()
        } else if focused && show_cursor {
            s.value_with_cursor()
        } else {
            s.value.clone()
        }
    });

    tui! {
        <Paragraph text={text} />
    }
}
