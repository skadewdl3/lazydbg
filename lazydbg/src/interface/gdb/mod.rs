use lazydbg_mi::{
    MiCommand, Record, build_line,
    commands::{FileExecFile, FileSymbolFile},
    parse_line,
};
use thiserror::Error;
use tracing::{error, info};

use crate::interface::{DbgBackend, backend::DbgBackendStatus};
use std::{
    io::Write,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use std::io;
use std::io::{BufRead, BufReader};

pub struct GdbBackend {
    pub process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    status: DbgBackendStatus,
    token: u64,
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("deserialization")]
    Deserialize(#[from] lazydbg_mi::error::DeserializationError),
    #[error("serialization")]
    Serialize(#[from] lazydbg_mi::error::SerializationError),
    #[error("parse")]
    Parse(#[from] lazydbg_mi::error::ParseError),
    #[error("io")]
    Io(#[from] std::io::Error),
}

impl GdbBackend {
    pub fn new() -> Self {
        let mut process = Command::new("gdb")
            .args(["-q", "--interpreter=mi3"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Unable to run `gdb --interpreter=mi3`. Please make sure `gdb` is on path.");

        let stdin = process.stdin.take().unwrap();
        let stdout = BufReader::new(process.stdout.take().unwrap());

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

    pub fn send<C: MiCommand>(&mut self, cmd: C) -> Result<C::Reply, BackendError> {
        let token = self.use_token();
        let cmd_str =
            build_line(&cmd, token).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        self.stdin.write_all(cmd_str.as_bytes())?;
        self.stdin.flush()?;

        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.stdout.read_line(&mut line)?;
            if bytes_read == 0 {
                return Err(
                    io::Error::new(std::io::ErrorKind::UnexpectedEof, "GDB closed stdout").into(),
                );
            }
            let record = parse_line(&line)?;
            if let Record::Result { token: Some(t), .. } = record
                && t == token
            {
                let reply = C::parse_reply(&record)
                    .expect("reply doesn't contain a map")
                    .map_err(|e| e.into());
                return reply;
            }
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

    fn open_file(&mut self, path: String) {
        let res = self.send(FileExecFile {
            positional: Some(path),
        });
        match res {
            Ok(reply) => {
                if let Ok(json) = serde_json::to_string(&reply) {
                    info!("{}", json);
                }
            }
            Err(err) => {
                error!("{}", err.to_string())
            }
        };
    }

    fn load_symbols(&mut self, path: String) {
        let res = self.send(FileSymbolFile {
            positional: Some(path),
        });

        match res {
            Ok(reply) => {
                if let Ok(json) = serde_json::to_string(&reply) {
                    info!("{}", json);
                }
            }
            Err(err) => {
                error!("{}", err.to_string())
            }
        };
    }
}
