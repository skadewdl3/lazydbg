use crate::{
    interface::{DbgBackend, backend::DbgBackendStatus},
    parsers::mi::{
        MiCommand, Record, build_line,
        commands::{FileExecFile, FileSymbolFile},
        parse_line,
    },
};
use std::{
    io::{Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

pub struct GdbBackend {
    pub process: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    status: DbgBackendStatus,
    token: u64,
}

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
            token: 0,
        }
    }

    pub fn use_token(&mut self) -> u64 {
        let tk = self.token;
        self.token += 1;
        tk
    }

    pub fn send<C: MiCommand>(&mut self, cmd: C) -> std::io::Result<C::Reply> {
        // todo!()
        let token = self.use_token();
        let cmd_str = build_line(&cmd, token).unwrap();
        let bytes = cmd_str.as_bytes();
        self.stdin.write_all(bytes).unwrap();
        let mut output = String::new();
        self.stdout.read_to_string(&mut output);
        let output = parse_line(&output);
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

    fn open_file(&mut self, path: String) {
        self.send(FileExecFile {
            positional: Some(path),
        });
    }

    fn load_symbols(&mut self, path: String) {
        self.send(FileSymbolFile {
            positional: Some(path),
        });
    }
}
