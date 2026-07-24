use serde::de::DeserializeOwned;
use std::collections::HashMap;

use crate::{Value, error::DeserializationError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultClass {
    Done,
    Running,
    Connected,
    Error,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncKind {
    Exec,
    Status,
    Notify,
} // * + =

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Console,
    Target,
    Log,
} // ~ @ &

#[derive(Debug, Clone)]
pub enum AsyncClass {
    // Exec (*)
    Running,
    Stopped,

    // Status (+)
    Download,

    // Notify (=)

    // Thread groups
    ThreadGroupAdded,
    ThreadGroupRemoved,
    ThreadGroupStarted,
    ThreadGroupExited,

    // Threads
    ThreadCreated,
    ThreadExited,
    ThreadSelected,

    // Shared libraries
    LibraryLoaded,
    LibraryUnloaded,

    // Breakpoints
    BreakpointCreated,
    BreakpointModified,
    BreakpointDeleted,

    // Tracepoints / tracing
    TraceframeChanged,
    TsvCreated,
    TsvModified,
    TsvDeleted,

    // Process recording
    RecordStarted,
    RecordStopped,

    // Misc
    CmdParamChanged,
    MemoryChanged,
    RegisterChanged,

    // Forward compatibility
    Unknown(Box<str>),
}

#[derive(Debug, Clone)]
pub enum Record {
    Result {
        token: Option<u64>,
        class: ResultClass,
        results: HashMap<String, Value>,
    },
    Async {
        token: Option<u64>,
        kind: AsyncKind,
        class: AsyncClass,
        results: HashMap<String, Value>,
    },
    Stream {
        kind: StreamKind,
        text: String,
    },
    Prompt, // "(gdb)"
}

impl Record {
    /// If this record carries a results map, deserialize it into `T`.
    pub fn parse_results<T: DeserializeOwned>(&self) -> Option<Result<T, DeserializationError>> {
        match self {
            Record::Result { results, .. } | Record::Async { results, .. } => {
                Some(T::deserialize(Value::Tuple(results.clone())))
            }
            _ => None,
        }
    }
}
