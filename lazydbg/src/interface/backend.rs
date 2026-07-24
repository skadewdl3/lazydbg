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
    fn frames(&mut self);
}

// maybe have traits for DbgFrame (one frame dyn object), DbgBreakpoints (breakpoint handler),
// DbgRegisters
