use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, StatefulWidget, Widget},
};

pub struct Input<'a> {
    placeholder: &'a str,
    block: Option<Block<'a>>,
    style: Style,
    focused: bool,
}

impl<'a> Input<'a> {
    pub fn new(placeholder: &'a str) -> Self {
        Self {
            placeholder,
            block: None,
            style: Style::default(),
            focused: true,
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
        } else if self.focused {
            state.value_with_cursor()
        } else {
            state.value.clone()
        };

        Line::from(text).style(self.style).render(area, buf);
    }
}

#[derive(Default, Clone, Debug, Eq, PartialEq)]
pub struct InputState {
    value: String,
    cursor: usize,
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
        use ratatui::crossterm::event::{KeyCode, KeyModifiers};

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

    fn insert(&mut self, ch: char) {
        let byte = self.byte_index(self.cursor);
        self.value.insert(byte, ch);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.byte_index(self.cursor - 1);
        let end = self.byte_index(self.cursor);
        self.value.replace_range(start..end, "");
        self.cursor -= 1;
    }

    fn delete(&mut self) {
        if self.cursor >= self.char_len() {
            return;
        }
        let start = self.byte_index(self.cursor);
        let end = self.byte_index(self.cursor + 1);
        self.value.replace_range(start..end, "");
    }

    fn value_with_cursor(&self) -> String {
        let mut value = self.value.clone();
        value.insert(self.byte_index(self.cursor), '|');
        value
    }

    fn clamp_cursor(&mut self) {
        self.cursor = self.cursor.min(self.char_len());
    }

    fn char_len(&self) -> usize {
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
