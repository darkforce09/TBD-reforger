//! Data models — Rust port of `internal/models`.
//!
//! Field order + JSON tags mirror the Go structs (the wire contract): snake_case
//! throughout (the 241 census tags), `omitempty` → `skip_serializing_if`, Go
//! RFC3339Nano timestamps via [`serde_helpers`]. The 12 enums map to the Postgres
//! ENUM types. The camelCase compiled-doc + export structs live in `services`/
//! `handlers`, not here. GORM `DeletedAt` fields are omitted — soft delete is
//! enforced in the query layer (the 4 tables: users, missions, events, announcements).

pub mod serde_helpers;

pub mod admin;
pub mod content;
pub mod event;
pub mod mission;
pub mod registry;
pub mod telemetry;
pub mod user;

pub use admin::*;
pub use content::*;
pub use event::*;
pub use mission::*;
pub use registry::*;
pub use telemetry::*;
pub use user::*;

/// `jsonb` passthrough: sqlx decodes the column into a `RawValue` and serde re-emits
/// it verbatim (Postgres-normalized bytes, no re-serialization) — Encoder hazard #8.
pub type RawJson = sqlx::types::Json<Box<serde_json::value::RawValue>>;
