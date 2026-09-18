//! Community-content wire/database models.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`crate::core::wire_format`]. The enums map to Postgres ENUM types. Soft-delete columns are
//! absent from these structs — the filter is enforced in the query layer.

pub mod announcement;
pub mod modpack;
pub mod wiki;

pub use announcement::{Announcement, AnnouncementStatus, AnnouncementTag};
pub use modpack::{Modpack, ModpackMod};
pub use wiki::{VehicleDatabase, WikiPage};
