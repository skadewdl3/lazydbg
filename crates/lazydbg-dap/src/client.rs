use std::{
    collections::HashMap,
    io::{Read, Write},
    marker::PhantomData,
    num::NonZeroU32,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    Response, RunInTerminalRequest, RunInTerminalResponseBody, StartDebuggingRequest,
    codec::{DEFAULT_MAX_CONTENT_LENGTH, FrameReader},
    error::{Error, Result},
    protocol::{DapResponse, Extensible, Incoming, decode_event, decode_reverse_request},
    request::DapRequest,
};

type Pending = Arc<Mutex<HashMap<u32, mpsc::Sender<Extensible<Response>>>>>;

pub struct PendingRequest<R> {
    receiver: Option<mpsc::Receiver<Extensible<Response>>>,
    pending: Pending,
    request_seq: u32,
    command: String,
    response: PhantomData<R>,
}

impl<R: DeserializeOwned> PendingRequest<R> {
    pub fn wait(self) -> Result<R> {
        self.wait_response().map(DapResponse::into_body)
    }

    pub fn wait_timeout(self, timeout: Duration) -> Result<R> {
        self.wait_response_timeout(timeout)
            .map(DapResponse::into_body)
    }

    pub fn wait_response(mut self) -> Result<DapResponse<R>> {
        let response = self
            .receiver
            .take()
            .expect("pending request receiver was already consumed")
            .recv()
            .map_err(|_| Error::Disconnected)?;
        decode_response(response, &self.command)
    }

    pub fn wait_response_timeout(mut self, timeout: Duration) -> Result<DapResponse<R>> {
        let response = self
            .receiver
            .take()
            .expect("pending request receiver was already consumed")
            .recv_timeout(timeout)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => Error::Timeout(timeout),
                mpsc::RecvTimeoutError::Disconnected => Error::Disconnected,
            })?;
        decode_response(response, &self.command)
    }
}

impl<R> Drop for PendingRequest<R> {
    fn drop(&mut self) {
        self.pending
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .remove(&self.request_seq);
    }
}

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

    pub fn send<Rq: DapRequest>(&self, arguments: &Rq) -> Result<Rq::Response> {
        self.begin(arguments)?.wait()
    }

    pub fn send_timeout<Rq: DapRequest>(
        &self,
        arguments: &Rq,
        timeout: Duration,
    ) -> Result<Rq::Response> {
        self.begin(arguments)?.wait_timeout(timeout)
    }

    pub fn send_response<Rq: DapRequest>(
        &self,
        arguments: &Rq,
    ) -> Result<DapResponse<Rq::Response>> {
        self.begin(arguments)?.wait_response()
    }

    pub fn send_response_timeout<Rq: DapRequest>(
        &self,
        arguments: &Rq,
        timeout: Duration,
    ) -> Result<DapResponse<Rq::Response>> {
        self.begin(arguments)?.wait_response_timeout(timeout)
    }

    pub fn begin<Rq: DapRequest>(&self, arguments: &Rq) -> Result<PendingRequest<Rq::Response>> {
        let arguments = Rq::INCLUDE_ARGUMENTS.then_some(arguments);
        self.begin_raw(Rq::COMMAND, arguments)
    }

    pub fn send_custom<A: Serialize, R: DeserializeOwned>(
        &self,
        command: &str,
        arguments: &A,
    ) -> Result<R> {
        self.begin_custom(command, arguments)?.wait()
    }

    pub fn send_custom_timeout<A: Serialize, R: DeserializeOwned>(
        &self,
        command: &str,
        arguments: &A,
        timeout: Duration,
    ) -> Result<R> {
        self.begin_custom(command, arguments)?.wait_timeout(timeout)
    }

    pub fn send_custom_response<A: Serialize, R: DeserializeOwned>(
        &self,
        command: &str,
        arguments: &A,
    ) -> Result<DapResponse<R>> {
        self.begin_custom(command, arguments)?.wait_response()
    }

    pub fn send_custom_response_timeout<A: Serialize, R: DeserializeOwned>(
        &self,
        command: &str,
        arguments: &A,
        timeout: Duration,
    ) -> Result<DapResponse<R>> {
        self.begin_custom(command, arguments)?
            .wait_response_timeout(timeout)
    }

    pub fn begin_custom<A: Serialize, R>(
        &self,
        command: &str,
        arguments: &A,
    ) -> Result<PendingRequest<R>> {
        self.begin_raw(command, Some(arguments))
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
        request_seq: i32,
        command: &str,
        body: Option<&B>,
    ) -> Result<()> {
        self.respond_success(request_seq, command, body)
    }

    pub fn respond_error(
        &self,
        request_seq: i32,
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

    fn begin_raw<A: Serialize, R>(
        &self,
        command: &str,
        arguments: Option<&A>,
    ) -> Result<PendingRequest<R>> {
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

        Ok(PendingRequest {
            receiver: Some(receiver),
            pending: Arc::clone(&self.pending),
            request_seq: seq.get(),
            command: command.to_owned(),
            response: PhantomData,
        })
    }

    fn respond_success<B: Serialize>(
        &self,
        request_seq: i32,
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
                    let response = Extensible::<Response>::from_value(raw)?;
                    let request_seq = response.request_seq.get();
                    let sender = pending
                        .lock()
                        .unwrap_or_else(|lock| lock.into_inner())
                        .remove(&request_seq);
                    if let Some(sender) = sender {
                        let _ = sender.send(response);
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

fn decode_response<R: DeserializeOwned>(
    response: Extensible<Response>,
    expected_command: &str,
) -> Result<DapResponse<R>> {
    let (response, raw) = response.into_parts();
    if response.command != expected_command {
        return Err(Error::CommandMismatch {
            expected: expected_command.to_owned(),
            actual: response.command,
        });
    }
    if !response.success {
        return Err(Error::Protocol {
            command: expected_command.to_owned(),
            message: response.message.unwrap_or_else(|| "request failed".into()),
            body: response.body,
        });
    }
    let body = Extensible::from_value(response.body.unwrap_or(Value::Null))?;
    Ok(DapResponse::new(
        response.seq,
        response.request_seq.get(),
        response.command,
        body,
        raw,
    ))
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
    request_seq: i32,
    success: bool,
    command: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a B>,
}
