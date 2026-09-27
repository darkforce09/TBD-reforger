//! Server-Sent Events framing: raw bytes in, dispatched messages out.
//!
//! **Role:** an incremental parser for the `text/event-stream` format. It decodes UTF-8 across
//! chunk boundaries, splits lines on LF, CRLF or a lone CR, reads the `event`, `data`, `id` and
//! `retry` fields and the comment lines, and dispatches one message per blank line.
//! **Position:** fed byte chunks by a stream transport (the audit stream's reader loop) in the
//! order the network delivers them; hands the parsed items back to that transport. Pure, so it
//! compiles and is tested natively, where the tests are its only caller.
//! **Signals & state:** the parser owns its decode buffer (an incomplete UTF-8 sequence carried to
//! the next chunk), the partial line, and the event type, data and last event id buffers.
//! **Invariants:** the parsing follows the HTML standard's event-stream interpretation. A chunk may
//! end anywhere, including inside a multi-byte character or between the CR and LF of one line
//! break, and the result equals parsing the whole stream at once. Malformed UTF-8 decodes to
//! U+FFFD. A leading byte order mark is dropped once. A message is dispatched only on a blank line
//! and only when a `data` field was seen; a frame the stream ends in the middle of is never
//! dispatched. The last event id persists from frame to frame until an `id` field replaces it, and
//! an `id` value holding U+0000 is ignored.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

/// One dispatched event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseMessage {
    /// The event type: the frame's last `event` field, or `message` when the frame names none.
    pub event: String,
    /// The frame's `data` fields joined by line feeds.
    pub data: String,
    /// The last event id at dispatch: the most recent `id` field on this stream, carried across
    /// frames that name none; empty when no id has been set.
    pub last_event_id: String,
}

/// One item the parser surfaces, in stream order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SseItem {
    /// A dispatched event.
    Message(SseMessage),
    /// A comment line: a keep-alive, whose text carries nothing.
    Comment,
    /// A `retry` field: the reconnection time the server asks for, in milliseconds.
    Retry(u64),
}

/// The type an event carries when its frame names none.
pub const DEFAULT_EVENT_TYPE: &str = "message";

/// The byte order mark a stream may open with.
const BYTE_ORDER_MARK: char = '\u{FEFF}';

/// An incremental event-stream parser for one connection.
#[derive(Clone, Debug)]
pub struct SseParser {
    /// The tail of the last chunk that ends inside a multi-byte character.
    undecoded: Vec<u8>,
    /// The line read so far, without its terminator.
    line: String,
    /// The previous character was a CR, so an LF right after it closes nothing.
    after_carriage_return: bool,
    /// No character has been read yet, so a byte order mark is still dropped.
    at_stream_start: bool,
    /// The `event` field of the frame being read.
    event_type: String,
    /// The `data` fields of the frame being read, each followed by a line feed.
    data: String,
    /// The last event id buffer: the latest `id` field value, kept across frames.
    last_event_id_buffer: String,
}

impl Default for SseParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SseParser {
    /// A parser at the start of a stream.
    pub fn new() -> Self {
        Self {
            undecoded: Vec::new(),
            line: String::new(),
            after_carriage_return: false,
            at_stream_start: true,
            event_type: String::new(),
            data: String::new(),
            last_event_id_buffer: String::new(),
        }
    }

    /// Parse one chunk and return every item it completes, in stream order.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseItem> {
        let text = self.decode(bytes);
        let mut out = Vec::new();
        for c in text.chars() {
            if self.at_stream_start {
                self.at_stream_start = false;
                if c == BYTE_ORDER_MARK {
                    continue;
                }
            }
            if self.after_carriage_return {
                self.after_carriage_return = false;
                if c == '\n' {
                    continue;
                }
            }
            match c {
                '\r' => {
                    self.after_carriage_return = true;
                    self.end_line(&mut out);
                }
                '\n' => self.end_line(&mut out),
                _ => self.line.push(c),
            }
        }
        out
    }

    /// Decode the carried tail plus `bytes`, keeping an incomplete trailing character for the
    /// next chunk and replacing every invalid sequence with U+FFFD.
    fn decode(&mut self, bytes: &[u8]) -> String {
        let mut pending = std::mem::take(&mut self.undecoded);
        pending.extend_from_slice(bytes);
        let mut out = String::with_capacity(pending.len());
        let mut rest: &[u8] = &pending;
        loop {
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    out.push_str(valid);
                    rest = &[];
                    break;
                }
                Err(error) => {
                    let (valid, tail) = rest.split_at(error.valid_up_to());
                    out.push_str(std::str::from_utf8(valid).unwrap_or_default());
                    match error.error_len() {
                        Some(invalid) => {
                            out.push(char::REPLACEMENT_CHARACTER);
                            rest = &tail[invalid..];
                        }
                        None => {
                            rest = tail;
                            break;
                        }
                    }
                }
            }
        }
        self.undecoded = rest.to_vec();
        out
    }

    /// Interpret the line just closed: a blank line dispatches, a colon-led line is a comment,
    /// and anything else is a field.
    fn end_line(&mut self, out: &mut Vec<SseItem>) {
        let line = std::mem::take(&mut self.line);
        if line.is_empty() {
            self.dispatch(out);
            return;
        }
        if line.starts_with(':') {
            out.push(SseItem::Comment);
            return;
        }
        let (field, value) = match line.find(':') {
            Some(colon) => {
                let value = &line[colon + 1..];
                (&line[..colon], value.strip_prefix(' ').unwrap_or(value))
            }
            None => (line.as_str(), ""),
        };
        match field {
            "event" => self.event_type = value.to_string(),
            "data" => {
                self.data.push_str(value);
                self.data.push('\n');
            }
            "id" if !value.contains('\0') => self.last_event_id_buffer = value.to_string(),
            "retry" if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
                out.push(SseItem::Retry(value.parse().unwrap_or(u64::MAX)));
            }
            _ => {}
        }
    }

    /// Close the frame: emit the message, stamped with the last event id, when data was seen.
    fn dispatch(&mut self, out: &mut Vec<SseItem>) {
        let event_type = std::mem::take(&mut self.event_type);
        let mut data = std::mem::take(&mut self.data);
        if data.is_empty() {
            return;
        }
        if data.ends_with('\n') {
            data.pop();
        }
        let event = if event_type.is_empty() {
            DEFAULT_EVENT_TYPE.to_string()
        } else {
            event_type
        };
        out.push(SseItem::Message(SseMessage {
            event,
            data,
            last_event_id: self.last_event_id_buffer.clone(),
        }));
    }
}

#[cfg(test)]
#[path = "tests/sse_frames.rs"]
mod tests;
