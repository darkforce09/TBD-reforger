//! The typed identifiers of the API.
//!
//! **Role:** declares one newtype per id concept the API crates pass across a public boundary:
//! the key of every table a domain owns, the Discord snowflakes ([`DiscordUserId`] and its
//! siblings), the game runtime's own keys and the keys of the equipment datasets and ballistics
//! catalogs, so an event id and a mission id are distinct types the compiler keeps apart.
//! **Position:** the lowest API crate, depending on `newtype_ids` and sqlx only; every API
//! domain crate, the API application and its integration tests name their ids through it. The
//! modules group the ids by the domain that owns the table.
//! **Signals & state:** none; plain data types.
//! **Invariants:** every id is serde-transparent and `#[sqlx(transparent)]`, so its JSON, its
//! text form and its SQL bind are exactly those of the bare `Uuid`, `String` or integer it wraps; the
//! wire shapes, the goldens and the SQL stay byte-equal.

mod administration;
mod ballistics;
mod community_content;
mod discord;
mod equipment_datasets;
mod identity_and_access;
mod match_telemetry;
mod missions;
mod operations;
pub mod prelude;
mod server_infrastructure;

#[cfg(test)]
#[path = "tests/wire_identity.rs"]
mod tests;

pub use administration::{AuditLogEntryId, AuditTargetId, WarningId};
pub use ballistics::{BallisticsCatalogId, BallisticsShellId, BallisticsWeaponId};
pub use community_content::{
    AnnouncementId, ModGuid, ModpackId, ModpackModId, VehicleDatabaseId, WikiPageId, WorkshopItemId,
};
pub use discord::{
    DiscordClientId, DiscordGuildId, DiscordMessageId, DiscordRoleId, DiscordUserId,
};
pub use equipment_datasets::{
    EquipmentFieldId, EquipmentGenerationId, EquipmentNativeInstanceId, EquipmentNodeId,
    EquipmentResourceId,
};
pub use identity_and_access::{ArmaPlayerId, AuthenticationSessionId, RefreshTokenId};
pub use match_telemetry::{MatchEventId, MatchId, MatchPlayerStatId, SourceEventId, SourceMatchId};
pub use missions::{
    LoadoutModpackId, MissionArmoryId, MissionArtifactId, MissionDeploymentId, MissionId,
    MissionReviewCommentId, MissionReviewId, MissionSlotUid, MissionVersionId,
    RegistryCompatibilityId, RegistryItemId, SubmittedMissionId, UserFactionId,
};
pub use operations::{
    EventGroupId, EventId, EventMissionId, EventParticipantAllocationId, EventRegistrationId,
    FireMissionId, LeaveRequestId, LiveSlotOccupancyId, OrbatReservationId, OrbatSlotId,
    PlayerLifeId, SubmittedEventId,
};
pub use server_infrastructure::{
    FleetCommandId, MachineCredentialId, RuntimeSessionId, ScenarioId, ServerId,
    ServerStatusSampleId,
};
