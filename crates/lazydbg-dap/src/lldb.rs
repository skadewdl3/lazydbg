//! LLDB-specific DAP request arguments.

#[cfg(feature = "lldb-21")]
mod versioned {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    use crate::{Capabilities, DapRequest, InitializeRequestArguments, request::EmptyResponse};

    pub type Extensions = BTreeMap<String, Value>;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct InitializeArguments {
        #[serde(flatten)]
        pub standard: InitializeRequestArguments,
        #[serde(
            rename = "$__lldb_sourceInitFile",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub source_init_file: Option<bool>,
        #[serde(flatten)]
        pub extensions: Extensions,
    }

    impl InitializeArguments {
        pub fn new(adapter_id: impl Into<String>) -> Self {
            Self {
                standard: InitializeRequestArguments {
                    adapter_id: adapter_id.into(),
                    client_id: None,
                    client_name: None,
                    columns_start_at1: None,
                    lines_start_at1: None,
                    locale: None,
                    path_format: None,
                    supports_ansi_styling: None,
                    supports_args_can_be_interpreted_by_shell: None,
                    supports_invalidated_event: None,
                    supports_memory_event: None,
                    supports_memory_references: None,
                    supports_progress_reporting: None,
                    supports_run_in_terminal_request: None,
                    supports_start_debugging_request: None,
                    supports_variable_paging: None,
                    supports_variable_type: None,
                },
                source_init_file: None,
                extensions: Extensions::new(),
            }
        }
    }

    impl DapRequest for InitializeArguments {
        const COMMAND: &'static str = "initialize";
        type Response = Capabilities;
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Configuration {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub debugger_root: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub enable_auto_variable_summaries: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub enable_synthetic_child_debugging: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub display_extended_backtrace: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stop_on_entry: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub timeout: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub command_escape_prefix: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub custom_frame_format: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub custom_thread_format: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub source_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub source_map: Option<SourceMap>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub pre_init_commands: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub init_commands: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub pre_run_commands: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub post_run_commands: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stop_commands: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub exit_commands: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub terminate_commands: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub program: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub target_triple: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub platform_name: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum SourceMap {
        Map(BTreeMap<String, String>),
        Pairs(Vec<(String, String)>),
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum Environment {
        Map(BTreeMap<String, String>),
        List(Vec<String>),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Console {
        #[serde(rename = "internalConsole")]
        Internal,
        #[serde(rename = "integratedTerminal")]
        IntegratedTerminal,
        #[serde(rename = "externalTerminal")]
        ExternalTerminal,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LaunchArguments {
        #[serde(flatten)]
        pub configuration: Configuration,
        #[serde(rename = "__restart", default, skip_serializing_if = "Option::is_none")]
        pub restart: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub no_debug: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub launch_commands: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub cwd: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub args: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub env: Option<Environment>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub detach_on_error: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub disable_aslr: Option<bool>,
        #[serde(
            rename = "disableSTDIO",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub disable_stdio: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub shell_expand_arguments: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub run_in_terminal: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub console: Option<Console>,
        #[cfg(feature = "lldb-22")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub stdio: Option<Vec<Option<String>>>,
        #[serde(flatten)]
        pub extensions: Extensions,
    }

    impl DapRequest for LaunchArguments {
        const COMMAND: &'static str = "launch";
        type Response = EmptyResponse;
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum ProcessId {
        Number(u64),
        String(String),
    }

    #[cfg(feature = "lldb-22")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DapSession {
        pub target_id: u64,
        pub debugger_id: u64,
    }

    #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AttachArguments {
        #[serde(flatten)]
        pub configuration: Configuration,
        #[serde(rename = "__restart", default, skip_serializing_if = "Option::is_none")]
        pub restart: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub attach_commands: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub pid: Option<ProcessId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub wait_for: Option<bool>,
        #[serde(
            rename = "gdb-remote-port",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub gdb_remote_port: Option<i32>,
        #[serde(
            rename = "gdb-remote-hostname",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        pub gdb_remote_hostname: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub core_file: Option<String>,
        #[cfg(feature = "lldb-22")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub session: Option<DapSession>,
        #[serde(flatten)]
        pub extensions: Extensions,
    }

    impl DapRequest for AttachArguments {
        const COMMAND: &'static str = "attach";
        type Response = EmptyResponse;
    }
}

#[cfg(feature = "lldb-21")]
pub use versioned::*;
