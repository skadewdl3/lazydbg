//! GDB/MI parsing (nom) and command serialization (serde), no raw strings in app code.
pub mod command;
pub mod commands;
pub mod de;
pub mod parser;
pub mod record;
pub mod ser;
pub mod value;

pub use command::{MiCommand, build_line};
pub use record::{AsyncKind, Record, ResultClass, StreamKind};
pub use value::Value;

/// Parse a single line of GDB/MI stdout. `None` for blank lines / non-MI grammar.
pub fn parse_line(line: &str) -> Option<Record> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.is_empty() {
        return None;
    }
    parser::record(line).ok().map(|(_, rec)| rec)
}
