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
pub use record::{AsyncKind, Record, ResultClass, StreamKind};
pub use value::Value;

use crate::error::ParseError;

/// Parse a single line of GDB/MI stdout. `None` for blank lines / non-MI grammar.
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
