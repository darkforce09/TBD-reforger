//! The JSON wire contract's shared serialization primitives: the timestamp formats every model
//! renders through, and the `jsonb` passthrough type.

pub mod go_compatible_time;
pub mod raw_json;

pub use go_compatible_time::{go_date, go_time, go_time_opt};
pub use raw_json::RawJson;
