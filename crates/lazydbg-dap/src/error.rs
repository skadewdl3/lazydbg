use std::{io, time::Duration};

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid DAP header: {0}")]
    InvalidHeader(String),
    #[error("DAP message is missing Content-Length")]
    MissingContentLength,
    #[error("DAP message has more than one Content-Length header")]
    DuplicateContentLength,
    #[error("DAP content length {actual} exceeds the configured limit {limit}")]
    ContentTooLarge { actual: usize, limit: usize },
    #[error("DAP JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("DAP connection closed")]
    Disconnected,
    #[error("DAP request timed out after {0:?}")]
    Timeout(Duration),
    #[error("DAP sequence number exhausted")]
    SequenceExhausted,
    #[error("response command mismatch: expected {expected:?}, received {actual:?}")]
    CommandMismatch { expected: String, actual: String },
    #[error("DAP request {command:?} failed: {message}")]
    Protocol {
        command: String,
        message: String,
        body: Option<Value>,
    },
    #[error("response received for unknown request sequence {0}")]
    UnknownResponse(u32),
    #[error("unsupported DAP message type {0:?}")]
    UnknownMessageType(String),
}

pub type Result<T> = std::result::Result<T, Error>;
