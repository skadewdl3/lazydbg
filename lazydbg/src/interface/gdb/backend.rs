use lazydbg_mi::{
    MiCommand,
    commands::{
        BreakInsert, BreakList, ExecRun, FileExecFile, FileSymbolFile, GdbExit, StackListFrames,
    },
};
use tracing::{debug, error, info};

use crate::interface::{
    DbgBackend,
    backend::{BackendError, DbgBackendStatus, DbgFrame},
};

use super::transport::MiTransport;

pub struct GdbBackend {
    transport: MiTransport,
}

impl GdbBackend {
    pub fn new() -> Self {
        Self::spawn("gdb").unwrap_or_else(|error| {
            panic!(
                "Unable to run `gdb --interpreter=mi3`. Please make sure gdb is on PATH: {error}"
            )
        })
    }

    fn spawn(program: &str) -> Result<Self, BackendError> {
        Ok(Self {
            transport: MiTransport::spawn(program)?,
        })
    }

    fn send<C: MiCommand>(&self, command: C) -> Result<C::Reply, BackendError> {
        self.transport.send(command)
    }
}

impl DbgBackend for GdbBackend {
    fn init(&mut self) {}

    fn kill(&mut self) {
        if self.status() == DbgBackendStatus::Killed {
            return;
        }
        self.transport.mark_waiting();
        if let Err(error) = self.transport.send(GdbExit {}) {
            debug!(%error, "GDB closed without acknowledging exit");
        }
        self.transport.terminate();
    }

    fn status(&mut self) -> DbgBackendStatus {
        match self.transport.status() {
            Ok(status) => status,
            Err(error) => {
                error!(%error, "failed to poll gdb process");
                DbgBackendStatus::Killed
            }
        }
    }

    fn open_file(&mut self, path: String) {
        log_result(self.send(FileExecFile {
            positional: Some(path),
        }));
    }

    fn load_symbols(&mut self, path: String) {
        log_result(self.send(FileSymbolFile {
            positional: Some(path),
        }));
    }

    fn breakpoints(&mut self) {
        log_result(self.send(BreakList {}));
    }

    fn set_breakpoint(&mut self, breakpoint: String) {
        log_result(self.send(BreakInsert {
            positional: Some(breakpoint),
            ..Default::default()
        }));
    }

    fn run(&mut self) {
        log_result(self.send(ExecRun {}));
    }

    fn frames(&mut self) -> Result<Vec<Box<dyn DbgFrame>>, BackendError> {
        let response = self.send(StackListFrames::default())?;
        info!(?response, "GDB frame information");
        Ok(response
            .stack
            .into_iter()
            .map(|frame| Box::new(frame) as Box<dyn DbgFrame>)
            .collect())
    }
}

fn log_result<T: std::fmt::Debug>(result: Result<T, BackendError>) {
    match result {
        Ok(response) => info!(?response),
        Err(error) => error!(%error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires gdb on PATH; run explicitly as a debugger smoke test"]
    fn loads_an_executable_with_a_real_gdb_process() {
        let mut backend = GdbBackend::spawn("gdb").unwrap();
        backend
            .send(FileExecFile {
                positional: Some("/bin/true".into()),
            })
            .unwrap();
        backend.kill();
    }
}
