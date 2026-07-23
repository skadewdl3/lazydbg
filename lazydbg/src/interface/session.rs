use crate::interface::{DbgBackend, backend::DbgBackendStatus};

pub struct DbgSession {
    backend: Box<dyn DbgBackend>,
}

impl DbgSession {
    pub fn new(backend: Box<dyn DbgBackend>) -> Self {
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
}
