use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};

use crate::*;

pub trait DapRequest: Serialize {
    const COMMAND: &'static str;
    const INCLUDE_ARGUMENTS: bool = true;
    type Response: DeserializeOwned;
}

macro_rules! request {
    ($arguments:ty, $response:ty, $command:literal) => {
        impl DapRequest for $arguments {
            const COMMAND: &'static str = $command;
            type Response = $response;
        }
    };
}

request!(CancelArguments, EmptyResponse, "cancel");
request!(
    RunInTerminalRequestArguments,
    RunInTerminalResponseBody,
    "runInTerminal"
);
request!(
    StartDebuggingRequestArguments,
    EmptyResponse,
    "startDebugging"
);
request!(InitializeRequestArguments, Capabilities, "initialize");
request!(
    ConfigurationDoneArguments,
    EmptyResponse,
    "configurationDone"
);
request!(LaunchRequestArguments, EmptyResponse, "launch");
request!(AttachRequestArguments, EmptyResponse, "attach");
request!(RestartArguments, EmptyResponse, "restart");
request!(DisconnectArguments, EmptyResponse, "disconnect");
request!(TerminateArguments, EmptyResponse, "terminate");
request!(
    BreakpointLocationsArguments,
    BreakpointLocationsResponseBody,
    "breakpointLocations"
);
request!(
    SetBreakpointsArguments,
    SetBreakpointsResponseBody,
    "setBreakpoints"
);
request!(
    SetFunctionBreakpointsArguments,
    SetFunctionBreakpointsResponseBody,
    "setFunctionBreakpoints"
);
request!(
    SetExceptionBreakpointsArguments,
    SetExceptionBreakpointsResponseBody,
    "setExceptionBreakpoints"
);
request!(
    DataBreakpointInfoArguments,
    DataBreakpointInfoResponseBody,
    "dataBreakpointInfo"
);
request!(
    SetDataBreakpointsArguments,
    SetDataBreakpointsResponseBody,
    "setDataBreakpoints"
);
request!(
    SetInstructionBreakpointsArguments,
    SetInstructionBreakpointsResponseBody,
    "setInstructionBreakpoints"
);
request!(ContinueArguments, ContinueResponseBody, "continue");
request!(NextArguments, EmptyResponse, "next");
request!(StepInArguments, EmptyResponse, "stepIn");
request!(StepOutArguments, EmptyResponse, "stepOut");
request!(StepBackArguments, EmptyResponse, "stepBack");
request!(ReverseContinueArguments, EmptyResponse, "reverseContinue");
request!(RestartFrameArguments, EmptyResponse, "restartFrame");
request!(GotoArguments, EmptyResponse, "goto");
request!(PauseArguments, EmptyResponse, "pause");
request!(StackTraceArguments, StackTraceResponseBody, "stackTrace");
request!(ScopesArguments, ScopesResponseBody, "scopes");
request!(VariablesArguments, VariablesResponseBody, "variables");
request!(SetVariableArguments, SetVariableResponseBody, "setVariable");
request!(SourceArguments, SourceResponseBody, "source");
request!(TerminateThreadsArguments, EmptyResponse, "terminateThreads");
request!(ModulesArguments, ModulesResponseBody, "modules");
request!(
    LoadedSourcesArguments,
    LoadedSourcesResponseBody,
    "loadedSources"
);
request!(EvaluateArguments, EvaluateResponseBody, "evaluate");
request!(
    SetExpressionArguments,
    SetExpressionResponseBody,
    "setExpression"
);
request!(
    StepInTargetsArguments,
    StepInTargetsResponseBody,
    "stepInTargets"
);
request!(GotoTargetsArguments, GotoTargetsResponseBody, "gotoTargets");
request!(CompletionsArguments, CompletionsResponseBody, "completions");
request!(
    ExceptionInfoArguments,
    ExceptionInfoResponseBody,
    "exceptionInfo"
);
request!(ReadMemoryArguments, ReadMemoryResponseBody, "readMemory");
request!(WriteMemoryArguments, WriteMemoryResponseBody, "writeMemory");
request!(DisassembleArguments, DisassembleResponseBody, "disassemble");
request!(LocationsArguments, LocationsResponseBody, "locations");

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EmptyResponse;

impl<'de> Deserialize<'de> for EmptyResponse {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(Self)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ThreadsArguments;

impl DapRequest for ThreadsArguments {
    const COMMAND: &'static str = "threads";
    const INCLUDE_ARGUMENTS: bool = false;
    type Response = ThreadsResponseBody;
}
