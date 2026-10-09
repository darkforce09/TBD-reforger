//! Reads SSE events from a response body, from the in-process router or from the `api-server`
//! binary over HTTP, with bounded waits that tell an open stream, a delivered event and a closed body apart.

use std::time::{Duration, Instant};

use axum::body::BodyDataStream;
use futures::StreamExt;
use serde_json::Value;
use tokio::time::timeout;

/// One SSE event: its name (`None` for an audit row), its id and its data.
#[derive(Debug, Clone)]
pub(crate) struct SseEvent {
    pub event: Option<String>,
    pub id: Option<String>,
    pub data: String,
}

impl SseEvent {
    /// The event id as a publication sequence.
    pub(crate) fn sequence(&self) -> i64 {
        self.id
            .as_deref()
            .and_then(|id| id.parse().ok())
            .unwrap_or_else(|| panic!("event without a numeric id: {self:?}"))
    }

    /// The event data as JSON.
    pub(crate) fn json(&self) -> Value {
        serde_json::from_str(&self.data)
            .unwrap_or_else(|error| panic!("event data is not JSON ({error}): {self:?}"))
    }

    /// `true` for an audit row: an unnamed event.
    pub(crate) fn is_row(&self) -> bool {
        self.event.is_none()
    }

    /// `(sequence, audit id)` of an audit row.
    pub(crate) fn publication(&self) -> (i64, i64) {
        assert!(self.is_row(), "an audit row is an unnamed event: {self:?}");
        (
            self.sequence(),
            self.json()["id"]
                .as_i64()
                .unwrap_or_else(|| panic!("row without an id: {self:?}")),
        )
    }
}

/// Where a reader's bytes come from.
enum BodySource {
    /// A response of the in-process router.
    Router(BodyDataStream),
    /// A response of the `api-server` binary over HTTP.
    Http(reqwest::Response),
}

impl BodySource {
    /// The next piece of the body as text; `Ok(None)` once the body has ended cleanly.
    async fn next_text(&mut self) -> Result<Option<String>, String> {
        let bytes = match self {
            Self::Router(body) => match body.next().await {
                None => return Ok(None),
                Some(piece) => piece.map_err(|error| error.to_string())?.to_vec(),
            },
            Self::Http(response) => match response.chunk().await {
                Ok(None) => return Ok(None),
                Ok(Some(piece)) => piece.to_vec(),
                Err(error) => return Err(error.to_string()),
            },
        };
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|error| format!("SSE is UTF-8: {error}"))
    }
}

/// What a bounded wait on a stream saw.
enum SseWait {
    Event(SseEvent),
    Ended,
    Quiet,
}

/// Reads SSE events from one response body frame by frame.
pub(crate) struct SseReader {
    source: BodySource,
    pending: String,
    ended: bool,
}

impl SseReader {
    /// A reader over a response of the in-process router.
    pub(crate) fn from_router(response: axum::response::Response) -> Self {
        Self::new(BodySource::Router(response.into_body().into_data_stream()))
    }

    /// A reader over a response of the `api-server` binary.
    pub(crate) fn from_http(response: reqwest::Response) -> Self {
        Self::new(BodySource::Http(response))
    }

    fn new(source: BodySource) -> Self {
        Self {
            source,
            pending: String::new(),
            ended: false,
        }
    }

    /// The next event, the end of the body, or quiet once `bound` passes. Comment-only blocks
    /// (keep-alives) are skipped; a body that fails instead of ending fails the case.
    async fn wait(&mut self, bound: Duration) -> SseWait {
        let deadline = Instant::now() + bound;
        loop {
            while let Some(end) = self.pending.find("\n\n") {
                let block: String = self.pending.drain(..end + 2).collect();
                if let Some(event) = parse_block(&block) {
                    return SseWait::Event(event);
                }
            }
            if self.ended {
                return SseWait::Ended;
            }
            let left = deadline.saturating_duration_since(Instant::now());
            match timeout(left, self.source.next_text()).await {
                Err(_) => return SseWait::Quiet,
                Ok(Ok(None)) => {
                    assert!(
                        self.pending.trim().is_empty(),
                        "the body ended inside an event: {:?}",
                        self.pending
                    );
                    self.ended = true;
                }
                Ok(Ok(Some(text))) => self.pending.push_str(&text),
                Ok(Err(error)) => panic!("the stream body failed instead of ending: {error}"),
            }
        }
    }

    /// The next event, failing the case when none arrives within `bound` or the body ends.
    pub(crate) async fn expect_event(&mut self, bound: Duration, why: &str) -> SseEvent {
        match self.wait(bound).await {
            SseWait::Event(event) => event,
            SseWait::Ended => panic!("the stream ended before an event: {why}"),
            SseWait::Quiet => panic!("no event within {bound:?}: {why}"),
        }
    }

    /// The next `count` events, every one an audit row, all within `bound`.
    pub(crate) async fn expect_rows(
        &mut self,
        count: usize,
        bound: Duration,
        why: &str,
    ) -> Vec<SseEvent> {
        let deadline = Instant::now() + bound;
        let mut rows = Vec::with_capacity(count);
        while rows.len() < count {
            let left = deadline.saturating_duration_since(Instant::now());
            let event = self.expect_event(left, why).await;
            assert!(
                event.is_row(),
                "expected row {} of {count} ({why}), got {event:?}",
                rows.len()
            );
            rows.push(event);
        }
        rows
    }

    /// Asserts the stream stays open and silent for `bound`.
    pub(crate) async fn expect_quiet(&mut self, bound: Duration, why: &str) {
        match self.wait(bound).await {
            SseWait::Quiet => {}
            SseWait::Event(event) => {
                panic!("expected no event within {bound:?} ({why}), got {event:?}")
            }
            SseWait::Ended => panic!("expected an open stream ({why}), the body ended"),
        }
    }

    /// Every event delivered before the body ends, failing the case when it has not ended within
    /// `bound`.
    pub(crate) async fn read_to_end(&mut self, bound: Duration, why: &str) -> Vec<SseEvent> {
        let deadline = Instant::now() + bound;
        let mut events = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.wait(left).await {
                SseWait::Event(event) => events.push(event),
                SseWait::Ended => return events,
                SseWait::Quiet => panic!(
                    "the body did not end within {bound:?} ({why}); events before the wait \
                     ran out: {events:?}"
                ),
            }
        }
    }
}

/// One `\n\n`-terminated SSE block as an event, or `None` for a comment-only block.
fn parse_block(block: &str) -> Option<SseEvent> {
    let mut event = None;
    let mut id = None;
    let mut data: Vec<&str> = Vec::new();
    let mut fields = 0;
    for line in block.lines().filter(|line| !line.is_empty()) {
        if line.starts_with(':') {
            continue;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        fields += 1;
        match field {
            "event" => event = Some(value.to_owned()),
            "id" => id = Some(value.to_owned()),
            "data" => data.push(value),
            _ => {}
        }
    }
    (fields > 0).then(|| SseEvent {
        event,
        id,
        data: data.join("\n"),
    })
}
