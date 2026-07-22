use serde::{Serialize, Deserialize};
use crate::command::MiCommand;
use crate::value::EmptyReply;

/// `-stack-info-frame`
#[derive(Serialize, Default)]
pub struct StackInfoFrame {}
impl MiCommand for StackInfoFrame { const OP: &'static str = "stack-info-frame"; type Reply = StackInfoFrameReply; }
#[derive(Deserialize, Debug)]
pub struct StackInfoFrameReply { pub frame: FrameInfo }
#[derive(Deserialize, Debug)]
pub struct FrameInfo {
    pub level: Option<String>,
    pub addr: Option<String>,
    pub func: Option<String>,
    pub file: Option<String>,
    pub line: Option<String>,
    pub args: Option<Vec<crate::Value>>,
}

/// `-stack-info-depth [max-depth]`
#[derive(Serialize, Default)]
pub struct StackInfoDepth { pub positional: Option<String> }
impl StackInfoDepth {
    pub fn new() -> Self { Self::default() }
    pub fn with_max_depth(max_depth: u32) -> Self { Self { positional: Some(max_depth.to_string()) } }
}
impl MiCommand for StackInfoDepth { const OP: &'static str = "stack-info-depth"; type Reply = StackInfoDepthReply; }
#[derive(Deserialize, Debug)]
pub struct StackInfoDepthReply { pub depth: String }

/// `-stack-list-arguments show-values [low-frame high-frame]`
#[derive(Serialize, Default)]
pub struct StackListArguments { pub positional: Vec<String> }
impl StackListArguments {
    pub fn new(show_values: bool, frame_range: Option<(u32, u32)>) -> Self {
        let mut positional = vec![if show_values { "1" } else { "0" }.to_string()];
        if let Some((lo, hi)) = frame_range { positional.push(lo.to_string()); positional.push(hi.to_string()); }
        Self { positional }
    }
}
impl MiCommand for StackListArguments { const OP: &'static str = "stack-list-arguments"; type Reply = StackListArgumentsReply; }
#[derive(Deserialize, Debug)]
pub struct StackListArgumentsReply { #[serde(rename = "stack-args")] pub stack_args: Vec<StackFrameArgs> }
#[derive(Deserialize, Debug)]
pub struct StackFrameArgs { pub frame: FrameArgsEntry }
#[derive(Deserialize, Debug)]
pub struct FrameArgsEntry { pub level: String, pub args: Vec<crate::Value> }

/// `-stack-list-frames [low-frame high-frame]`
#[derive(Serialize, Default)]
pub struct StackListFrames { pub positional: Vec<String> }
impl StackListFrames {
    pub fn all() -> Self { Self::default() }
    pub fn range(low: u32, high: u32) -> Self { Self { positional: vec![low.to_string(), high.to_string()] } }
}
impl MiCommand for StackListFrames { const OP: &'static str = "stack-list-frames"; type Reply = StackListFramesReply; }
#[derive(Deserialize, Debug)]
pub struct StackListFramesReply { pub stack: Vec<FrameInfo> }

/// `-stack-list-locals print-values`
#[derive(Serialize, Default)]
pub struct StackListLocals { pub positional: String }
impl StackListLocals {
    pub fn new(print_values: bool) -> Self { Self { positional: if print_values { "1" } else { "0" }.to_string() } }
}
impl MiCommand for StackListLocals { const OP: &'static str = "stack-list-locals"; type Reply = StackListLocalsReply; }
#[derive(Deserialize, Debug)]
pub struct StackListLocalsReply { pub locals: Vec<crate::Value> }

/// `-stack-select-frame framenum`
#[derive(Serialize, Default)]
pub struct StackSelectFrame { pub positional: String }
impl StackSelectFrame {
    pub fn new(framenum: u32) -> Self { Self { positional: framenum.to_string() } }
}
impl MiCommand for StackSelectFrame { const OP: &'static str = "stack-select-frame"; type Reply = EmptyReply; }
