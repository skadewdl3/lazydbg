use crate::interface::gdb::BackendError;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DbgBackendStatus {
    Active,
    Waiting,
    Killed,
}

pub trait DbgBackend {
    fn init(&mut self);
    fn kill(&mut self);
    fn status(&mut self) -> DbgBackendStatus;
    fn open_file(&mut self, path: String);
    fn load_symbols(&mut self, path: String);
    fn breakpoints(&mut self);
    fn set_breakpoint(&mut self, bp: String);
    fn run(&mut self);
    fn frames(&mut self) -> Result<Vec<Box<dyn DbgFrame>>, BackendError>;
}

pub trait DbgFrame {
    fn level(&self) -> Option<String>;
    fn addr(&self) -> Option<String>;
    fn func(&self) -> Option<String>;
    fn file(&self) -> Option<String>;
    fn line(&self) -> Option<String>;
    fn clone_box(&self) -> Box<dyn DbgFrame>;
}

impl Clone for Box<dyn DbgFrame> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
// maybe have traits for DbgFrame (one frame dyn object), DbgBreakpoints (breakpoint handler),
// DbgRegisters
