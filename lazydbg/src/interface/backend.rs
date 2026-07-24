use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use lazydbg_mi::{MiCommand, Record};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DbgBackendStatus {
    Active,
    Waiting,
    Killed,
}

pub trait DbgBackend {
    fn kill(&mut self);
    fn status(&mut self) -> DbgBackendStatus;
    fn open_file(&mut self, path: String);
    fn load_symbols(&mut self, path: String);
}
