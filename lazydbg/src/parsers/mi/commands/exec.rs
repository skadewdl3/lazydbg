use crate::command::MiCommand;
use crate::value::EmptyReply;
use serde::Serialize;

macro_rules! nullary_exec {
    ($struct_name:ident, $op:literal) => {
        #[derive(Serialize, Default)]
        pub struct $struct_name {}
        impl MiCommand for $struct_name {
            const OP: &'static str = $op;
            type Reply = EmptyReply;
        }
    };
}

/// `-exec-abort`
nullary_exec!(ExecAbort, "exec-abort");

/// `-exec-arguments args`
#[derive(Serialize, Default)]
pub struct ExecArguments {
    pub positional: String,
}
impl MiCommand for ExecArguments {
    const OP: &'static str = "exec-arguments";
    type Reply = EmptyReply;
}

/// `-exec-continue` (async: reply is `^running`, real state change arrives as a later `*stopped` async record)
nullary_exec!(ExecContinue, "exec-continue");
/// `-exec-finish`
nullary_exec!(ExecFinish, "exec-finish");
/// `-exec-interrupt`
nullary_exec!(ExecInterrupt, "exec-interrupt");
/// `-exec-next`
nullary_exec!(ExecNext, "exec-next");
/// `-exec-next-instruction`
nullary_exec!(ExecNextInstruction, "exec-next-instruction");
/// `-exec-return`
nullary_exec!(ExecReturn, "exec-return");
/// `-exec-run`
nullary_exec!(ExecRun, "exec-run");
/// `-exec-show-arguments`
nullary_exec!(ExecShowArguments, "exec-show-arguments");
/// `-exec-step`
nullary_exec!(ExecStep, "exec-step");
/// `-exec-step-instruction`
nullary_exec!(ExecStepInstruction, "exec-step-instruction");

/// `-exec-until [location]`
#[derive(Serialize, Default)]
pub struct ExecUntil {
    pub positional: Option<String>,
}
impl MiCommand for ExecUntil {
    const OP: &'static str = "exec-until";
    type Reply = EmptyReply;
}
