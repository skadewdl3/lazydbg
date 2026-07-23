use crate::parsers::mi::{command::MiCommand, value::EmptyReply};
use serde::{Deserialize, Serialize};

/// `-environment-cd pathdir`
#[derive(Serialize, Default)]
pub struct EnvironmentCd {
    pub positional: String,
}
impl MiCommand for EnvironmentCd {
    const OP: &'static str = "environment-cd";
    type Reply = EmptyReply;
}

/// `-environment-directory pathdir`
#[derive(Serialize, Default)]
pub struct EnvironmentDirectory {
    pub positional: String,
}
impl MiCommand for EnvironmentDirectory {
    const OP: &'static str = "environment-directory";
    type Reply = EmptyReply;
}

/// `-environment-path (pathdir)+`
#[derive(Serialize, Default)]
pub struct EnvironmentPath {
    pub positional: Vec<String>,
}
impl MiCommand for EnvironmentPath {
    const OP: &'static str = "environment-path";
    type Reply = EmptyReply;
}

/// `-environment-pwd`
#[derive(Serialize, Default)]
pub struct EnvironmentPwd {}
impl MiCommand for EnvironmentPwd {
    const OP: &'static str = "environment-pwd";
    type Reply = EmptyReply;
} // pwd itself arrives as a `~` console stream record, not in ^done results
