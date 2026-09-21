//! The JSON wire contract's shared serialization primitives: the timestamp formats every model
//! renders through, and the `jsonb` passthrough type.

pub mod raw_json;
pub mod rfc3339_timestamps;

pub use raw_json::RawJson;
pub use rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_date, rfc3339_utc_opt};
