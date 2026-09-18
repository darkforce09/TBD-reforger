//! Cross-domain database and wire models. The mission and operations domains keep their own
//! under [`crate::missions::models`] and [`crate::operations::models`].
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`crate::core::wire_format`]. The enums map to the Postgres ENUM types. The camelCase
//! compiled-doc and export structs live in `services` / `handlers`, not here. Soft-delete
//! columns are absent from these structs — the filter is enforced in the query layer (the
//! soft-deletable table reached from here is users; announcements belong to
//! [`crate::community_content::models`]).

pub mod admin;
pub mod telemetry;

pub use admin::*;
pub use telemetry::*;
