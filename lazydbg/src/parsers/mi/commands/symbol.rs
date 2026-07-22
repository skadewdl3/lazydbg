use crate::command::MiCommand;
use serde::Serialize;

/// `-symbol-info-address symbol`
#[derive(Serialize, Default)]
pub struct SymbolInfoAddress {
    pub positional: String,
}
impl MiCommand for SymbolInfoAddress {
    const OP: &'static str = "symbol-info-address";
    type Reply = crate::Value;
}

/// `-symbol-info-file`
#[derive(Serialize, Default)]
pub struct SymbolInfoFile {}
impl MiCommand for SymbolInfoFile {
    const OP: &'static str = "symbol-info-file";
    type Reply = crate::Value;
}

/// `-symbol-info-function`
#[derive(Serialize, Default)]
pub struct SymbolInfoFunction {}
impl MiCommand for SymbolInfoFunction {
    const OP: &'static str = "symbol-info-function";
    type Reply = crate::Value;
}

/// `-symbol-info-line`
#[derive(Serialize, Default)]
pub struct SymbolInfoLine {}
impl MiCommand for SymbolInfoLine {
    const OP: &'static str = "symbol-info-line";
    type Reply = crate::Value;
}

/// `-symbol-info-symbol addr`
#[derive(Serialize, Default)]
pub struct SymbolInfoSymbol {
    pub positional: String,
}
impl MiCommand for SymbolInfoSymbol {
    const OP: &'static str = "symbol-info-symbol";
    type Reply = crate::Value;
}

/// `-symbol-list-functions`
#[derive(Serialize, Default)]
pub struct SymbolListFunctions {}
impl MiCommand for SymbolListFunctions {
    const OP: &'static str = "symbol-list-functions";
    type Reply = crate::Value;
}

/// `-symbol-list-types`
#[derive(Serialize, Default)]
pub struct SymbolListTypes {}
impl MiCommand for SymbolListTypes {
    const OP: &'static str = "symbol-list-types";
    type Reply = crate::Value;
}

/// `-symbol-list-variables`
#[derive(Serialize, Default)]
pub struct SymbolListVariables {}
impl MiCommand for SymbolListVariables {
    const OP: &'static str = "symbol-list-variables";
    type Reply = crate::Value;
}

/// `-symbol-locate`
#[derive(Serialize, Default)]
pub struct SymbolLocate {}
impl MiCommand for SymbolLocate {
    const OP: &'static str = "symbol-locate";
    type Reply = crate::Value;
}

/// `-symbol-type variable`
#[derive(Serialize, Default)]
pub struct SymbolType {
    pub positional: String,
}
impl MiCommand for SymbolType {
    const OP: &'static str = "symbol-type";
    type Reply = crate::Value;
}
