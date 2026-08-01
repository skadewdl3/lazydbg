use std::io::{BufRead, BufReader, Read, Write};

use serde::Serialize;

use crate::error::{Error, Result};

const MAX_HEADER_BYTES: usize = 8 * 1024;
pub const DEFAULT_MAX_CONTENT_LENGTH: usize = 64 * 1024 * 1024;

pub struct FrameReader<R> {
    reader: BufReader<R>,
    max_content_length: usize,
}

impl<R: Read> FrameReader<R> {
    pub fn new(reader: R) -> Self {
        Self::with_max_content_length(reader, DEFAULT_MAX_CONTENT_LENGTH)
    }

    pub fn with_max_content_length(reader: R, max_content_length: usize) -> Self {
        Self {
            reader: BufReader::new(reader),
            max_content_length,
        }
    }

    pub fn read_frame(&mut self) -> Result<Option<Vec<u8>>> {
        let mut content_length = None;
        let mut header_bytes = 0usize;
        let mut saw_header = false;

        loop {
            let mut line = Vec::new();
            let read = self.reader.read_until(b'\n', &mut line)?;
            if read == 0 {
                if saw_header {
                    return Err(Error::InvalidHeader("unexpected EOF in header".into()));
                }
                return Ok(None);
            }
            saw_header = true;
            header_bytes = header_bytes
                .checked_add(read)
                .ok_or_else(|| Error::InvalidHeader("header length overflow".into()))?;
            if header_bytes > MAX_HEADER_BYTES {
                return Err(Error::InvalidHeader("header exceeds 8 KiB".into()));
            }
            if line == b"\r\n" {
                break;
            }
            if !line.ends_with(b"\r\n") {
                return Err(Error::InvalidHeader("header lines must end in CRLF".into()));
            }
            line.truncate(line.len() - 2);
            let line = std::str::from_utf8(&line)
                .map_err(|_| Error::InvalidHeader("header is not ASCII".into()))?;
            if !line.is_ascii() {
                return Err(Error::InvalidHeader("header is not ASCII".into()));
            }
            let (name, value) = line
                .split_once(": ")
                .ok_or_else(|| Error::InvalidHeader(line.to_owned()))?;
            if name.eq_ignore_ascii_case("Content-Length") {
                if content_length.is_some() {
                    return Err(Error::DuplicateContentLength);
                }
                content_length = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| Error::InvalidHeader(line.to_owned()))?,
                );
            }
        }

        let content_length = content_length.ok_or(Error::MissingContentLength)?;
        if content_length > self.max_content_length {
            return Err(Error::ContentTooLarge {
                actual: content_length,
                limit: self.max_content_length,
            });
        }
        let mut content = vec![0; content_length];
        self.reader.read_exact(&mut content)?;
        Ok(Some(content))
    }
}

pub fn encode<T: Serialize>(message: &T) -> Result<Vec<u8>> {
    let content = serde_json::to_vec(message)?;
    let mut framed = format!("Content-Length: {}\r\n\r\n", content.len()).into_bytes();
    framed.extend_from_slice(&content);
    Ok(framed)
}

pub fn write<T: Serialize, W: Write>(writer: &mut W, message: &T) -> Result<()> {
    writer.write_all(&encode(message)?)?;
    writer.flush()?;
    Ok(())
}
