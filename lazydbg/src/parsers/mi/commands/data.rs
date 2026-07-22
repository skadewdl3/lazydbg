use crate::command::MiCommand;
use crate::value::EmptyReply;
use serde::{Deserialize, Serialize};

/// `-data-disassemble [-s start -e end | -f file -l line [-n lines]] -- mode`
#[derive(Serialize, Default)]
pub struct DataDisassemble {
    #[serde(rename = "s")]
    pub start_addr: Option<String>,
    #[serde(rename = "e")]
    pub end_addr: Option<String>,
    #[serde(rename = "f")]
    pub filename: Option<String>,
    #[serde(rename = "l")]
    pub linenum: Option<String>,
    #[serde(rename = "n")]
    pub lines: Option<String>,
    pub positional: String, // mode: "0" (disassembly only) or "1" (mixed source+asm)
}
impl MiCommand for DataDisassemble {
    const OP: &'static str = "data-disassemble";
    const DASH_DASH_BEFORE_POSITIONAL: bool = true;
    type Reply = DataDisassembleReply;
}
#[derive(Deserialize, Debug)]
pub struct DataDisassembleReply {
    pub asm_insns: Vec<AsmInsn>,
}
#[derive(Deserialize, Debug)]
pub struct AsmInsn {
    pub address: Option<String>,
    #[serde(rename = "func-name")]
    pub func_name: Option<String>,
    pub offset: Option<String>,
    pub inst: Option<String>,
    pub line: Option<String>,
    pub file: Option<String>,
    pub line_asm_insn: Option<Vec<AsmInsn>>,
}

/// `-data-evaluate-expression expr`
#[derive(Serialize, Default)]
pub struct DataEvaluateExpression {
    pub positional: String,
}
impl MiCommand for DataEvaluateExpression {
    const OP: &'static str = "data-evaluate-expression";
    type Reply = ValueReply;
}
#[derive(Deserialize, Debug)]
pub struct ValueReply {
    pub value: String,
}

/// `-data-list-changed-registers`
#[derive(Serialize, Default)]
pub struct DataListChangedRegisters {}
impl MiCommand for DataListChangedRegisters {
    const OP: &'static str = "data-list-changed-registers";
    type Reply = ChangedRegistersReply;
}
#[derive(Deserialize, Debug)]
pub struct ChangedRegistersReply {
    #[serde(rename = "changed-registers")]
    pub changed_registers: Vec<String>,
}

/// `-data-list-register-names [(regno)+]`
#[derive(Serialize, Default)]
pub struct DataListRegisterNames {
    pub positional: Vec<String>,
}
impl DataListRegisterNames {
    pub fn all() -> Self {
        Self::default()
    }
    pub fn for_regnos(regnos: impl IntoIterator<Item = u32>) -> Self {
        Self {
            positional: regnos.into_iter().map(|n| n.to_string()).collect(),
        }
    }
}
impl MiCommand for DataListRegisterNames {
    const OP: &'static str = "data-list-register-names";
    type Reply = RegisterNamesReply;
}
#[derive(Deserialize, Debug)]
pub struct RegisterNamesReply {
    #[serde(rename = "register-names")]
    pub register_names: Vec<String>,
}

/// `-data-list-register-values fmt [(regno)*]`
#[derive(Serialize, Default)]
pub struct DataListRegisterValues {
    pub positional: Vec<String>,
}
impl DataListRegisterValues {
    pub fn new(fmt: RegisterFormat, regnos: impl IntoIterator<Item = u32>) -> Self {
        let mut positional = vec![fmt.as_str().to_string()];
        positional.extend(regnos.into_iter().map(|n| n.to_string()));
        Self { positional }
    }
}
impl MiCommand for DataListRegisterValues {
    const OP: &'static str = "data-list-register-values";
    type Reply = RegisterValuesReply;
}
#[derive(Deserialize, Debug)]
pub struct RegisterValuesReply {
    #[serde(rename = "register-values")]
    pub register_values: Vec<RegisterValue>,
}
#[derive(Deserialize, Debug)]
pub struct RegisterValue {
    pub number: String,
    pub value: String,
}

pub enum RegisterFormat {
    Hex,
    Octal,
    Binary,
    Decimal,
    Raw,
    Natural,
}
impl RegisterFormat {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Hex => "x",
            Self::Octal => "o",
            Self::Binary => "t",
            Self::Decimal => "d",
            Self::Raw => "r",
            Self::Natural => "N",
        }
    }
}

/// `-data-read-memory [-o byte-offset] address word-format word-size nr-rows nr-cols [aschar]`
#[derive(Serialize, Default)]
pub struct DataReadMemory {
    #[serde(rename = "o")]
    pub byte_offset: Option<String>,
    pub positional: Vec<String>,
}
impl DataReadMemory {
    pub fn new(
        address: impl Into<String>,
        word_format: char,
        word_size: u32,
        nr_rows: u32,
        nr_cols: u32,
        aschar: Option<char>,
    ) -> Self {
        let mut positional = vec![
            address.into(),
            word_format.to_string(),
            word_size.to_string(),
            nr_rows.to_string(),
            nr_cols.to_string(),
        ];
        if let Some(c) = aschar {
            positional.push(c.to_string());
        }
        Self {
            byte_offset: None,
            positional,
        }
    }
}
impl MiCommand for DataReadMemory {
    const OP: &'static str = "data-read-memory";
    type Reply = DataReadMemoryReply;
}
#[derive(Deserialize, Debug)]
pub struct DataReadMemoryReply {
    pub addr: String,
    #[serde(rename = "nr-bytes")]
    pub nr_bytes: String,
    #[serde(rename = "total-bytes")]
    pub total_bytes: String,
    #[serde(rename = "next-row")]
    pub next_row: String,
    #[serde(rename = "prev-row")]
    pub prev_row: String,
    #[serde(rename = "next-page")]
    pub next_page: String,
    #[serde(rename = "prev-page")]
    pub prev_page: String,
    pub memory: Vec<MemoryRow>,
}
#[derive(Deserialize, Debug)]
pub struct MemoryRow {
    pub addr: String,
    pub data: Vec<String>,
    pub ascii: Option<String>,
}

/// `-display-delete number`
#[derive(Serialize, Default)]
pub struct DisplayDelete {
    pub positional: String,
}
impl MiCommand for DisplayDelete {
    const OP: &'static str = "display-delete";
    type Reply = EmptyReply;
}

/// `-display-disable number`
#[derive(Serialize, Default)]
pub struct DisplayDisable {
    pub positional: String,
}
impl MiCommand for DisplayDisable {
    const OP: &'static str = "display-disable";
    type Reply = EmptyReply;
}

/// `-display-enable number`
#[derive(Serialize, Default)]
pub struct DisplayEnable {
    pub positional: String,
}
impl MiCommand for DisplayEnable {
    const OP: &'static str = "display-enable";
    type Reply = EmptyReply;
}

/// `-display-insert expression`
#[derive(Serialize, Default)]
pub struct DisplayInsert {
    pub positional: String,
}
impl MiCommand for DisplayInsert {
    const OP: &'static str = "display-insert";
    type Reply = crate::Value;
}

/// `-display-list`
#[derive(Serialize, Default)]
pub struct DisplayList {}
impl MiCommand for DisplayList {
    const OP: &'static str = "display-list";
    type Reply = crate::Value;
}
