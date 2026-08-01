use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    marker::PhantomData,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use serde::de::DeserializeOwned;

use crate::{
    MiCommand, Record, ResultClass, StreamKind, Value, build_line,
    error::{Error, Result},
    parse_line,
};

type Pending = Arc<Mutex<HashMap<u64, mpsc::Sender<Record>>>>;

#[derive(Debug, Clone)]
pub struct MiResponse<R> {
    pub token: u64,
    pub class: ResultClass,
    pub reply: R,
    pub results: HashMap<String, Value>,
}

impl<R> MiResponse<R> {
    pub fn into_reply(self) -> R {
        self.reply
    }
}

pub struct PendingCommand<R> {
    receiver: Option<mpsc::Receiver<Record>>,
    pending: Pending,
    token: u64,
    command: String,
    response: PhantomData<R>,
}

impl<R: DeserializeOwned> PendingCommand<R> {
    pub fn wait(self) -> Result<R> {
        self.wait_response().map(MiResponse::into_reply)
    }

    pub fn wait_timeout(self, timeout: Duration) -> Result<R> {
        self.wait_response_timeout(timeout)
            .map(MiResponse::into_reply)
    }

    pub fn wait_response(mut self) -> Result<MiResponse<R>> {
        let record = self
            .receiver
            .take()
            .expect("pending command receiver was already consumed")
            .recv()
            .map_err(|_| Error::Disconnected)?;
        decode_response(record, &self.command)
    }

    pub fn wait_response_timeout(mut self, timeout: Duration) -> Result<MiResponse<R>> {
        let record = self
            .receiver
            .take()
            .expect("pending command receiver was already consumed")
            .recv_timeout(timeout)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => Error::Timeout(timeout),
                mpsc::RecvTimeoutError::Disconnected => Error::Disconnected,
            })?;
        decode_response(record, &self.command)
    }
}

impl<R> Drop for PendingCommand<R> {
    fn drop(&mut self) {
        self.pending
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .remove(&self.token);
    }
}

#[derive(Debug)]
pub enum Incoming {
    Record(Record),
    ReaderError(Error),
}

pub struct Client<W> {
    writer: Arc<Mutex<W>>,
    pending: Pending,
    next_token: Arc<AtomicU64>,
}

impl<W> Clone for Client<W> {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
            pending: Arc::clone(&self.pending),
            next_token: Arc::clone(&self.next_token),
        }
    }
}

impl<W: Write + Send + 'static> Client<W> {
    pub fn spawn<R: Read + Send + 'static>(
        reader: R,
        writer: W,
    ) -> (Self, mpsc::Receiver<Incoming>) {
        let pending = Pending::default();
        let (incoming_tx, incoming_rx) = mpsc::channel();
        let reader_pending = Arc::clone(&pending);
        thread::spawn(move || {
            read_loop(BufReader::new(reader), &reader_pending, &incoming_tx);
            reader_pending
                .lock()
                .unwrap_or_else(|lock| lock.into_inner())
                .clear();
        });

        (
            Self {
                writer: Arc::new(Mutex::new(writer)),
                pending,
                next_token: Arc::new(AtomicU64::new(0)),
            },
            incoming_rx,
        )
    }

    pub fn send<C: MiCommand>(&self, command: &C) -> Result<C::Reply> {
        self.begin(command)?.wait()
    }

    pub fn send_timeout<C: MiCommand>(&self, command: &C, timeout: Duration) -> Result<C::Reply> {
        self.begin(command)?.wait_timeout(timeout)
    }

    pub fn send_response<C: MiCommand>(&self, command: &C) -> Result<MiResponse<C::Reply>> {
        self.begin(command)?.wait_response()
    }

    pub fn send_response_timeout<C: MiCommand>(
        &self,
        command: &C,
        timeout: Duration,
    ) -> Result<MiResponse<C::Reply>> {
        self.begin(command)?.wait_response_timeout(timeout)
    }

    pub fn begin<C: MiCommand>(&self, command: &C) -> Result<PendingCommand<C::Reply>> {
        let token = self.use_token()?;
        let command_line = build_line(command, token)?;
        let (sender, receiver) = mpsc::channel();
        self.pending
            .lock()
            .unwrap_or_else(|lock| lock.into_inner())
            .insert(token, sender);

        let write_result = (|| -> std::io::Result<()> {
            let mut writer = self.writer.lock().unwrap_or_else(|lock| lock.into_inner());
            writer.write_all(command_line.as_bytes())?;
            writer.flush()
        })();
        if let Err(error) = write_result {
            self.pending
                .lock()
                .unwrap_or_else(|lock| lock.into_inner())
                .remove(&token);
            return Err(error.into());
        }

        Ok(PendingCommand {
            receiver: Some(receiver),
            pending: Arc::clone(&self.pending),
            token,
            command: C::OP.to_owned(),
            response: PhantomData,
        })
    }

    fn use_token(&self) -> Result<u64> {
        self.next_token
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |token| {
                token.checked_add(1)
            })
            .map_err(|_| Error::TokenExhausted)
    }
}

fn read_loop(mut reader: impl BufRead, pending: &Pending, incoming: &mpsc::Sender<Incoming>) {
    let mut line = String::new();
    loop {
        line.clear();
        let result = (|| -> Result<bool> {
            if reader.read_line(&mut line)? == 0 {
                return Ok(false);
            }
            let record = parse_record_or_target_output(&line)?;
            if let Record::Result {
                token: Some(token), ..
            } = &record
            {
                let sender = pending
                    .lock()
                    .unwrap_or_else(|lock| lock.into_inner())
                    .remove(token);
                if let Some(sender) = sender {
                    let _ = sender.send(record);
                }
            } else {
                incoming
                    .send(Incoming::Record(record))
                    .map_err(|_| Error::Disconnected)?;
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

fn parse_record_or_target_output(line: &str) -> Result<Record> {
    if looks_like_mi_control_record(line) {
        parse_line(line).map_err(Into::into)
    } else {
        Ok(Record::Stream {
            kind: StreamKind::Target,
            text: line.trim_end_matches(['\r', '\n']).to_owned(),
        })
    }
}

fn looks_like_mi_control_record(line: &str) -> bool {
    let line = line.trim_end_matches(['\r', '\n']);
    if line == "(gdb)" {
        return true;
    }
    if matches!(line.as_bytes(), [b'~' | b'@' | b'&', b'"', ..]) {
        return true;
    }
    matches!(
        line.trim_start_matches(|character: char| character.is_ascii_digit())
            .chars()
            .next(),
        Some('^' | '*' | '+' | '=')
    )
}

fn decode_response<R: DeserializeOwned>(record: Record, command: &str) -> Result<MiResponse<R>> {
    let Record::Result {
        token: Some(token),
        class,
        results,
    } = record
    else {
        return Err(Error::UnexpectedRecord(record));
    };
    if class == ResultClass::Error {
        let message = match results.get("msg") {
            Some(Value::Str(message)) => message.clone(),
            _ => "command failed".into(),
        };
        return Err(Error::Command {
            command: command.to_owned(),
            message,
            results,
        });
    }
    let reply = R::deserialize(Value::Tuple(results.clone()))?;
    Ok(MiResponse {
        token,
        class,
        reply,
        results,
    })
}
