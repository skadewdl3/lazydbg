pub mod ext;
pub mod hooks;
pub mod keys;
pub mod layout;
pub mod measure;
pub mod node;
pub mod style;

pub use reactatui_macros::{children, component, style, tui};

pub use ext::FrameExt;
pub use layout::{
    Align, FlexBasis, FlexItemNode, FlexNode, GridItemNode, GridNode, Justify, Padding,
};
pub use node::{StateHandle, TuiNode};
pub use style::CombinedStyle;

pub mod prelude {
    pub use crate::hooks::{
        Emitter, KeyHandle, Propagation, State, try_use_global, use_emit, use_global,
        use_global_or_default, use_global_with, use_key, use_on, use_state,
    };
    // layout::Style/Align/Justify/FlexBasis stay out of the prelude glob —
    // this module already re-exports ratatui::style::Style below, and
    // colliding those two under one glob import would be a footgun.
    // CombinedStyle and the `style!` macro are safe to include: neither
    // name collides with anything else here.
    pub use crate::{
        CombinedStyle, FlexItemNode, FlexNode, FrameExt, GridItemNode, GridNode, Padding,
        StateHandle, TuiNode, children, component, style, tui,
    };
    pub use ratatui::{
        layout::{Alignment, Constraint, Direction, Layout, Rect},
        style::{Color, Modifier, Style},
    };
}
