//! Request operation names for backend code.
//!
//! The generated schema uses DAP's `*Arguments` terminology. These renamed
//! re-exports present the same types as operations at the transport boundary.

pub use crate::request::ThreadsArguments as Threads;

impl Default for crate::ConfigurationDoneArguments {
    fn default() -> Self {
        Self(serde_json::Map::new())
    }
}
pub use crate::{
    ConfigurationDoneArguments as ConfigurationDone, DisconnectArguments as Disconnect,
    SetBreakpointsArguments as SetBreakpoints,
    SetFunctionBreakpointsArguments as SetFunctionBreakpoints,
    SetInstructionBreakpointsArguments as SetInstructionBreakpoints,
    StackTraceArguments as StackTrace,
};

#[cfg(feature = "lldb-21")]
pub use crate::lldb::{
    AttachArguments as Attach, InitializeArguments as Initialize, LaunchArguments as Launch,
};
