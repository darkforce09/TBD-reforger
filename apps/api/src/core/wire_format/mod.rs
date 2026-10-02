//! The JSON wire contract's shared serialization primitives: the timestamp formats every model
//! renders through, the `jsonb` passthrough type, and the canonical-JSON content digest.

pub mod content_digest;
pub mod raw_json;
pub mod rfc3339_timestamps;

pub use raw_json::RawJson;
pub use rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_date, rfc3339_utc_opt};
