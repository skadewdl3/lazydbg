use std::{collections::HashMap, io, time::Duration};

use serde::{de, ser};
use thiserror::Error;

use crate::{Record, Value};

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("GDB/MI parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("GDB/MI command serialization failed: {0}")]
    Serialize(#[from] SerializationError),
    #[error("GDB/MI reply deserialization failed: {0}")]
    Deserialize(#[from] DeserializationError),
    #[error("GDB/MI connection closed")]
    Disconnected,
    #[error("GDB/MI command timed out after {0:?}")]
    Timeout(Duration),
    #[error("GDB/MI token number exhausted")]
    TokenExhausted,
    #[error("GDB/MI command {command:?} failed: {message}")]
    Command {
        command: String,
        message: String,
        results: HashMap<String, Value>,
    },
    #[error("expected a GDB/MI result record, received {0:?}")]
    UnexpectedRecord(Record),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("command must be a struct")]
    InvalidCommandFormat,
    #[error("serialization error: {0}")]
    Serialize(String),
    #[error("{0} not supported as mi arguments")]
    UnsupportedMiArg(String),
}

impl ser::Error for SerializationError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        SerializationError::Serialize(msg.to_string())
    }
}

#[derive(Debug, Error)]
pub enum DeserializationError {
    #[error("expected numeric value, got {0}")]
    ExpectedNumeric(String),
    #[error("expected boolean value, got {0}")]
    ExpectedBool(String),
    #[error("expected boolean value, got {0}")]
    ExpectedString(String),
    #[error("error while parsing integer")]
    ParseInt,
    #[error("deserialization error: {0}")]
    Deserialize(String),
}

impl de::Error for DeserializationError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        DeserializationError::Deserialize(msg.to_string())
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("input to parser is empty")]
    EmptyInput,

    #[error("parse error near {0}")]
    Parse(String),
}
