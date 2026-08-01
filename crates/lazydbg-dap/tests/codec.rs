use std::io::Cursor;

use lazydbg_dap::{
    codec::{FrameReader, encode},
    error::Error,
};
use serde_json::json;

#[test]
fn content_length_counts_utf8_bytes() {
    let framed = encode(&json!({"output": "lambda: lambda".replace("lambda", "λ")})).unwrap();
    let separator = framed
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .unwrap();
    let header = std::str::from_utf8(&framed[..separator]).unwrap();
    let body = &framed[separator + 4..];
    assert_eq!(header, format!("Content-Length: {}", body.len()));
}

#[test]
fn reads_back_to_back_frames() {
    let first = encode(&json!({"seq": 1})).unwrap();
    let second = encode(&json!({"seq": 2})).unwrap();
    let mut bytes = first;
    bytes.extend(second);
    let mut reader = FrameReader::new(Cursor::new(bytes));

    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&reader.read_frame().unwrap().unwrap())
            .unwrap(),
        json!({"seq": 1})
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&reader.read_frame().unwrap().unwrap())
            .unwrap(),
        json!({"seq": 2})
    );
    assert!(reader.read_frame().unwrap().is_none());
}

#[test]
fn rejects_missing_duplicate_and_oversized_lengths() {
    let mut missing = FrameReader::new(Cursor::new(b"Other: 1\r\n\r\n"));
    assert!(matches!(
        missing.read_frame(),
        Err(Error::MissingContentLength)
    ));

    let duplicate = b"Content-Length: 0\r\nContent-Length: 0\r\n\r\n";
    let mut duplicate = FrameReader::new(Cursor::new(duplicate));
    assert!(matches!(
        duplicate.read_frame(),
        Err(Error::DuplicateContentLength)
    ));

    let mut oversized =
        FrameReader::with_max_content_length(Cursor::new(b"Content-Length: 2\r\n\r\n{}"), 1);
    assert!(matches!(
        oversized.read_frame(),
        Err(Error::ContentTooLarge { .. })
    ));
}
