use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use serde_json::Value;

use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Extensible<T> {
    typed: T,
    raw: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DapResponse<T> {
    pub seq: i32,
    pub request_seq: u32,
    pub command: String,
    pub body: Extensible<T>,
    raw: Value,
}

impl<T> DapResponse<T> {
    pub fn raw(&self) -> &Value {
        &self.raw
    }

    pub(crate) fn new(
        seq: i32,
        request_seq: u32,
        command: String,
        body: Extensible<T>,
        raw: Value,
    ) -> Self {
        Self {
            seq,
            request_seq,
            command,
            body,
            raw,
        }
    }

    pub fn into_body(self) -> T {
        self.body.into_typed()
    }
}

impl<T> Extensible<T> {
    pub fn typed(&self) -> &T {
        &self.typed
    }

    pub fn into_typed(self) -> T {
        self.typed
    }

    pub fn raw(&self) -> &Value {
        &self.raw
    }

    pub fn into_parts(self) -> (T, Value) {
        (self.typed, self.raw)
    }
}

impl<T: DeserializeOwned> Extensible<T> {
    pub fn from_value(raw: Value) -> serde_json::Result<Self> {
        let typed = serde_json::from_value(raw.clone())?;
        Ok(Self { typed, raw })
    }
}

impl<T> Deref for Extensible<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.typed
    }
}

impl<T: Serialize> Serialize for Extensible<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut raw = self.raw.clone();
        let typed = serde_json::to_value(&self.typed).map_err(serde::ser::Error::custom)?;
        merge_json(&mut raw, typed);
        raw.serialize(serializer)
    }
}

fn merge_json(raw: &mut Value, typed: Value) {
    match (raw, typed) {
        (Value::Object(raw), Value::Object(typed)) => {
            for (key, value) in typed {
                match raw.get_mut(&key) {
                    Some(raw_value) => merge_json(raw_value, value),
                    None => {
                        raw.insert(key, value);
                    }
                }
            }
        }
        (raw, typed) => *raw = typed,
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Extensible<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Value::deserialize(deserializer)?;
        Self::from_value(raw).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DapEvent {
    Initialized(Extensible<InitializedEvent>),
    Stopped(Extensible<StoppedEvent>),
    Continued(Extensible<ContinuedEvent>),
    Exited(Extensible<ExitedEvent>),
    Terminated(Extensible<TerminatedEvent>),
    Thread(Extensible<ThreadEvent>),
    Output(Extensible<OutputEvent>),
    Breakpoint(Extensible<BreakpointEvent>),
    Module(Extensible<ModuleEvent>),
    LoadedSource(Extensible<LoadedSourceEvent>),
    Process(Extensible<ProcessEvent>),
    Capabilities(Extensible<CapabilitiesEvent>),
    ProgressStart(Extensible<ProgressStartEvent>),
    ProgressUpdate(Extensible<ProgressUpdateEvent>),
    ProgressEnd(Extensible<ProgressEndEvent>),
    Invalidated(Extensible<InvalidatedEvent>),
    Memory(Extensible<MemoryEvent>),
    Unknown(UnknownMessage),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReverseRequest {
    RunInTerminal(Extensible<RunInTerminalRequest>),
    StartDebugging(Extensible<StartDebuggingRequest>),
    Unknown(UnknownMessage),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnknownMessage {
    pub seq: i32,
    pub name: String,
    pub body: Option<Value>,
    pub raw: Value,
}

#[derive(Debug)]
pub enum Incoming {
    Event(Box<DapEvent>),
    ReverseRequest(Box<ReverseRequest>),
    ReaderError(crate::error::Error),
}

pub(crate) fn decode_event(raw: Value) -> serde_json::Result<DapEvent> {
    let event = raw.get("event").and_then(Value::as_str).unwrap_or_default();
    macro_rules! known {
        ($variant:ident, $type:ty) => {
            Ok(DapEvent::$variant(Extensible::<$type>::from_value(raw)?))
        };
    }
    match event {
        "initialized" => known!(Initialized, InitializedEvent),
        "stopped" => known!(Stopped, StoppedEvent),
        "continued" => known!(Continued, ContinuedEvent),
        "exited" => known!(Exited, ExitedEvent),
        "terminated" => known!(Terminated, TerminatedEvent),
        "thread" => known!(Thread, ThreadEvent),
        "output" => known!(Output, OutputEvent),
        "breakpoint" => known!(Breakpoint, BreakpointEvent),
        "module" => known!(Module, ModuleEvent),
        "loadedSource" => known!(LoadedSource, LoadedSourceEvent),
        "process" => known!(Process, ProcessEvent),
        "capabilities" => known!(Capabilities, CapabilitiesEvent),
        "progressStart" => known!(ProgressStart, ProgressStartEvent),
        "progressUpdate" => known!(ProgressUpdate, ProgressUpdateEvent),
        "progressEnd" => known!(ProgressEnd, ProgressEndEvent),
        "invalidated" => known!(Invalidated, InvalidatedEvent),
        "memory" => known!(Memory, MemoryEvent),
        _ => Ok(DapEvent::Unknown(unknown_message(raw, "event")?)),
    }
}

pub(crate) fn decode_reverse_request(raw: Value) -> serde_json::Result<ReverseRequest> {
    let command = raw
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match command {
        "runInTerminal" => Ok(ReverseRequest::RunInTerminal(Extensible::from_value(raw)?)),
        "startDebugging" => Ok(ReverseRequest::StartDebugging(Extensible::from_value(raw)?)),
        _ => Ok(ReverseRequest::Unknown(unknown_message(raw, "command")?)),
    }
}

fn unknown_message(raw: Value, name_field: &str) -> serde_json::Result<UnknownMessage> {
    let seq = raw
        .get("seq")
        .and_then(Value::as_i64)
        .and_then(|seq| i32::try_from(seq).ok())
        .ok_or_else(|| serde_json::Error::io(std::io::Error::other("missing or invalid seq")))?;
    let name = raw
        .get(name_field)
        .and_then(Value::as_str)
        .ok_or_else(|| serde_json::Error::io(std::io::Error::other("missing message name")))?
        .to_owned();
    let body = raw
        .get(if name_field == "event" {
            "body"
        } else {
            "arguments"
        })
        .cloned();
    Ok(UnknownMessage {
        seq,
        name,
        body,
        raw,
    })
}
