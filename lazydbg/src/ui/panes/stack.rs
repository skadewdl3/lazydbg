use lazydbg_mi::commands::FrameInfo;
use ratatui::{
    crossterm::event::MouseButton,
    style::Style,
    widgets::{Borders, Paragraph},
};
use reactatui::{TuiNode, component, hooks::use_emit, tui};
use reactatui_widgets::Block;

use crate::ui::panes::Pane;
