use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::mpsc,
    thread,
    time::Duration,
};

use lazydbg_dap::{
    client::Client,
    codec::{FrameReader, write},
    error::Error,
    protocol::{DapEvent, Incoming},
};
use serde_json::{Value, json};

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
    PipeReader,
    PipeWriter,
) {
    let (to_adapter, from_client) = mpsc::channel();
    let (to_client, from_adapter) = mpsc::channel();
    let client_reader = PipeReader {
        receiver: from_adapter,
        buffered: VecDeque::new(),
    };
    let adapter_reader = PipeReader {
        receiver: from_client,
        buffered: VecDeque::new(),
    };
    let (client, incoming) = Client::spawn(client_reader, PipeWriter(to_adapter));
    (client, incoming, adapter_reader, PipeWriter(to_client))
}

#[test]
fn correlates_out_of_order_responses_and_delivers_events() {
    let (client, incoming, adapter, mut adapter_writer) = pair();
    let server = thread::spawn(move || {
        let mut reader = FrameReader::new(adapter);
        let first: Value = serde_json::from_slice(&reader.read_frame().unwrap().unwrap()).unwrap();
        let second: Value = serde_json::from_slice(&reader.read_frame().unwrap().unwrap()).unwrap();
        write(
            &mut adapter_writer,
            &json!({
                "seq": 1, "type": "event", "event": "initialized"
            }),
        )
        .unwrap();
        for request in [second, first] {
            write(
                &mut adapter_writer,
                &json!({
                    "seq": request["seq"].as_u64().unwrap() + 10,
                    "type": "response",
                    "request_seq": request["seq"],
                    "success": true,
                    "command": request["command"],
                    "body": { "echo": request["arguments"]["value"] }
                }),
            )
            .unwrap();
        }
    });

    let one = client.clone();
    let first = thread::spawn(move || {
        one.send_custom::<_, Value>("one", &json!({"value": 1}))
            .unwrap()
    });
    let two = client.clone();
    let second = thread::spawn(move || {
        two.send_custom::<_, Value>("two", &json!({"value": 2}))
            .unwrap()
    });

    assert_eq!(first.join().unwrap()["echo"], 1);
    assert_eq!(second.join().unwrap()["echo"], 2);
    assert!(matches!(
        incoming.recv().unwrap(),
        Incoming::Event(event) if matches!(*event, DapEvent::Initialized(_))
    ));
    server.join().unwrap();
}

#[test]
fn surfaces_protocol_errors() {
    let (client, _incoming, adapter, mut adapter_writer) = pair();
    thread::spawn(move || {
        let mut reader = FrameReader::new(adapter);
        let request: Value =
            serde_json::from_slice(&reader.read_frame().unwrap().unwrap()).unwrap();
        write(
            &mut adapter_writer,
            &json!({
                "seq": 0, "type": "response", "request_seq": request["seq"],
                "success": false, "command": request["command"], "message": "notStopped",
                "body": { "detail": "running" }
            }),
        )
        .unwrap();
    });

    let error = client
        .send_custom::<_, Value>("stackTrace", &json!({}))
        .unwrap_err();
    assert!(matches!(error, Error::Protocol { message, .. } if message == "notStopped"));
}

#[test]
fn a_late_response_after_timeout_does_not_close_the_connection() {
    let (client, _incoming, adapter, mut adapter_writer) = pair();
    thread::spawn(move || {
        let mut reader = FrameReader::new(adapter);
        let first: Value = serde_json::from_slice(&reader.read_frame().unwrap().unwrap()).unwrap();
        thread::sleep(Duration::from_millis(30));
        write(
            &mut adapter_writer,
            &json!({
                "seq": 10, "type": "response", "request_seq": first["seq"],
                "success": true, "command": first["command"]
            }),
        )
        .unwrap();
        let second: Value = serde_json::from_slice(&reader.read_frame().unwrap().unwrap()).unwrap();
        write(
            &mut adapter_writer,
            &json!({
                "seq": 11, "type": "response", "request_seq": second["seq"],
                "success": true, "command": second["command"],
                "body": { "acknowledged": true },
                "adapterExtension": "preserved"
            }),
        )
        .unwrap();
    });

    let timed_out =
        client.send_custom_timeout::<_, Value>("slow", &json!({}), Duration::from_millis(5));
    assert!(matches!(timed_out, Err(Error::Timeout(_))));
    let response = client
        .send_custom_response::<_, Value>("next", &json!({}))
        .unwrap();
    assert_eq!(response.body["acknowledged"], true);
    assert_eq!(response.raw()["command"], "next");
    assert_eq!(response.raw()["adapterExtension"], "preserved");
}

#[test]
fn pending_request_allows_event_processing_before_response_completion() {
    let (client, incoming, adapter, mut adapter_writer) = pair();
    thread::spawn(move || {
        let mut reader = FrameReader::new(adapter);
        let request: Value =
            serde_json::from_slice(&reader.read_frame().unwrap().unwrap()).unwrap();
        write(
            &mut adapter_writer,
            &json!({
                "seq": 2, "type": "event", "event": "initialized"
            }),
        )
        .unwrap();
        write(
            &mut adapter_writer,
            &json!({
                "seq": 3, "type": "response", "request_seq": request["seq"],
                "success": true, "command": request["command"],
                "body": { "started": true }
            }),
        )
        .unwrap();
    });

    let pending = client
        .begin_custom::<_, Value>("launch", &json!({}))
        .unwrap();
    assert!(matches!(
        incoming.recv().unwrap(),
        Incoming::Event(event) if matches!(*event, DapEvent::Initialized(_))
    ));
    assert_eq!(pending.wait().unwrap()["started"], true);
}
