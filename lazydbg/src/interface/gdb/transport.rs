use std::{
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

use lazydbg_mi::{
    MiCommand, Record,
    client::{Client, Incoming},
};
use tracing::{debug, error, trace, warn};

use crate::interface::backend::{BackendError, DbgBackendStatus};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) struct MiTransport {
    process: Child,
    client: Client<ChildStdin>,
    events: Option<JoinHandle<()>>,
    status: DbgBackendStatus,
}

impl MiTransport {
    pub(super) fn spawn(program: &str) -> Result<Self, BackendError> {
        let mut process = Command::new(program)
            .args(["-q", "--interpreter=mi3"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| BackendError::Protocol("gdb stdin was not piped".into()))?;
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| BackendError::Protocol("gdb stdout was not piped".into()))?;
        let (client, incoming) = Client::spawn(stdout, stdin);
        let events = spawn_event_reader(incoming);

        Ok(Self {
            process,
            client,
            events: Some(events),
            status: DbgBackendStatus::Active,
        })
    }

    pub(super) fn send<C: MiCommand>(&self, command: C) -> Result<C::Reply, BackendError> {
        debug!(command = C::OP, "sending GDB/MI command");
        self.client
            .send_timeout(&command, RESPONSE_TIMEOUT)
            .map_err(Into::into)
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

impl Drop for MiTransport {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn spawn_event_reader(incoming: mpsc::Receiver<Incoming>) -> JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(message) = incoming.recv() {
            match message {
                Incoming::Record(Record::Async {
                    kind,
                    class,
                    results,
                    ..
                }) => trace!(?kind, ?class, ?results, "received GDB/MI async record"),
                Incoming::Record(Record::Stream { kind, text }) => {
                    trace!(?kind, %text, "received GDB/MI stream record")
                }
                Incoming::Record(Record::Prompt) => {}
                Incoming::Record(record @ Record::Result { .. }) => {
                    warn!(?record, "received uncorrelated GDB/MI result")
                }
                Incoming::ReaderError(error) => {
                    error!(%error, "GDB/MI reader stopped");
                    break;
                }
            }
        }
    })
}
