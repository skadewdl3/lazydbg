use std::{
    collections::{BTreeMap, HashMap},
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, Condvar, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use lazydbg_dap::{
    Breakpoint, Capabilities, DapRequest, FunctionBreakpoint, InstructionBreakpoint,
    SetBreakpointsResponseBody, SetFunctionBreakpointsResponseBody,
    SetInstructionBreakpointsResponseBody, Source, SourceBreakpoint, StackFrame,
    StackTraceResponseBody, StoppedEventBody, ThreadsResponseBody,
    lldb::Configuration,
    requests::{
        ConfigurationDone, Disconnect, Initialize, Launch, SetBreakpoints, SetFunctionBreakpoints,
        SetInstructionBreakpoints, StackTrace, Threads,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info, trace, warn};

use crate::interface::{
    DbgBackend,
    backend::{BackendError, DbgBackendStatus, DbgFrame},
};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);
const INITIALIZED_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_HEADER_BYTES: usize = 8 * 1024;
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

type PendingMap = HashMap<u32, mpsc::Sender<ResponseEnvelope>>;

#[derive(Debug, Deserialize)]
struct ResponseEnvelope {
    #[serde(rename = "request_seq")]
    request_seq: u32,
    success: bool,
    command: String,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    body: Option<Value>,
}

#[derive(Serialize)]
struct RequestEnvelope<'a, A> {
    seq: u32,
    #[serde(rename = "type")]
    type_: &'static str,
    command: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<&'a A>,
}

struct PendingRequest<C: DapRequest> {
    receiver: mpsc::Receiver<ResponseEnvelope>,
    request: std::marker::PhantomData<C>,
}

#[derive(Default)]
struct AdapterState {
    initialized: bool,
    stopped_thread: Option<i32>,
    terminated: bool,
}

#[derive(Debug, Clone)]
enum RequestedBreakpoint {
    Source { path: String, line: u64 },
    Function(String),
    Instruction(String),
}

pub struct LldbBackend {
    process: Child,
    stdin: ChildStdin,
    reader: Option<JoinHandle<()>>,
    pending: Arc<Mutex<PendingMap>>,
    state: Arc<(Mutex<AdapterState>, Condvar)>,
    next_seq: u32,
    executable: Option<String>,
    requested_breakpoints: Vec<RequestedBreakpoint>,
    resolved_breakpoints: Vec<Breakpoint>,
    capabilities: Option<Capabilities>,
    launched: bool,
    status: DbgBackendStatus,
}

impl LldbBackend {
    pub fn new() -> Self {
        Self::spawn("lldb-dap").unwrap_or_else(|error| {
            panic!("Unable to run `lldb-dap`. Please make sure it is on PATH: {error}")
        })
    }

    fn spawn(program: &str) -> Result<Self, BackendError> {
        let mut process = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| BackendError::Protocol("lldb-dap stdin was not piped".into()))?;
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| BackendError::Protocol("lldb-dap stdout was not piped".into()))?;

        let pending = Arc::new(Mutex::new(PendingMap::new()));
        let state = Arc::new((Mutex::new(AdapterState::default()), Condvar::new()));
        let reader = spawn_reader(stdout, Arc::clone(&pending), Arc::clone(&state));

        Ok(Self {
            process,
            stdin,
            reader: Some(reader),
            pending,
            state,
            next_seq: 1,
            executable: None,
            requested_breakpoints: Vec::new(),
            resolved_breakpoints: Vec::new(),
            capabilities: None,
            launched: false,
            status: DbgBackendStatus::Active,
        })
    }

    fn initialize(&mut self) -> Result<(), BackendError> {
        let mut request = Initialize::new("lldb-dap");
        request.standard.client_id = Some("lazydbg".into());
        request.standard.client_name = Some("lazydbg".into());
        request.standard.columns_start_at1 = Some(true);
        request.standard.lines_start_at1 = Some(true);
        request.standard.path_format = Some("path".into());
        request.standard.supports_ansi_styling = Some(true);
        request.standard.supports_args_can_be_interpreted_by_shell = Some(false);
        request.standard.supports_invalidated_event = Some(false);
        request.standard.supports_memory_event = Some(false);
        request.standard.supports_memory_references = Some(true);
        request.standard.supports_progress_reporting = Some(false);
        request.standard.supports_run_in_terminal_request = Some(false);
        request.standard.supports_start_debugging_request = Some(false);
        request.standard.supports_variable_paging = Some(true);
        request.standard.supports_variable_type = Some(true);
        self.capabilities = Some(self.send(request)?);
        Ok(())
    }

    fn use_seq(&mut self) -> u32 {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.checked_add(1).unwrap_or(1);
        seq
    }

    fn begin_request<C: DapRequest>(
        &mut self,
        arguments: C,
    ) -> Result<PendingRequest<C>, BackendError> {
        let seq = self.use_seq();
        let request = RequestEnvelope {
            seq,
            type_: "request",
            command: C::COMMAND,
            arguments: C::INCLUDE_ARGUMENTS.then_some(&arguments),
        };
        let body = serde_json::to_vec(&request)?;
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(seq, tx);

        debug!(seq, command = C::COMMAND, "sending DAP request");
        let write_result = write_frame(&mut self.stdin, &body);
        if write_result.is_err() {
            self.pending.lock().unwrap().remove(&seq);
        }
        write_result?;
        Ok(PendingRequest {
            receiver: rx,
            request: std::marker::PhantomData,
        })
    }

    fn finish_response<C: DapRequest>(
        &self,
        pending: PendingRequest<C>,
    ) -> Result<C::Response, BackendError> {
        let response =
            pending
                .receiver
                .recv_timeout(RESPONSE_TIMEOUT)
                .map_err(|error| match error {
                    mpsc::RecvTimeoutError::Timeout => BackendError::Protocol(format!(
                        "timed out waiting for `{}` response",
                        C::COMMAND
                    )),
                    mpsc::RecvTimeoutError::Disconnected => {
                        BackendError::Protocol("lldb-dap reader stopped".into())
                    }
                })?;
        if response.command != C::COMMAND {
            return Err(BackendError::Protocol(format!(
                "response command was `{}`, expected `{}`",
                response.command,
                C::COMMAND
            )));
        }
        if !response.success {
            let detail = response
                .message
                .or_else(|| response.body.map(|body| body.to_string()))
                .unwrap_or_else(|| format!("`{}` request failed", C::COMMAND));
            return Err(BackendError::Protocol(detail));
        }

        serde_json::from_value(response.body.unwrap_or(Value::Null)).map_err(Into::into)
    }

    fn send<C: DapRequest>(&mut self, arguments: C) -> Result<C::Response, BackendError> {
        let pending = self.begin_request(arguments)?;
        self.finish_response(pending)
    }

    fn wait_until_initialized(&self) -> Result<(), BackendError> {
        let (lock, ready) = &*self.state;
        let state = lock.lock().unwrap();
        let (state, timeout) = ready
            .wait_timeout_while(state, INITIALIZED_TIMEOUT, |state| !state.initialized)
            .unwrap();
        if state.initialized {
            Ok(())
        } else if state.terminated {
            Err(BackendError::Protocol(
                "lldb-dap terminated before initialization completed".into(),
            ))
        } else if timeout.timed_out() {
            Err(BackendError::Protocol(
                "timed out waiting for lldb-dap's `initialized` event".into(),
            ))
        } else {
            Err(BackendError::Protocol(
                "lldb-dap initialization was interrupted".into(),
            ))
        }
    }

    fn launch(&mut self) -> Result<(), BackendError> {
        self.launch_with_args(None, false)
    }

    fn launch_with_args(
        &mut self,
        args: Option<Vec<String>>,
        stop_on_entry: bool,
    ) -> Result<(), BackendError> {
        let program = self
            .executable
            .clone()
            .ok_or_else(|| BackendError::Protocol("no executable has been opened".into()))?;
        {
            let (state, _) = &*self.state;
            let mut state = state.lock().unwrap();
            state.initialized = false;
            state.stopped_thread = None;
            state.terminated = false;
        }

        let request = Launch {
            configuration: Configuration {
                program: Some(program),
                stop_on_entry: Some(stop_on_entry),
                ..Default::default()
            },
            args,
            ..Default::default()
        };
        let launch = self.begin_request(request)?;
        self.wait_until_initialized()?;
        self.finish_response(launch)?;
        self.apply_all_breakpoints()?;

        self.send(ConfigurationDone(serde_json::Map::new()))?;
        self.launched = true;
        Ok(())
    }

    fn apply_all_breakpoints(&mut self) -> Result<(), BackendError> {
        self.resolved_breakpoints.clear();

        let mut sources: BTreeMap<String, Vec<SourceBreakpoint>> = BTreeMap::new();
        let mut functions = Vec::new();
        let mut instructions = Vec::new();
        for breakpoint in &self.requested_breakpoints {
            match breakpoint {
                RequestedBreakpoint::Source { path, line } => {
                    sources
                        .entry(path.clone())
                        .or_default()
                        .push(SourceBreakpoint {
                            column: None,
                            condition: None,
                            hit_condition: None,
                            line: *line,
                            log_message: None,
                            mode: None,
                        });
                }
                RequestedBreakpoint::Function(name) => functions.push(FunctionBreakpoint {
                    condition: None,
                    hit_condition: None,
                    name: name.clone(),
                }),
                RequestedBreakpoint::Instruction(reference) => {
                    instructions.push(InstructionBreakpoint {
                        condition: None,
                        hit_condition: None,
                        instruction_reference: reference.clone(),
                        mode: None,
                        offset: None,
                    });
                }
            }
        }

        for (path, breakpoints) in sources {
            let request = SetBreakpoints {
                breakpoints,
                lines: Vec::new(),
                source: Source {
                    path: Some(path),
                    ..Default::default()
                },
                source_modified: None,
            };
            let body: SetBreakpointsResponseBody = self.send(request)?;
            self.resolved_breakpoints.extend(body.breakpoints);
        }

        if !functions.is_empty() {
            let body: SetFunctionBreakpointsResponseBody = self.send(SetFunctionBreakpoints {
                breakpoints: functions,
            })?;
            self.resolved_breakpoints.extend(body.breakpoints);
        }

        if !instructions.is_empty() {
            let body: SetInstructionBreakpointsResponseBody =
                self.send(SetInstructionBreakpoints {
                    breakpoints: instructions,
                })?;
            self.resolved_breakpoints.extend(body.breakpoints);
        }

        Ok(())
    }

    fn selected_thread(&mut self) -> Result<i32, BackendError> {
        let stopped = {
            let (state, _) = &*self.state;
            state.lock().unwrap().stopped_thread
        };
        if let Some(thread) = stopped {
            return Ok(thread);
        }

        let threads: ThreadsResponseBody = self.send(Threads)?;
        threads
            .threads
            .first()
            .map(|thread| thread.id)
            .ok_or_else(|| BackendError::Protocol("the debuggee has no threads".into()))
    }

    fn terminate_adapter(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        self.reader.take();
        self.status = DbgBackendStatus::Killed;
    }
}

impl DbgBackend for LldbBackend {
    fn init(&mut self) {
        if let Err(error) = self.initialize() {
            error!(%error, "failed to initialize lldb-dap");
            self.terminate_adapter();
        }
    }

    fn kill(&mut self) {
        if self.status == DbgBackendStatus::Killed {
            return;
        }
        self.status = DbgBackendStatus::Waiting;
        let request = Disconnect {
            terminate_debuggee: Some(true),
            ..Default::default()
        };
        let disconnect = self.send(request);
        if let Err(error) = disconnect {
            debug!(%error, "lldb-dap closed without acknowledging disconnect");
        }
        self.terminate_adapter();
    }

    fn status(&mut self) -> DbgBackendStatus {
        match self.process.try_wait() {
            Ok(Some(_)) => {
                self.status = DbgBackendStatus::Killed;
                self.status
            }
            Ok(None) => self.status,
            Err(error) => {
                error!(%error, "failed to poll lldb-dap process");
                self.status
            }
        }
    }

    fn open_file(&mut self, path: String) {
        self.executable = Some(path);
    }

    fn load_symbols(&mut self, path: String) {
        // lldb-dap creates the target and loads its symbols as part of `launch`.
        self.executable = Some(path);
    }

    fn breakpoints(&mut self) {
        for breakpoint in &self.resolved_breakpoints {
            info!(
                id = ?breakpoint.id,
                verified = breakpoint.verified,
                line = ?breakpoint.line,
                message = ?breakpoint.message,
                "LLDB breakpoint"
            );
        }
    }

    fn set_breakpoint(&mut self, breakpoint: String) {
        let breakpoint = parse_breakpoint(&breakpoint);
        self.requested_breakpoints.push(breakpoint);
        if self.launched
            && let Err(error) = self.apply_all_breakpoints()
        {
            error!(%error, "failed to update LLDB breakpoints");
        }
    }

    fn run(&mut self) {
        match self.launch() {
            Ok(()) => info!("launched debuggee with lldb-dap"),
            Err(error) => error!(%error, "failed to launch debuggee with lldb-dap"),
        }
    }

    fn frames(&mut self) -> Result<Vec<Box<dyn DbgFrame>>, BackendError> {
        let thread_id = self.selected_thread()?;
        let request = StackTrace {
            format: None,
            levels: None,
            start_frame: None,
            thread_id,
        };
        let response: StackTraceResponseBody = self.send(request)?;
        Ok(response
            .stack_frames
            .into_iter()
            .enumerate()
            .map(|(level, frame)| Box::new(LldbFrame { level, frame }) as Box<dyn DbgFrame>)
            .collect())
    }
}

impl Drop for LldbBackend {
    fn drop(&mut self) {
        self.terminate_adapter();
    }
}

#[derive(Clone)]
struct LldbFrame {
    level: usize,
    frame: StackFrame,
}

impl DbgFrame for LldbFrame {
    fn level(&self) -> Option<String> {
        Some(self.level.to_string())
    }

    fn addr(&self) -> Option<String> {
        self.frame.instruction_pointer_reference.clone()
    }

    fn func(&self) -> Option<String> {
        Some(self.frame.name.clone())
    }

    fn file(&self) -> Option<String> {
        self.frame
            .source
            .as_ref()
            .and_then(|source| source.path.clone().or_else(|| source.name.clone()))
    }

    fn line(&self) -> Option<String> {
        (self.frame.line != 0).then(|| self.frame.line.to_string())
    }

    fn clone_box(&self) -> Box<dyn DbgFrame> {
        Box::new(self.clone())
    }
}

fn parse_breakpoint(value: &str) -> RequestedBreakpoint {
    if let Some(reference) = value.strip_prefix('*') {
        return RequestedBreakpoint::Instruction(reference.to_owned());
    }
    if value.starts_with("0x") {
        return RequestedBreakpoint::Instruction(value.to_owned());
    }
    if let Some((path, line)) = value.rsplit_once(':')
        && !path.is_empty()
        && let Ok(line) = line.parse()
    {
        return RequestedBreakpoint::Source {
            path: path.to_owned(),
            line,
        };
    }
    RequestedBreakpoint::Function(value.to_owned())
}

fn spawn_reader(
    stdout: ChildStdout,
    pending: Arc<Mutex<PendingMap>>,
    state: Arc<(Mutex<AdapterState>, Condvar)>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let body = match read_frame(&mut reader) {
                Ok(Some(body)) => body,
                Ok(None) => break,
                Err(error) => {
                    error!(%error, "failed to read lldb-dap message");
                    break;
                }
            };
            let message: Value = match serde_json::from_slice(&body) {
                Ok(message) => message,
                Err(error) => {
                    error!(%error, "failed to decode lldb-dap message");
                    continue;
                }
            };
            match message.get("type").and_then(Value::as_str) {
                Some("response") => match serde_json::from_value::<ResponseEnvelope>(message) {
                    Ok(response) => {
                        let request_seq = response.request_seq;
                        if let Some(sender) = pending.lock().unwrap().remove(&request_seq) {
                            let _ = sender.send(response);
                        } else {
                            warn!(request_seq, "received response for unknown DAP request");
                        }
                    }
                    Err(error) => error!(%error, "invalid lldb-dap response"),
                },
                Some("event") => handle_event(message, &state),
                Some("request") => warn!(message = %message, "unsupported reverse DAP request"),
                other => warn!(?other, "lldb-dap sent an unknown message type"),
            }
        }

        pending.lock().unwrap().clear();
        let (state, changed) = &*state;
        state.lock().unwrap().terminated = true;
        changed.notify_all();
    })
}

fn handle_event(message: Value, state: &Arc<(Mutex<AdapterState>, Condvar)>) {
    let event = message.get("event").and_then(Value::as_str);
    trace!(?event, "received DAP event");
    match event {
        Some("initialized") => {
            let (state, changed) = &**state;
            state.lock().unwrap().initialized = true;
            changed.notify_all();
        }
        Some("stopped") => {
            if let Some(body) = message.get("body").cloned() {
                match serde_json::from_value::<StoppedEventBody>(body) {
                    Ok(body) => {
                        let (state, _) = &**state;
                        state.lock().unwrap().stopped_thread = body.thread_id;
                    }
                    Err(error) => error!(%error, "invalid lldb-dap stopped event"),
                }
            }
        }
        Some("continued") => {
            let (state, _) = &**state;
            state.lock().unwrap().stopped_thread = None;
        }
        Some("terminated" | "exited") => {
            let (state, changed) = &**state;
            state.lock().unwrap().terminated = true;
            changed.notify_all();
        }
        _ => {}
    }
}

fn write_frame(writer: &mut impl Write, body: &[u8]) -> std::io::Result<()> {
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(body)?;
    writer.flush()
}

fn read_frame(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, BackendError> {
    let mut content_length = None;
    let mut header_bytes = 0;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return if header_bytes == 0 {
                Ok(None)
            } else {
                Err(BackendError::Protocol("EOF in DAP headers".into()))
            };
        }
        header_bytes += read;
        if header_bytes > MAX_HEADER_BYTES {
            return Err(BackendError::Protocol("DAP headers are too large".into()));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        let (name, value) = line
            .trim_end_matches(['\r', '\n'])
            .split_once(':')
            .ok_or_else(|| BackendError::Protocol("malformed DAP header".into()))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            if content_length.is_some() {
                return Err(BackendError::Protocol(
                    "duplicate DAP Content-Length header".into(),
                ));
            }
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| BackendError::Protocol("invalid DAP Content-Length".into()))?,
            );
        }
    }

    let content_length = content_length
        .ok_or_else(|| BackendError::Protocol("missing DAP Content-Length header".into()))?;
    if content_length > MAX_BODY_BYTES {
        return Err(BackendError::Protocol(
            "DAP message body is too large".into(),
        ));
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_supported_breakpoint_forms() {
        assert!(matches!(
            parse_breakpoint("src/main.rs:42"),
            RequestedBreakpoint::Source { path, line } if path == "src/main.rs" && line == 42
        ));
        assert!(matches!(
            parse_breakpoint("main"),
            RequestedBreakpoint::Function(name) if name == "main"
        ));
        assert!(matches!(
            parse_breakpoint("*0x1234"),
            RequestedBreakpoint::Instruction(reference) if reference == "0x1234"
        ));
    }

    #[test]
    fn framing_counts_utf8_bytes_and_reads_multiple_messages() {
        let mut wire = Vec::new();
        write_frame(&mut wire, "{\"text\":\"£\"}".as_bytes()).unwrap();
        write_frame(&mut wire, b"{}").unwrap();
        assert!(wire.starts_with(b"Content-Length: 13\r\n\r\n"));

        let mut reader = Cursor::new(wire);
        assert_eq!(
            read_frame(&mut reader).unwrap().unwrap(),
            "{\"text\":\"£\"}".as_bytes()
        );
        assert_eq!(read_frame(&mut reader).unwrap().unwrap(), b"{}");
        assert!(read_frame(&mut reader).unwrap().is_none());
    }

    #[test]
    #[ignore = "requires lldb-dap on PATH; run explicitly as an adapter smoke test"]
    fn initializes_and_launches_a_real_adapter() {
        let mut backend = LldbBackend::spawn("lldb-dap").unwrap();
        backend.initialize().unwrap();
        backend.executable = Some("/usr/bin/sleep".into());
        backend
            .launch_with_args(Some(vec!["2".into()]), true)
            .unwrap();
        assert!(!backend.frames().unwrap().is_empty());
        backend.kill();
    }
}
