//! Same-origin HTTP GETs from the browser.
//!
//! **Role:** whole-body GETs ([`fetch_bytes`], [`fetch_text`]), a streamed GET that reports its
//! progress as byte counts ([`open_streamed_body`], [`StreamedBody::read_to_end`],
//! [`fetch_bytes_streamed`]) and an HTTP Range GET that says which way it went
//! ([`fetch_range_outcome`]).
//! **Position:** foundation, wasm32 only, over `gloo-net` and the browser's body reader; the map
//! engine's streaming host and its terrain, satellite, water, label, vegetation and world loaders
//! fetch `/map-assets` through it and turn [`ByteProgress`] into their own progress events.
//! **Signals & state:** none beyond the request in flight; nothing is cached.
//! **Invariants:** a transport failure or a status outside 2xx is `None`, never an empty body.
//! Streamed progress counts the bytes that came out of the body reader and nothing else: one
//! report of zero bytes with the announced length before the first byte, one each time at least
//! `report_every_bytes` more arrived, and one at the end for any remainder. A Range GET succeeds
//! only on 206 with a `Content-Range` total, so a server that ignores `Range` never sends a whole
//! file.

use wasm_bindgen::JsCast;

/// How far a streamed body has come.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteProgress {
    /// Bytes received from the body so far.
    pub received: u64,
    /// The `content-length` the response announced, when it announced a number.
    pub total: Option<u64>,
}

/// GET `url` and keep the response only when its status is 2xx.
async fn get_successful(url: &str) -> Option<gloo_net::http::Response> {
    let response = gloo_net::http::Request::get(url).send().await.ok()?;
    (200..300).contains(&response.status()).then_some(response)
}

/// The whole body of a 2xx GET of `url`, or `None` on a transport failure or another status.
pub async fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    get_successful(url).await?.binary().await.ok()
}

/// The whole body of a 2xx GET of `url` as text, or `None` on a transport failure or another
/// status.
pub async fn fetch_text(url: &str) -> Option<String> {
    get_successful(url).await?.text().await.ok()
}

/// Where a streamed body's bytes come from.
enum BodySource {
    /// The browser's reader over the response body, chunk by chunk.
    Reader(web_sys::ReadableStreamDefaultReader),
    /// A response the browser exposes no body stream for, read whole.
    Whole(gloo_net::http::Response),
}

/// The body of a 2xx GET, opened and not yet read; [`StreamedBody::read_to_end`] reads it.
pub struct StreamedBody {
    content_length: Option<u64>,
    source: BodySource,
}

/// GETs `url` and opens its body for streaming, or `None` on a transport failure or a status
/// outside 2xx.
pub async fn open_streamed_body(url: &str) -> Option<StreamedBody> {
    let response = get_successful(url).await?;
    let content_length = response
        .headers()
        .get("content-length")
        .and_then(|value| value.parse::<u64>().ok());
    let source = match response.body() {
        Some(body) => BodySource::Reader(body.get_reader().unchecked_into()),
        None => BodySource::Whole(response),
    };
    Some(StreamedBody {
        content_length,
        source,
    })
}

impl StreamedBody {
    /// The `content-length` the response announced, when it announced a number.
    pub fn content_length(&self) -> Option<u64> {
        self.content_length
    }

    /// Reads the body to its end, handing each chunk to `sink` in order and reporting
    /// [`ByteProgress`] to `progress`: zero bytes before the first chunk, then whenever at least
    /// `report_every_bytes` more arrived, then once more for any remainder. Answers the number of
    /// bytes read.
    ///
    /// `None` when the reader fails, a chunk is malformed, or `sink` refuses a chunk by
    /// answering `false`.
    pub async fn read_to_end(
        self,
        report_every_bytes: u64,
        progress: &dyn Fn(ByteProgress),
        sink: &mut dyn FnMut(&[u8]) -> bool,
    ) -> Option<u64> {
        let total = self.content_length;
        progress(ByteProgress { received: 0, total });
        let mut received: u64 = 0;
        let mut reported: u64 = 0;
        match self.source {
            BodySource::Whole(response) => {
                let bytes = response.binary().await.ok()?;
                if !sink(&bytes) {
                    return None;
                }
                received = bytes.len() as u64;
            }
            BodySource::Reader(reader) => loop {
                let chunk = wasm_bindgen_futures::JsFuture::from(reader.read())
                    .await
                    .ok()?;
                let done = js_sys::Reflect::get(&chunk, &"done".into())
                    .ok()
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true);
                if done {
                    break;
                }
                let value = js_sys::Reflect::get(&chunk, &"value".into()).ok()?;
                let array: js_sys::Uint8Array = value.unchecked_into();
                if !sink(&array.to_vec()) {
                    return None;
                }
                received += u64::from(array.length());
                if received - reported >= report_every_bytes {
                    progress(ByteProgress { received, total });
                    reported = received;
                }
            },
        }
        if received > reported {
            progress(ByteProgress { received, total });
        }
        Some(received)
    }
}

/// The whole body of a 2xx GET of `url`, read through the body reader so `progress` sees the
/// bytes as they arrive (see [`StreamedBody::read_to_end`] for when it is called).
pub async fn fetch_bytes_streamed(
    url: &str,
    report_every_bytes: u64,
    progress: &dyn Fn(ByteProgress),
) -> Option<Vec<u8>> {
    let body = open_streamed_body(url).await?;
    let capacity = body
        .content_length()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or(0);
    let mut bytes: Vec<u8> = Vec::with_capacity(capacity);
    body.read_to_end(report_every_bytes, progress, &mut |chunk| {
        bytes.extend_from_slice(chunk);
        true
    })
    .await?;
    Some(bytes)
}

/// The bytes of a successful Range GET and the whole file's size.
pub struct RangeBody {
    /// The requested bytes.
    pub bytes: Vec<u8>,
    /// The size of the whole file, from `Content-Range`.
    pub total: u64,
}

/// Which way a Range GET went.
pub enum RangeOutcome {
    /// A 206 with a `Content-Range` total.
    Body(RangeBody),
    /// `429`; carries the server's `Retry-After` in seconds when it sent one.
    RateLimited {
        /// The server's `Retry-After`, in seconds.
        retry_after_s: Option<u64>,
    },
    /// Any other status (a 200 from a server ignoring `Range` included), or a transport or parse
    /// failure (status 0).
    Failed {
        /// The response status, or 0 when no response arrived.
        status: u16,
    },
}

/// GETs bytes `start..=end_inclusive` of `url` and reports which way it went (see
/// [`RangeOutcome`]). Succeeds only on 206: a 200 means the server ignored `Range`, and is
/// refused so a whole file is never downloaded in place of one range.
pub async fn fetch_range_outcome(url: &str, start: u64, end_inclusive: u64) -> RangeOutcome {
    let Ok(response) = gloo_net::http::Request::get(url)
        .header("Range", &format!("bytes={start}-{end_inclusive}"))
        .send()
        .await
    else {
        return RangeOutcome::Failed { status: 0 };
    };
    let status = response.status();
    if status == 429 {
        return RangeOutcome::RateLimited {
            retry_after_s: response
                .headers()
                .get("retry-after")
                .and_then(|value| value.parse::<u64>().ok()),
        };
    }
    if status != 206 {
        return RangeOutcome::Failed { status };
    }
    let total = response
        .headers()
        .get("content-range")
        .and_then(|content_range| content_range.split('/').nth(1)?.parse::<u64>().ok())
        .filter(|&total| total > 0);
    let (Some(total), Ok(bytes)) = (total, response.binary().await) else {
        return RangeOutcome::Failed { status };
    };
    RangeOutcome::Body(RangeBody { bytes, total })
}
