use ratatui::widgets::Paragraph;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, StatefulWidget, Widget},
};
use reactatui::{keybindings, prelude::*};

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
pub struct InputBase<'a> {
    placeholder: &'a str,
    block: Option<Block<'a>>,
    style: Style,
    focused: bool,
    show_cursor: bool,
}

impl<'a> InputBase<'a> {
    pub fn new(placeholder: &'a str) -> Self {
        Self {
            placeholder,
            block: None,
            style: style! {}.into(),
            focused: true,
            show_cursor: true,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: impl Into<Style>) -> Self {
        self.style = style.into();
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

impl StatefulWidget for InputBase<'_> {
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

/// A component input widget that tracks internal state, handles text editing,
/// and calls `on:submit` with the value on Enter or `None` on Escape.
#[component]
pub fn Input<'a>(
    placeholder: &'a str,
    focused: bool,
    show_cursor: bool,
    #[bind] value: State<String>,
    #[prop] on_submit: Callback<Option<String>>,
) -> TuiNode<'a> {
    let value = bind(value);
    let cursor = state(|| 0usize);
    focus(focused);

    if focused {
        let enter_submit = on_submit.clone();
        let escape_submit = on_submit.clone();
        let submit_value = value.clone();
        let submit_cursor = cursor.clone();
        let escape_value = value.clone();
        let escape_cursor = cursor.clone();
        let edit_value = value.clone();
        let edit_cursor = cursor.clone();

        keybindings! {
            "enter" => move || {
                let val = submit_value.get();
                submit_value.set(String::new());
                submit_cursor.set(0);
                enter_submit.call(Some(val));
            },
            "esc" => move || {
                escape_value.set(String::new());
                escape_cursor.set(0);
                escape_submit.call(None);
            },
            key(k) if !matches!(k.code, KeyCode::Tab | KeyCode::BackTab) => move |event| {
                let cursor = edit_cursor.get();
                let next_cursor = edit_value.with_mut(|value| {
                    let cursor = cursor.min(value.chars().count());
                    let mut state = InputState {
                        value: value.clone(),
                        cursor,
                    };
                    state.handle_key(event);
                    *value = state.value;
                    state.cursor
                });
                edit_cursor.set(next_cursor);
                Propagation::Stop
            }
        }
    }

    let text = value.with(|value| {
        let state = InputState {
            value: value.clone(),
            cursor: cursor.get().min(value.chars().count()),
        };
        if state.value.is_empty() {
            placeholder.to_string()
        } else if focused && show_cursor {
            state.value_with_cursor()
        } else {
            state.value
        }
    });

    tui! {
        <Paragraph::new(text) />
    }
}
