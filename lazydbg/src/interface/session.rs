use reactatui::hooks::global_or;

use crate::interface::{
    DbgBackend,
    backend::{DbgBackendStatus, DbgFrame},
};

pub struct DbgSession {
    backend: Box<dyn DbgBackend>,
}

impl DbgSession {
    pub fn new(backend: Box<dyn DbgBackend>) -> Self {
        let mut backend = backend;
        backend.init();
        Self { backend }
    }

    pub fn is_alive(&mut self) -> bool {
        self.backend.status() == DbgBackendStatus::Active
    }

    pub fn stop(&mut self) {
        self.backend.kill();
    }

    pub fn open_file<'a>(&mut self, path: String) {
        self.backend.open_file(path.clone());
        self.backend.load_symbols(path);
    }

    pub fn list_breakpoints(&mut self) {
        self.backend.breakpoints();
    }

    pub fn set_breakpoint(&mut self, bp: String) {
        self.backend.set_breakpoint(bp);
    }

    pub fn run(&mut self) {
        self.backend.run();
    }

    pub fn frames(&mut self) {
        let f = self.backend.frames().unwrap();
        let frames = global_or::<Vec<Box<dyn DbgFrame>>>("frames", || Vec::new());
        frames.set(f);
    }
}
