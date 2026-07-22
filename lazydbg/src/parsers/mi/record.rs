use crate::value::Value;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

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
pub enum Record {
    Result {
        token: Option<u64>,
        class: ResultClass,
        results: HashMap<String, Value>,
    },
    Async {
        token: Option<u64>,
        kind: AsyncKind,
        class: String,
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
    pub fn parse_results<T: DeserializeOwned>(&self) -> Option<Result<T, crate::de::Error>> {
        match self {
            Record::Result { results, .. } | Record::Async { results, .. } => {
                Some(T::deserialize(Value::Tuple(results.clone())))
            }
            _ => None,
        }
    }
}
