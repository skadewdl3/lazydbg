use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Read, Write},
    sync::mpsc,
    thread,
    time::Duration,
};

use lazydbg_mi::{
    MiCommand, Record,
    client::{Client, Incoming},
    error::Error,
    record::AsyncClass,
};
use serde::{Deserialize, Serialize};

struct PipeReader {
    receiver: mpsc::Receiver<Vec<u8>>,
    buffered: VecDeque<u8>,
}

impl Read for PipeReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        while self.buffered.is_empty() {
            match self.receiver.recv() {
                Ok(bytes) => self.buffered.extend(bytes),
                Err(_) => return Ok(0),
            }
        }
        let count = output.len().min(self.buffered.len());
        for byte in &mut output[..count] {
            *byte = self.buffered.pop_front().unwrap();
        }
        Ok(count)
    }
}

#[derive(Clone)]
struct PipeWriter(mpsc::Sender<Vec<u8>>);

impl Write for PipeWriter {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        self.0
            .send(input.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "pipe reader closed"))?;
        Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn pair() -> (
    Client<PipeWriter>,
    mpsc::Receiver<Incoming>,
    BufReader<PipeReader>,
    PipeWriter,
) {
    let (to_gdb, from_client) = mpsc::channel();
    let (to_client, from_gdb) = mpsc::channel();
    let client_reader = PipeReader {
        receiver: from_gdb,
        buffered: VecDeque::new(),
    };
    let gdb_reader = PipeReader {
        receiver: from_client,
        buffered: VecDeque::new(),
    };
    let (client, incoming) = Client::spawn(client_reader, PipeWriter(to_gdb));
    (
        client,
        incoming,
        BufReader::new(gdb_reader),
        PipeWriter(to_client),
    )
}

#[derive(Serialize)]
struct Echo {
    positional: String,
}

impl MiCommand for Echo {
    const OP: &'static str = "echo";
    type Reply = EchoReply;
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct EchoReply {
    value: String,
}

fn read_command(reader: &mut impl BufRead) -> (u64, String) {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let (token, rest) = line.split_once('-').unwrap();
    let value = rest.split_whitespace().nth(1).unwrap().to_owned();
    (token.parse().unwrap(), value)
}

#[test]
fn correlates_out_of_order_replies_and_delivers_async_records() {
    let (client, incoming, mut gdb, mut gdb_writer) = pair();
    let server = thread::spawn(move || {
        let first = read_command(&mut gdb);
        let second = read_command(&mut gdb);
        gdb_writer
            .write_all(b"*stopped,reason=\"breakpoint-hit\",thread-id=\"1\"\n")
            .unwrap();
        for (token, value) in [second, first] {
            writeln!(gdb_writer, "{token}^done,value=\"{value}\"").unwrap();
        }
    });

    let first = client
        .begin(&Echo {
            positional: "one".into(),
        })
        .unwrap();
    let second = client
        .begin(&Echo {
            positional: "two".into(),
        })
        .unwrap();

    assert!(matches!(
        incoming.recv().unwrap(),
        Incoming::Record(Record::Async {
            class: AsyncClass::Stopped,
            ..
        })
    ));
    assert_eq!(first.wait().unwrap().value, "one");
    assert_eq!(second.wait().unwrap().value, "two");
    server.join().unwrap();
}

#[test]
fn surfaces_gdb_error_results() {
    let (client, _incoming, mut gdb, mut gdb_writer) = pair();
    thread::spawn(move || {
        let (token, _) = read_command(&mut gdb);
        writeln!(
            gdb_writer,
            "{token}^error,msg=\"No symbol table is loaded\""
        )
        .unwrap();
    });

    let error = client
        .send(&Echo {
            positional: "value".into(),
        })
        .unwrap_err();
    assert!(matches!(
        error,
        Error::Command { command, message, .. }
            if command == "echo" && message == "No symbol table is loaded"
    ));
}

#[test]
fn a_late_reply_after_timeout_does_not_close_the_connection() {
    let (client, _incoming, mut gdb, mut gdb_writer) = pair();
    thread::spawn(move || {
        let first = read_command(&mut gdb);
        thread::sleep(Duration::from_millis(30));
        writeln!(gdb_writer, "{}^done,value=\"{}\"", first.0, first.1).unwrap();
        let second = read_command(&mut gdb);
        writeln!(gdb_writer, "{}^done,value=\"{}\"", second.0, second.1).unwrap();
    });

    let timed_out = client.send_timeout(
        &Echo {
            positional: "slow".into(),
        },
        Duration::from_millis(5),
    );
    assert!(matches!(timed_out, Err(Error::Timeout(_))));
    let response = client
        .send_response(&Echo {
            positional: "next".into(),
        })
        .unwrap();
    assert_eq!(response.class, lazydbg_mi::ResultClass::Done);
    assert_eq!(response.reply.value, "next");
    assert!(matches!(
        response.results.get("value"),
        Some(lazydbg_mi::Value::Str(value)) if value == "next"
    ));
}

#[test]
fn parser_failure_stops_pending_commands_instead_of_hanging() {
    let (client, incoming, mut gdb, mut gdb_writer) = pair();
    thread::spawn(move || {
        let _ = read_command(&mut gdb);
        gdb_writer.write_all(b"1^not-a-result-class\n").unwrap();
    });

    let pending = client
        .begin(&Echo {
            positional: "value".into(),
        })
        .unwrap();
    assert!(matches!(
        incoming.recv().unwrap(),
        Incoming::ReaderError(Error::Parse(_))
    ));
    assert!(matches!(pending.wait(), Err(Error::Disconnected)));
}

#[test]
fn raw_inferior_output_is_delivered_without_stopping_response_processing() {
    let (client, incoming, mut gdb, mut gdb_writer) = pair();
    thread::spawn(move || {
        let (token, value) = read_command(&mut gdb);
        gdb_writer
            .write_all(b"starting call chain\nlevel1: 42\n")
            .unwrap();
        writeln!(gdb_writer, "{token}^done,value=\"{value}\"").unwrap();
    });

    let pending = client
        .begin(&Echo {
            positional: "value".into(),
        })
        .unwrap();
    for expected in ["starting call chain", "level1: 42"] {
        assert!(matches!(
            incoming.recv().unwrap(),
            Incoming::Record(Record::Stream {
                kind: lazydbg_mi::StreamKind::Target,
                text,
            }) if text == expected
        ));
    }
    assert_eq!(pending.wait().unwrap().value, "value");
}
