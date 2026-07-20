use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DbgBackendStatus {
    Active,
    Waiting,
    Killed,
}

pub trait DbgBackend {
    fn kill(&mut self);
    fn status(&mut self) -> DbgBackendStatus;
}

pub struct GdbBackend {
    pub process: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    status: DbgBackendStatus,
}
pub struct LldbBackend;

impl GdbBackend {
    pub fn new() -> Self {
        let mut process = Command::new("gdb")
            .args(["--interpreter=mi3"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Unable to run `gdb --interpreter=mi3`. Please make sure `gdb` is on path.");

        let stdin = process.stdin.take().unwrap();
        let stdout = process.stdout.take().unwrap();

        Self {
            process,
            stdin,
            stdout,
            status: DbgBackendStatus::Active,
        }
    }
}
impl DbgBackend for GdbBackend {
    fn kill(&mut self) {
        self.status = DbgBackendStatus::Waiting;
        self.process.kill().unwrap();
        self.process.wait().unwrap();
        self.status = DbgBackendStatus::Killed;
    }

    fn status(&mut self) -> DbgBackendStatus {
        match self.process.try_wait() {
            Ok(Some(_)) => DbgBackendStatus::Killed,

            Ok(None) => DbgBackendStatus::Active,
            _ => panic!("Error polling debugger process status"),
        }
    }
}

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
