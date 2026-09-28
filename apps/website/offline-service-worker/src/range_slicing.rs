//! Byte-range requests answered from a cached full body.
//!
//! **Role:** parses a single-range `Range: bytes=…` header and plans the response to it over a
//! body of known length: the full body, one `206 Partial Content` slice, or `416 Range Not
//! Satisfiable`.
//! **Position:** the worker's map-asset handler calls [`plan_range_response`] with the request's
//! header and the cached body's length, then slices the cached blob by the returned
//! [`ByteSlice`]; the renderer's `Range` reads of the satellite mosaic land here offline.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a returned slice always satisfies `start <= end_inclusive < total_len`; a
//! header that is not a single well-formed `bytes` range is ignored and the full body is served
//! (RFC 9110 §14.2); a well-formed range that selects no byte is unsatisfiable, never an empty
//! `206`.

/// `206 Partial Content`.
pub const STATUS_PARTIAL_CONTENT: u16 = 206;

/// `416 Range Not Satisfiable`.
pub const STATUS_RANGE_NOT_SATISFIABLE: u16 = 416;

/// One byte range as the client wrote it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteRangeRequest {
    /// `bytes=a-b`: bytes `a` through `b`, inclusive.
    Bounded {
        /// First byte offset.
        start: u64,
        /// Last byte offset, inclusive.
        end_inclusive: u64,
    },
    /// `bytes=a-`: byte `a` through the end.
    OpenEnded {
        /// First byte offset.
        start: u64,
    },
    /// `bytes=-n`: the last `n` bytes.
    Suffix {
        /// How many trailing bytes.
        length: u64,
    },
}

/// Why a `Range` header is ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeHeaderError {
    /// The unit is not `bytes`.
    UnsupportedUnit,
    /// More than one range; the full body is served instead of a multipart response.
    MultipleRanges,
    /// A range that does not follow `a-b`, `a-` or `-n` with decimal offsets and `a <= b`.
    Malformed,
}

/// One satisfiable slice of a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSlice {
    /// First byte offset.
    pub start: u64,
    /// Last byte offset, inclusive.
    pub end_inclusive: u64,
    /// Length of the whole body.
    pub total_len: u64,
}

impl ByteSlice {
    /// One past the last byte offset, the `end` argument of `Blob.slice`.
    pub fn end_exclusive(&self) -> u64 {
        self.end_inclusive + 1
    }

    /// Number of bytes in the slice, the `Content-Length` of the `206`.
    pub fn len(&self) -> u64 {
        self.end_exclusive() - self.start
    }

    /// Always false: a slice holds at least one byte.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The `Content-Range` header value, `bytes a-b/total`.
    pub fn content_range(&self) -> String {
        format!(
            "bytes {}-{}/{}",
            self.start, self.end_inclusive, self.total_len
        )
    }
}

/// How to answer a request over a body of known length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeResponsePlan {
    /// No usable `Range` header: answer with the full body and its original status.
    FullBody,
    /// Answer `206` with this slice.
    Partial(ByteSlice),
    /// Answer `416`; the `Content-Range` is [`unsatisfied_content_range`] of this length.
    Unsatisfiable {
        /// Length of the whole body.
        total_len: u64,
    },
}

/// Parses a `Range` header value holding one byte range.
pub fn parse_range_header(value: &str) -> Result<ByteRangeRequest, RangeHeaderError> {
    let (unit, ranges) = value
        .trim()
        .split_once('=')
        .ok_or(RangeHeaderError::Malformed)?;
    if !unit.trim().eq_ignore_ascii_case("bytes") {
        return Err(RangeHeaderError::UnsupportedUnit);
    }
    if ranges.contains(',') {
        return Err(RangeHeaderError::MultipleRanges);
    }
    let (first, last) = ranges
        .trim()
        .split_once('-')
        .ok_or(RangeHeaderError::Malformed)?;
    match (parse_offset(first)?, parse_offset(last)?) {
        (Some(start), Some(end_inclusive)) if start <= end_inclusive => {
            Ok(ByteRangeRequest::Bounded {
                start,
                end_inclusive,
            })
        }
        (Some(start), None) => Ok(ByteRangeRequest::OpenEnded { start }),
        (None, Some(length)) => Ok(ByteRangeRequest::Suffix { length }),
        _ => Err(RangeHeaderError::Malformed),
    }
}

/// Resolves `request` against a body of `total_len` bytes.
pub fn resolve_range(request: ByteRangeRequest, total_len: u64) -> RangeResponsePlan {
    let unsatisfiable = RangeResponsePlan::Unsatisfiable { total_len };
    if total_len == 0 {
        return unsatisfiable;
    }
    let last = total_len - 1;
    let (start, end_inclusive) = match request {
        ByteRangeRequest::Bounded {
            start,
            end_inclusive,
        } => (start, end_inclusive.min(last)),
        ByteRangeRequest::OpenEnded { start } => (start, last),
        ByteRangeRequest::Suffix { length: 0 } => return unsatisfiable,
        ByteRangeRequest::Suffix { length } => (total_len.saturating_sub(length), last),
    };
    if start > last {
        return unsatisfiable;
    }
    RangeResponsePlan::Partial(ByteSlice {
        start,
        end_inclusive,
        total_len,
    })
}

/// Plans the response to a request carrying `range_header` (if any) over a body of `total_len`
/// bytes; an unusable header yields [`RangeResponsePlan::FullBody`].
pub fn plan_range_response(range_header: Option<&str>, total_len: u64) -> RangeResponsePlan {
    match range_header.map(parse_range_header) {
        Some(Ok(request)) => resolve_range(request, total_len),
        None | Some(Err(_)) => RangeResponsePlan::FullBody,
    }
}

/// The `Content-Range` header value of a `416`, `bytes */total`.
pub fn unsatisfied_content_range(total_len: u64) -> String {
    format!("bytes */{total_len}")
}

fn parse_offset(text: &str) -> Result<Option<u64>, RangeHeaderError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    if !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RangeHeaderError::Malformed);
    }
    text.parse()
        .map(Some)
        .map_err(|_| RangeHeaderError::Malformed)
}

#[cfg(test)]
#[path = "tests/range_slicing.rs"]
mod tests;
