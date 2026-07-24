use crate::{command::MiCommand, value::EmptyReply};
use serde::{Deserialize, Serialize};

/// `-break-after number count`
#[derive(Serialize, Default)]
pub struct BreakAfter {
    pub positional: Vec<String>,
}
impl BreakAfter {
    pub fn new(number: u32, count: u32) -> Self {
        Self {
            positional: vec![number.to_string(), count.to_string()],
        }
    }
}
impl MiCommand for BreakAfter {
    const OP: &'static str = "break-after";
    type Reply = EmptyReply;
}

/// `-break-condition number expr`
#[derive(Serialize, Default)]
pub struct BreakCondition {
    pub positional: Vec<String>,
}
impl BreakCondition {
    pub fn new(number: u32, expr: impl Into<String>) -> Self {
        Self {
            positional: vec![number.to_string(), expr.into()],
        }
    }
}
impl MiCommand for BreakCondition {
    const OP: &'static str = "break-condition";
    type Reply = EmptyReply;
}

/// `-break-delete (breakpoint)+`
#[derive(Serialize, Default)]
pub struct BreakDelete {
    pub positional: Vec<String>,
}
impl BreakDelete {
    pub fn new(numbers: impl IntoIterator<Item = u32>) -> Self {
        Self {
            positional: numbers.into_iter().map(|n| n.to_string()).collect(),
        }
    }
}
impl MiCommand for BreakDelete {
    const OP: &'static str = "break-delete";
    type Reply = EmptyReply;
}

/// `-break-disable (breakpoint)+`
#[derive(Serialize, Default)]
pub struct BreakDisable {
    pub positional: Vec<String>,
}
impl BreakDisable {
    pub fn new(numbers: impl IntoIterator<Item = u32>) -> Self {
        Self {
            positional: numbers.into_iter().map(|n| n.to_string()).collect(),
        }
    }
}
impl MiCommand for BreakDisable {
    const OP: &'static str = "break-disable";
    type Reply = EmptyReply;
}

/// `-break-enable (breakpoint)+`
#[derive(Serialize, Default)]
pub struct BreakEnable {
    pub positional: Vec<String>,
}
impl BreakEnable {
    pub fn new(numbers: impl IntoIterator<Item = u32>) -> Self {
        Self {
            positional: numbers.into_iter().map(|n| n.to_string()).collect(),
        }
    }
}
impl MiCommand for BreakEnable {
    const OP: &'static str = "break-enable";
    type Reply = EmptyReply;
}

/// `-break-info breakpoint` (result shape undocumented -> Value)
#[derive(Serialize, Default)]
pub struct BreakInfo {
    pub positional: String,
}
impl MiCommand for BreakInfo {
    const OP: &'static str = "break-info";
    type Reply = crate::Value;
}

/// `-break-insert [-t] [-h] [-r] [-c condition] [-i ignore-count] [-p thread] [line|addr]`
#[derive(Serialize, Default)]
pub struct BreakInsert {
    #[serde(rename = "t")]
    pub temporary: bool,
    #[serde(rename = "h")]
    pub hardware: bool,
    #[serde(rename = "r")]
    pub regex: bool, // location becomes a regexp pattern when set
    #[serde(rename = "c")]
    pub condition: Option<String>,
    #[serde(rename = "i")]
    pub ignore_count: Option<String>,
    #[serde(rename = "p")]
    pub thread: Option<String>,
    pub positional: Option<String>, // function | file:line | file:func | *addr | regex pattern
}
impl MiCommand for BreakInsert {
    const OP: &'static str = "break-insert";
    type Reply = BreakInsertReply;
}
#[derive(Serialize, Deserialize, Debug)]
pub struct BreakInsertReply {
    pub bkpt: BreakpointInfo,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct BreakpointInfo {
    pub number: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub disp: Option<String>,
    pub enabled: Option<String>,
    pub addr: Option<String>,
    pub func: Option<String>,
    pub file: Option<String>,
    pub line: Option<String>,
    pub times: Option<String>,
}

/// `-break-list`
#[derive(Serialize, Default)]
pub struct BreakList {}
impl MiCommand for BreakList {
    const OP: &'static str = "break-list";
    type Reply = BreakListReply;
}
#[derive(Serialize, Deserialize, Debug)]
pub struct BreakListReply {
    #[serde(rename = "BreakpointTable")]
    pub breakpoint_table: BreakpointTable,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct BreakpointTable {
    pub nr_rows: String,
    pub nr_cols: String,
    pub body: Vec<BreakpointTableRow>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct BreakpointTableRow {
    pub bkpt: BreakpointInfo,
}

/// `-break-watch [-a | -r] expr`
#[derive(Serialize, Default)]
pub struct BreakWatch {
    #[serde(rename = "a")]
    pub access: bool,
    #[serde(rename = "r")]
    pub read: bool,
    pub positional: String,
}
impl MiCommand for BreakWatch {
    const OP: &'static str = "break-watch";
    type Reply = BreakWatchReply;
}
#[derive(Serialize, Deserialize, Debug)]
pub struct BreakWatchReply {
    pub wpt: WatchpointInfo,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct WatchpointInfo {
    pub number: String,
    pub exp: String,
}
