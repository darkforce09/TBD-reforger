//! The names a caller of the browser helpers imports with `use browser_platform::prelude::*;`.

pub use crate::fetch::{
    ByteProgress, RangeBody, RangeOutcome, StreamedBody, fetch_bytes, fetch_bytes_streamed,
    fetch_range_outcome, fetch_text, open_streamed_body,
};
pub use crate::{console_error, console_log, console_warn};
