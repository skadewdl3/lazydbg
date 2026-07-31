use reactatui::{Runtime, hooks::State};

use crate::{
    interface::{DbgSession, backend::DbgFrame},
    logger::SharedLogStore,
    ui::panes::Pane,
};

pub const APP_STATE_KEY: &str = "app_state";

pub struct AppState {
    pub should_quit: State<bool>,
    pub session: State<DbgSession>,
    pub frames: State<Vec<Box<dyn DbgFrame>>>,
    pub active_pane: State<Pane>,
    pub log_scroll: State<(u16, u16)>,
    pub logs: SharedLogStore,
}

impl AppState {
    pub fn new(runtime: &Runtime, session: DbgSession, logs: SharedLogStore) -> Self {
        Self {
            should_quit: runtime.create_state(false),
            session: runtime.create_state(session),
            frames: runtime.create_state(Vec::new()),
            active_pane: runtime.create_state(Pane::Frame),
            log_scroll: runtime.create_state((0, 0)),
            logs,
        }
    }
}
