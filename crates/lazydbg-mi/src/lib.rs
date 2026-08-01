//! Typed commands, records, serialization, parsing, and client transport for GDB/MI.
//!
//! [`client::Client`] owns command tokens, response correlation, timeouts, and
//! asynchronous record delivery. Consumers send [`MiCommand`] values and receive
//! their associated Rust reply types.

pub mod client;
pub mod command;
#[allow(unused_doc_comments)]
pub mod commands;
pub mod de;
pub mod error;
pub mod parser;
pub mod record;
pub mod ser;
pub mod value;

pub use command::{MiCommand, build_line};
use nom::Finish;
pub use record::{AsyncClass, AsyncKind, Record, ResultClass, StreamKind};
pub use value::Value;

use crate::error::ParseError;

/// Parse a single GDB/MI stdout record.
pub fn parse_line(line: &str) -> Result<Record, ParseError> {
    let line = line.trim_end_matches(['\r', '\n']);
    tracing::debug!("Parsing mi output: {:#?}", line);
    if line.is_empty() {
        return Err(crate::error::ParseError::EmptyInput);
    }

    parser::record(line)
        .finish()
        .map(|(_, record)| record)
        .map_err(|err| ParseError::Parse(err.input.to_string()))
}
