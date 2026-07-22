use crate::command::MiCommand;
use crate::value::EmptyReply;
use serde::{Deserialize, Serialize};

/// `-target-attach pid|file`
#[derive(Serialize, Default)]
pub struct TargetAttach {
    pub positional: String,
}
impl MiCommand for TargetAttach {
    const OP: &'static str = "target-attach";
    type Reply = crate::Value;
}

/// `-target-compare-sections [section]`
#[derive(Serialize, Default)]
pub struct TargetCompareSections {
    pub positional: Option<String>,
}
impl MiCommand for TargetCompareSections {
    const OP: &'static str = "target-compare-sections";
    type Reply = crate::Value;
}

/// `-target-detach`
#[derive(Serialize, Default)]
pub struct TargetDetach {}
impl MiCommand for TargetDetach {
    const OP: &'static str = "target-detach";
    type Reply = EmptyReply;
}

/// `-target-download`
#[derive(Serialize, Default)]
pub struct TargetDownload {}
impl MiCommand for TargetDownload {
    const OP: &'static str = "target-download";
    type Reply = TargetDownloadReply;
}
#[derive(Deserialize, Debug)]
pub struct TargetDownloadReply {
    pub address: String,
    #[serde(rename = "load-size")]
    pub load_size: String,
    #[serde(rename = "transfer-rate")]
    pub transfer_rate: String,
    #[serde(rename = "write-rate")]
    pub write_rate: String,
}

/// `-target-exec-status`
#[derive(Serialize, Default)]
pub struct TargetExecStatus {}
impl MiCommand for TargetExecStatus {
    const OP: &'static str = "target-exec-status";
    type Reply = crate::Value;
}

/// `-target-list-available-targets`
#[derive(Serialize, Default)]
pub struct TargetListAvailableTargets {}
impl MiCommand for TargetListAvailableTargets {
    const OP: &'static str = "target-list-available-targets";
    type Reply = crate::Value;
}

/// `-target-list-current-targets`
#[derive(Serialize, Default)]
pub struct TargetListCurrentTargets {}
impl MiCommand for TargetListCurrentTargets {
    const OP: &'static str = "target-list-current-targets";
    type Reply = crate::Value;
}

/// `-target-list-parameters`
#[derive(Serialize, Default)]
pub struct TargetListParameters {}
impl MiCommand for TargetListParameters {
    const OP: &'static str = "target-list-parameters";
    type Reply = crate::Value;
}

/// `-target-select type parameters...`
#[derive(Serialize, Default)]
pub struct TargetSelect {
    pub positional: Vec<String>,
}
impl TargetSelect {
    pub fn new(
        target_type: impl Into<String>,
        parameters: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut positional = vec![target_type.into()];
        positional.extend(parameters);
        Self { positional }
    }
}
impl MiCommand for TargetSelect {
    const OP: &'static str = "target-select";
    type Reply = TargetSelectReply;
}
#[derive(Deserialize, Debug)]
pub struct TargetSelectReply {
    pub addr: String,
    pub func: String,
    pub args: Vec<crate::Value>,
}
