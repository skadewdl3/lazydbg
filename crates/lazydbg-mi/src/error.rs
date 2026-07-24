use thiserror::Error;

use serde::{de, ser};

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
    #[error("input to parser in empty")]
    EmptyInput,

    #[error("parse error near {0}")]
    Parse(String),
}
