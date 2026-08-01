use std::{
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Arc, Condvar, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use lazydbg_dap::{
    DapRequest,
    client::{Client, PendingRequest},
    protocol::{DapEvent, Incoming},
};
use tracing::{debug, error, trace, warn};

use crate::interface::backend::{BackendError, DbgBackendStatus};

pub(super) const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);
const INITIALIZED_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Default)]
struct AdapterState {
    initialized: bool,
    stopped_thread: Option<i32>,
    terminated: bool,
}

pub(super) struct DapTransport {
    process: Child,
    client: Client<ChildStdin>,
    events: Option<JoinHandle<()>>,
    state: Arc<(Mutex<AdapterState>, Condvar)>,
    status: DbgBackendStatus,
}

impl DapTransport {
    pub(super) fn spawn(program: &str) -> Result<Self, BackendError> {
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
        let state = Arc::new((Mutex::new(AdapterState::default()), Condvar::new()));
        let (client, incoming) = Client::spawn(stdout, stdin);
        let events = spawn_event_reader(incoming, Arc::clone(&state));

        Ok(Self {
            process,
            client,
            events: Some(events),
            state,
            status: DbgBackendStatus::Active,
        })
    }

    pub(super) fn send<C: DapRequest>(&self, request: C) -> Result<C::Response, BackendError> {
        debug!(command = C::COMMAND, "sending DAP request");
        self.client
            .send_timeout(&request, RESPONSE_TIMEOUT)
            .map_err(Into::into)
    }

    pub(super) fn begin<C: DapRequest>(
        &self,
        request: C,
    ) -> Result<PendingRequest<C::Response>, BackendError> {
        debug!(command = C::COMMAND, "beginning DAP request");
        self.client.begin(&request).map_err(Into::into)
    }

    pub(super) fn reset_session(&self) {
        let (state, _) = &*self.state;
        let mut state = state.lock().unwrap();
        state.initialized = false;
        state.stopped_thread = None;
        state.terminated = false;
    }

    pub(super) fn wait_until_initialized(&self) -> Result<(), BackendError> {
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

    pub(super) fn stopped_thread(&self) -> Option<i32> {
        let (state, _) = &*self.state;
        state.lock().unwrap().stopped_thread
    }

    pub(super) fn status(&mut self) -> Result<DbgBackendStatus, BackendError> {
        if self.process.try_wait()?.is_some() {
            self.status = DbgBackendStatus::Killed;
        }
        Ok(self.status)
    }

    pub(super) fn mark_waiting(&mut self) {
        self.status = DbgBackendStatus::Waiting;
    }

    pub(super) fn terminate(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
        if let Some(events) = self.events.take() {
            let _ = events.join();
        }
        self.status = DbgBackendStatus::Killed;
    }
}

impl Drop for DapTransport {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn spawn_event_reader(
    incoming: mpsc::Receiver<Incoming>,
    state: Arc<(Mutex<AdapterState>, Condvar)>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(message) = incoming.recv() {
            match message {
                Incoming::Event(event) => handle_event(*event, &state),
                Incoming::ReverseRequest(request) => {
                    warn!(?request, "unsupported reverse DAP request")
                }
                Incoming::ReaderError(error) => {
                    error!(%error, "lldb-dap reader stopped");
                    break;
                }
            }
        }

        let (state, changed) = &*state;
        state.lock().unwrap().terminated = true;
        changed.notify_all();
    })
}

fn handle_event(event: DapEvent, state: &Arc<(Mutex<AdapterState>, Condvar)>) {
    trace!(?event, "received DAP event");
    match event {
        DapEvent::Initialized(_) => {
            let (state, changed) = &**state;
            state.lock().unwrap().initialized = true;
            changed.notify_all();
        }
        DapEvent::Stopped(event) => {
            let (state, _) = &**state;
            state.lock().unwrap().stopped_thread = event.body.thread_id;
        }
        DapEvent::Continued(_) => {
            let (state, _) = &**state;
            state.lock().unwrap().stopped_thread = None;
        }
        DapEvent::Terminated(_) | DapEvent::Exited(_) => {
            let (state, changed) = &**state;
            state.lock().unwrap().terminated = true;
            changed.notify_all();
        }
        _ => {}
    }
}
