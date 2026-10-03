//! The JSON wire contract's shared serialization primitives: the midnight-UTC date spelling, the
//! `jsonb` passthrough type, and the canonical-JSON content digest. The instant spellings every
//! model writes are `fleet_wire_contract::rfc3339_timestamps`.

pub mod content_digest;
pub mod raw_json;
pub mod rfc3339_dates;

pub use raw_json::RawJson;
pub use rfc3339_dates::rfc3339_utc_date;
