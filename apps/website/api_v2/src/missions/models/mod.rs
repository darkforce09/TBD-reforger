//! Mission-domain database and wire models.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`crate::core::wire_format`]. The enums map to the Postgres ENUM types. The camelCase
//! compiled-document and export structs live in `services` / `handlers`, not here. Soft-delete
//! columns are absent from these structs — the filter is enforced in the query layer.

pub mod faction;
pub mod mission;
pub mod registry;

pub use faction::UserFaction;
pub use mission::{
    GameMode, Mission, MissionArmory, MissionBookmark, MissionDefaultOverride,
    MissionDefaultValueBucket, MissionStatus, MissionVersion, TerrainType, WeatherType,
};
pub use registry::{RegistryCompatEdge, RegistryItem};
