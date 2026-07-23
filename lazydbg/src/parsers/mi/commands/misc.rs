use crate::parsers::mi::{command::MiCommand, value::EmptyReply};
use serde::{Deserialize, Serialize};

/// `-gdb-exit`
#[derive(Serialize, Default)]
pub struct GdbExit {}
impl MiCommand for GdbExit {
    const OP: &'static str = "gdb-exit";
    type Reply = EmptyReply;
}

/// `-gdb-set var=value` (e.g. "$foo=3")
#[derive(Serialize, Default)]
pub struct GdbSet {
    pub positional: String,
}
impl MiCommand for GdbSet {
    const OP: &'static str = "gdb-set";
    type Reply = EmptyReply;
}

/// `-gdb-show variable`
#[derive(Serialize, Default)]
pub struct GdbShow {
    pub positional: String,
}
impl MiCommand for GdbShow {
    const OP: &'static str = "gdb-show";
    type Reply = GdbShowReply;
}
#[derive(Deserialize, Debug)]
pub struct GdbShowReply {
    pub value: String,
}

/// `-gdb-version` (result arrives entirely as `~` console stream records, not `^done` fields)
#[derive(Serialize, Default)]
pub struct GdbVersion {}
impl MiCommand for GdbVersion {
    const OP: &'static str = "gdb-version";
    type Reply = EmptyReply;
}
