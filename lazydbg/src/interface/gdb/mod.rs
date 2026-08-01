use lazydbg_mi::{
    MiCommand, Record, Value, build_line,
    commands::{
        BreakInsert, BreakList, ExecRun, FileExecFile, FileSymbolFile, FrameInfo, StackListFrames,
    },
    parse_line,
    record::AsyncClass,
};
use tracing::{error, info, trace};

use crate::interface::{
    DbgBackend,
    backend::{BackendError, DbgBackendStatus, DbgFrame},
};
use std::{
    collections::HashMap,
    io::Write,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

use std::io;
use std::io::{BufRead, BufReader};

type PendingMap = HashMap<u64, mpsc::Sender<Record>>;
type AsyncListener = Box<dyn Fn(&AsyncClass, &HashMap<String, Value>) + Send + 'static>;

#[allow(unused)]
pub struct GdbBackend {
    pub process: Child,
    stdin: ChildStdin,
    reader: JoinHandle<()>,
    status: DbgBackendStatus,
    token: u64,
    pending: Arc<Mutex<PendingMap>>,
    async_listeners: Arc<Mutex<Vec<AsyncListener>>>,
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
        let stdout = process.stdout.take().unwrap();

        let pending: Arc<Mutex<HashMap<u64, mpsc::Sender<Record>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_reader = Arc::clone(&pending);

        let async_listeners: Arc<Mutex<Vec<AsyncListener>>> = Arc::new(Mutex::new(Vec::new()));
        let listeners_reader = Arc::clone(&async_listeners);

        let reader = thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        // EOF: gdb exited, stop reading
                        break;
                    }
                    Ok(_) => {
                        let record = match parse_line(&line) {
                            Ok(r) => r,
                            Err(e) => {
                                error!("failed to parse MI line {:?}: {}", line, e);
                                continue;
                            }
                        };

                        match &record {
                            Record::Result { token: Some(t), .. } => {
                                let sender = pending_reader.lock().unwrap().remove(t);
                                match sender {
                                    Some(tx) => {
                                        let _ = tx.send(record);
                                    }
                                    None => {
                                        error!("received result for unknown token {}", t);
                                    }
                                }
                            }
                            Record::Async { class, results, .. } => {
                                trace!("{:?}: {:#?}", class, results);
                                let listeners = listeners_reader.lock().unwrap();
                                for listener in listeners.iter() {
                                    listener(class, results);
                                }
                            }
                            _ => {
                                // Stream records (console/target/log) and Prompt
                                // are dropped here for now.
                            }
                        }
                    }
                    Err(e) => {
                        error!("error reading from gdb stdout: {}", e);
                        break;
                    }
                }
            }
        });

        Self {
            process,
            stdin,
            reader,
            status: DbgBackendStatus::Active,
            token: 0,
            pending,
            async_listeners,
        }
    }

    // TODO: add listeners
    fn register_async_listeners(&mut self) {}

    pub fn use_token(&mut self) -> u64 {
        let tk = self.token;
        self.token += 1;
        tk
    }

    pub fn send<C: MiCommand>(&mut self, cmd: C) -> Result<C::Reply, BackendError> {
        let token = self.use_token();
        let cmd_str = build_line(&cmd, token)?;

        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(token, tx);

        tracing::debug!("Sending mi command: {:#?}", cmd_str);
        self.stdin.write_all(cmd_str.as_bytes())?;
        self.stdin.flush()?;

        let record = rx.recv().map_err(|_| {
            BackendError::Io(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "reader thread died",
            ))
        })?;

        C::parse_reply(&record).unwrap().map_err(|err| err.into())
    }

    /// Register a listener that runs whenever an async record matching
    /// `wanted` comes in. `AsyncClass::Unknown` compares by inner string,
    /// so registering for `AsyncClass::Unknown("foo".into())` only fires
    /// for that exact unrecognized class name.
    #[allow(unused)]
    pub fn on_async<F>(&mut self, wanted: AsyncClass, callback: F)
    where
        F: Fn(&HashMap<String, Value>) + Send + 'static,
    {
        self.async_listeners.lock().unwrap().push(Box::new(
            move |class: &AsyncClass, results: &HashMap<String, Value>| {
                let matches = match (&wanted, class) {
                    (AsyncClass::Unknown(w), AsyncClass::Unknown(c)) => w == c,
                    _ => std::mem::discriminant(class) == std::mem::discriminant(&wanted),
                };
                if matches {
                    callback(results);
                }
            },
        ));
    }
}

impl DbgBackend for GdbBackend {
    fn init(&mut self) {
        self.register_async_listeners();
    }

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
                info!("{:#?}", reply);
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
                info!("{:#?}", reply);
            }
            Err(err) => {
                error!("{}", err.to_string())
            }
        };
    }

    fn breakpoints(&mut self) {
        let res = self.send(BreakList {});

        match res {
            Ok(reply) => {
                info!("{:#?}", reply);
            }
            Err(err) => {
                error!("{}", err.to_string())
            }
        };
    }

    fn set_breakpoint(&mut self, bp: String) {
        let res = self.send(BreakInsert {
            positional: Some(bp),
            ..Default::default()
        });

        match res {
            Ok(reply) => {
                info!("{:#?}", reply);
            }
            Err(err) => {
                error!("{}", err.to_string())
            }
        };
    }

    fn run(&mut self) {
        let res = self.send(ExecRun {});

        match res {
            Ok(reply) => {
                info!("{:#?}", reply);
            }
            Err(err) => {
                error!("{}", err.to_string())
            }
        };
    }

    fn frames(&mut self) -> Result<Vec<Box<dyn DbgFrame>>, BackendError> {
        let res = self.send(StackListFrames::default());

        match res {
            Ok(reply) => {
                info!("frame info: {:#?}", reply);
                let frames: Vec<Box<dyn DbgFrame>> = reply
                    .stack
                    .into_iter()
                    .map(|f| Box::new(f) as Box<dyn DbgFrame>)
                    .collect();
                Ok(frames)
            }
            Err(err) => {
                error!("{}", err.to_string());
                Err(err)
            }
        }
    }
}

impl DbgFrame for FrameInfo {
    fn addr(&self) -> Option<String> {
        return self.addr.clone();
    }

    fn func(&self) -> Option<String> {
        return self.func.clone();
    }
    fn file(&self) -> Option<String> {
        return self.file.clone();
    }
    fn line(&self) -> Option<String> {
        return self.line.clone();
    }

    fn level(&self) -> Option<String> {
        return self.level.clone();
    }

    fn clone_box(&self) -> Box<dyn DbgFrame> {
        Box::new(self.clone())
    }
}
