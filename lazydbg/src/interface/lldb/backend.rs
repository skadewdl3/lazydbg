use std::collections::BTreeMap;

use lazydbg_dap::{
    Breakpoint, Capabilities, FunctionBreakpoint, InstructionBreakpoint,
    SetBreakpointsResponseBody, SetFunctionBreakpointsResponseBody,
    SetInstructionBreakpointsResponseBody, Source, SourceBreakpoint, StackTraceResponseBody,
    ThreadsResponseBody,
    lldb::Configuration,
    requests::{
        ConfigurationDone, Disconnect, Initialize, Launch, SetBreakpoints, SetFunctionBreakpoints,
        SetInstructionBreakpoints, StackTrace, Threads,
    },
};
use tracing::{debug, error, info};

use crate::interface::{
    DbgBackend,
    backend::{BackendError, DbgBackendStatus, DbgFrame},
};

use super::{
    breakpoint::{RequestedBreakpoint, parse as parse_breakpoint},
    frame::LldbFrame,
    transport::{DapTransport, RESPONSE_TIMEOUT},
};

pub struct LldbBackend {
    transport: DapTransport,
    executable: Option<String>,
    requested_breakpoints: Vec<RequestedBreakpoint>,
    resolved_breakpoints: Vec<Breakpoint>,
    capabilities: Option<Capabilities>,
    launched: bool,
}

impl LldbBackend {
    pub fn new() -> Self {
        Self::spawn("lldb-dap").unwrap_or_else(|error| {
            panic!("Unable to run `lldb-dap`. Please make sure it is on PATH: {error}")
        })
    }

    fn spawn(program: &str) -> Result<Self, BackendError> {
        Ok(Self {
            transport: DapTransport::spawn(program)?,
            executable: None,
            requested_breakpoints: Vec::new(),
            resolved_breakpoints: Vec::new(),
            capabilities: None,
            launched: false,
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
        self.capabilities = Some(self.transport.send(request)?);
        Ok(())
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
        self.transport.reset_session();
        let request = Launch {
            configuration: Configuration {
                program: Some(program),
                stop_on_entry: Some(stop_on_entry),
                ..Default::default()
            },
            args,
            ..Default::default()
        };
        let launch = self.transport.begin(request)?;
        self.transport.wait_until_initialized()?;
        launch.wait_timeout(RESPONSE_TIMEOUT)?;
        self.apply_all_breakpoints()?;
        self.transport.send(ConfigurationDone::default())?;
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
            let body: SetBreakpointsResponseBody = self.transport.send(SetBreakpoints {
                breakpoints,
                lines: Vec::new(),
                source: Source {
                    path: Some(path),
                    ..Default::default()
                },
                source_modified: None,
            })?;
            self.resolved_breakpoints.extend(body.breakpoints);
        }
        if !functions.is_empty() {
            let body: SetFunctionBreakpointsResponseBody =
                self.transport.send(SetFunctionBreakpoints {
                    breakpoints: functions,
                })?;
            self.resolved_breakpoints.extend(body.breakpoints);
        }
        if !instructions.is_empty() {
            let body: SetInstructionBreakpointsResponseBody =
                self.transport.send(SetInstructionBreakpoints {
                    breakpoints: instructions,
                })?;
            self.resolved_breakpoints.extend(body.breakpoints);
        }
        Ok(())
    }

    fn selected_thread(&mut self) -> Result<i32, BackendError> {
        if let Some(thread) = self.transport.stopped_thread() {
            return Ok(thread);
        }
        let threads: ThreadsResponseBody = self.transport.send(Threads)?;
        threads
            .threads
            .first()
            .map(|thread| thread.id)
            .ok_or_else(|| BackendError::Protocol("the debuggee has no threads".into()))
    }
}

impl DbgBackend for LldbBackend {
    fn init(&mut self) {
        if let Err(error) = self.initialize() {
            error!(%error, "failed to initialize lldb-dap");
            self.transport.terminate();
        }
    }

    fn kill(&mut self) {
        if self.status() == DbgBackendStatus::Killed {
            return;
        }
        self.transport.mark_waiting();
        let request = Disconnect {
            terminate_debuggee: Some(true),
            ..Default::default()
        };
        if let Err(error) = self.transport.send(request) {
            debug!(%error, "lldb-dap closed without acknowledging disconnect");
        }
        self.transport.terminate();
    }

    fn status(&mut self) -> DbgBackendStatus {
        match self.transport.status() {
            Ok(status) => status,
            Err(error) => {
                error!(%error, "failed to poll lldb-dap process");
                DbgBackendStatus::Killed
            }
        }
    }

    fn open_file(&mut self, path: String) {
        self.executable = Some(path);
    }

    fn load_symbols(&mut self, path: String) {
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
        self.requested_breakpoints
            .push(parse_breakpoint(&breakpoint));
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
        let response: StackTraceResponseBody = self.transport.send(StackTrace {
            format: None,
            levels: None,
            start_frame: None,
            thread_id,
        })?;
        Ok(response
            .stack_frames
            .into_iter()
            .enumerate()
            .map(|(level, frame)| Box::new(LldbFrame::new(level, frame)) as Box<dyn DbgFrame>)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires lldb-dap on PATH; run explicitly as an adapter smoke test"]
    fn initializes_launches_and_reads_frames_from_a_real_adapter() {
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
