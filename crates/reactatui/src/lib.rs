//! React-like rendering and hooks for Ratatui.
//!
//! Provides the `tui!` macro for declaring TUI node trees and the `#[component]` 
//! attribute macro for functional components, along with a powerful hooks system.

pub mod ext;
pub mod flex;
pub mod hooks;
pub mod layout;
pub mod node;
pub mod widgets;

pub use ratatui;
pub use reactatui_macros::{component, tui};

pub use ext::FrameExt;
pub use flex::{FlexItemNode, FlexNode};
pub use layout::Padding;
pub use node::{StateHandle, TuiNode};
pub use widgets::{Input, InputState};

pub mod prelude {
    pub use crate::hooks::{
        Emitter, KeyHandle, Propagation, State,
        use_emit, use_key, use_on, use_state, use_state_keyed,
    };
    pub use crate::{
        FlexItemNode, FlexNode, FrameExt, Input, InputState, Padding, StateHandle, TuiNode,
        component, ratatui, tui,
    };
    pub use ratatui::{
        layout::{Alignment, Constraint, Direction, Layout, Rect},
        style::{Color, Modifier, Style},
        widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Row, Table, Tabs},
    };
}
