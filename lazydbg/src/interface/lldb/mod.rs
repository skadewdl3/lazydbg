use crate::interface::{DbgBackend, backend::DbgBackendStatus};

pub struct LldbBackend;

impl LldbBackend {
    pub fn new() -> Self {
        todo!()
    }
}

impl DbgBackend for LldbBackend {
    fn kill(&mut self) {
        todo!()
    }

    fn status(&mut self) -> DbgBackendStatus {
        todo!()
    }
}
