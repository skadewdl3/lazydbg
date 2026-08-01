use std::{
    collections::HashMap,
    io::{Read, Write},
    num::NonZeroU32,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    RunInTerminalRequest, RunInTerminalResponseBody, StartDebuggingRequest,
    codec::{DEFAULT_MAX_CONTENT_LENGTH, FrameReader},
    error::{Error, Result},
    protocol::{DapResponse, Extensible, Incoming, decode_event, decode_reverse_request},
    request::DapRequest,
};

type Pending = Arc<Mutex<HashMap<u32, mpsc::Sender<Value>>>>;

#[derive(Debug, Clone, Copy)]
pub struct ConnectionConfig {
    pub max_content_length: usize,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_content_length: DEFAULT_MAX_CONTENT_LENGTH,
        }
    }
}

pub struct Client<W> {
    writer: Arc<Mutex<W>>,
    pending: Pending,
    next_seq: Arc<AtomicU32>,
}

impl<W> Clone for Client<W> {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
            pending: Arc::clone(&self.pending),
            next_seq: Arc::clone(&self.next_seq),
        }
    }
}

impl<W: Write + Send + 'static> Client<W> {
    pub fn spawn<R: Read + Send + 'static>(
        reader: R,
        writer: W,
    ) -> (Self, mpsc::Receiver<Incoming>) {
        Self::spawn_with_config(reader, writer, ConnectionConfig::default())
    }

    pub fn spawn_with_config<R: Read + Send + 'static>(
        reader: R,
        writer: W,
        config: ConnectionConfig,
    ) -> (Self, mpsc::Receiver<Incoming>) {
        let pending = Pending::default();
        let (incoming_tx, incoming_rx) = mpsc::channel();
        let reader_pending = Arc::clone(&pending);
        thread::spawn(move || {
            read_loop(reader, config, &reader_pending, &incoming_tx);
            reader_pending
                .lock()
                .unwrap_or_else(|lock| lock.into_inner())
                .clear();
        });

        (
            Self {
                writer: Arc::new(Mutex::new(writer)),
                pending,
                next_seq: Arc::new(AtomicU32::new(1)),
            },
            incoming_rx,
        )
    }

    pub fn send<Rq: DapRequest>(&self, arguments: &Rq) -> Result<DapResponse<Rq::Response>> {
        self.send_request::<Rq, Rq::Response>(arguments, None)
    }

    pub fn send_timeout<Rq: DapRequest>(
        &self,
        arguments: &Rq,
        timeout: Duration,
    ) -> Result<DapResponse<Rq::Response>> {
        self.send_request::<Rq, Rq::Response>(arguments, Some(timeout))
    }

    pub fn send_custom<A: Serialize, R: DeserializeOwned>(
        &self,
        command: &str,
        arguments: &A,
    ) -> Result<DapResponse<R>> {
        self.send_raw(command, Some(arguments), None)
    }

    pub fn send_custom_timeout<A: Serialize, R: DeserializeOwned>(
        &self,
        command: &str,
        arguments: &A,
        timeout: Duration,
    ) -> Result<DapResponse<R>> {
        self.send_raw(command, Some(arguments), Some(timeout))
    }

    pub fn respond_run_in_terminal(
        &self,
        request: &Extensible<RunInTerminalRequest>,
        body: &RunInTerminalResponseBody,
    ) -> Result<()> {
        self.respond_success(request.seq, "runInTerminal", Some(body))
    }

    pub fn respond_start_debugging(
        &self,
        request: &Extensible<StartDebuggingRequest>,
    ) -> Result<()> {
        self.respond_success::<()>(request.seq, "startDebugging", None)
    }

    pub fn respond_custom<B: Serialize>(
        &self,
        request_seq: NonZeroU32,
        command: &str,
        body: Option<&B>,
    ) -> Result<()> {
        self.respond_success(request_seq, command, body)
    }

    pub fn respond_error(
        &self,
        request_seq: NonZeroU32,
        command: &str,
        message: &str,
        body: Option<&Value>,
    ) -> Result<()> {
        let response = OutboundResponse {
            seq: self.use_seq()?,
            kind: "response",
            request_seq,
            success: false,
            command,
            message: Some(message),
            body,
        };
        self.write(&response)
    }

    fn send_request<Rq: DapRequest, R: DeserializeOwned>(
        &self,
        arguments: &Rq,
        timeout: Option<Duration>,
    ) -> Result<DapResponse<R>> {
        let arguments = Rq::INCLUDE_ARGUMENTS.then_some(arguments);
        self.send_raw(Rq::COMMAND, arguments, timeout)
    }

    fn send_raw<A: Serialize, R: DeserializeOwned>(
        &self,
        command: &str,
        arguments: Option<&A>,
        timeout: Option<Duration>,
    ) -> Result<DapResponse<R>> {
        let seq = self.use_seq()?;
        let request = OutboundRequest {
            seq,
            kind: "request",
            command,
            arguments,
        };
        let (sender, receiver) = mpsc::channel();
        self.pending
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .insert(seq.get(), sender);
        if let Err(error) = self.write(&request) {
            self.pending
                .lock()
                .unwrap_or_else(|lock| lock.into_inner())
                .remove(&seq.get());
            return Err(error);
        }

        let raw = match timeout {
            Some(timeout) => receiver.recv_timeout(timeout).map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => Error::Timeout(timeout),
                mpsc::RecvTimeoutError::Disconnected => Error::Disconnected,
            }),
            None => receiver.recv().map_err(|_| Error::Disconnected),
        };
        let raw = match raw {
            Ok(raw) => raw,
            Err(error) => {
                self.pending
                    .lock()
                    .unwrap_or_else(|lock| lock.into_inner())
                    .remove(&seq.get());
                return Err(error);
            }
        };

        let header: ResponseHeader = serde_json::from_value(raw.clone())?;
        if header.command != command {
            return Err(Error::CommandMismatch {
                expected: command.to_owned(),
                actual: header.command,
            });
        }
        if !header.success {
            return Err(Error::Protocol {
                command: command.to_owned(),
                message: header.message.unwrap_or_else(|| "request failed".into()),
                body: header.body,
            });
        }
        let body = header.body.map(Extensible::from_value).transpose()?;
        Ok(DapResponse::new(
            header.seq,
            header.request_seq,
            header.command,
            body,
            raw,
        ))
    }

    fn respond_success<B: Serialize>(
        &self,
        request_seq: NonZeroU32,
        command: &str,
        body: Option<&B>,
    ) -> Result<()> {
        let response = OutboundResponse {
            seq: self.use_seq()?,
            kind: "response",
            request_seq,
            success: true,
            command,
            message: None,
            body,
        };
        self.write(&response)
    }

    fn write<T: Serialize>(&self, message: &T) -> Result<()> {
        let mut writer = self.writer.lock().unwrap_or_else(|lock| lock.into_inner());
        crate::codec::write(&mut *writer, message)
    }

    fn use_seq(&self) -> Result<NonZeroU32> {
        let seq = self
            .next_seq
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |seq| {
                (seq < i32::MAX as u32).then_some(seq + 1)
            })
            .map_err(|_| Error::SequenceExhausted)?;
        NonZeroU32::new(seq).ok_or(Error::SequenceExhausted)
    }
}

fn read_loop<R: Read>(
    reader: R,
    config: ConnectionConfig,
    pending: &Pending,
    incoming: &mpsc::Sender<Incoming>,
) {
    let mut frames = FrameReader::with_max_content_length(reader, config.max_content_length);
    loop {
        let result = (|| -> Result<bool> {
            let Some(frame) = frames.read_frame()? else {
                return Ok(false);
            };
            let raw: Value = serde_json::from_slice(&frame)?;
            let message_type = raw
                .get("type")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::UnknownMessageType(String::new()))?;
            match message_type {
                "response" => {
                    let request_seq = raw
                        .get("request_seq")
                        .and_then(Value::as_u64)
                        .and_then(|seq| u32::try_from(seq).ok())
                        .ok_or_else(|| Error::InvalidHeader("invalid request_seq".into()))?;
                    let sender = pending
                        .lock()
                        .unwrap_or_else(|lock| lock.into_inner())
                        .remove(&request_seq);
                    if let Some(sender) = sender {
                        let _ = sender.send(raw);
                    }
                }
                "event" => incoming
                    .send(Incoming::Event(Box::new(decode_event(raw)?)))
                    .map_err(|_| Error::Disconnected)?,
                "request" => incoming
                    .send(Incoming::ReverseRequest(Box::new(decode_reverse_request(
                        raw,
                    )?)))
                    .map_err(|_| Error::Disconnected)?,
                other => return Err(Error::UnknownMessageType(other.to_owned())),
            }
            Ok(true)
        })();
        match result {
            Ok(true) => {}
            Ok(false) => break,
            Err(error) => {
                let _ = incoming.send(Incoming::ReaderError(error));
                break;
            }
        }
    }
}

#[derive(Serialize)]
struct OutboundRequest<'a, A> {
    seq: NonZeroU32,
    #[serde(rename = "type")]
    kind: &'static str,
    command: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<&'a A>,
}

#[derive(Serialize)]
struct OutboundResponse<'a, B> {
    seq: NonZeroU32,
    #[serde(rename = "type")]
    kind: &'static str,
    request_seq: NonZeroU32,
    success: bool,
    command: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a B>,
}

#[derive(Deserialize)]
struct ResponseHeader {
    seq: u32,
    request_seq: u32,
    success: bool,
    command: String,
    message: Option<String>,
    body: Option<Value>,
}
