//! Data models — Rust port of `internal/models`.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`crate::core::wire_format`]. The 12 enums map to the Postgres ENUM types. The camelCase
//! compiled-doc and export structs live in `services` / `handlers`, not here. Soft-delete
//! columns are absent from these structs — the filter is enforced in the query layer (the 4
//! soft-deletable tables: users, missions, events, announcements).

pub mod admin;
pub mod content;
pub mod event;
pub mod faction;
pub mod mission;
pub mod registry;
pub mod telemetry;
pub mod user;

pub use admin::*;
pub use content::*;
pub use event::*;
pub use faction::*;
pub use mission::*;
pub use registry::*;
pub use telemetry::*;
pub use user::*;
