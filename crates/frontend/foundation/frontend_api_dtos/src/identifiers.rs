//! The typed identifiers every API DTO, endpoint and consumer holds instead of a bare string or
//! integer.
//!
//! **Role:** one newtype per identifier kind the API sends or accepts, named as the API's own
//! identifier crate names the same concept (`MissionId`, `EventMissionId`, `ServerId`,
//! `DiscordUserId`, …), plus self-describing names for the kinds only the single-page app holds.
//! **Position:** the leaf of the DTO tree; every DTO module, the endpoints, the SSE frames, the
//! session and the pages name these types. It imports nothing from the rest of the app.
//! **Signals & state:** none; plain data types.
//! **Invariants:** every identifier is `#[serde(transparent)]` over the exact wire type (a
//! `String` for every text id, never a parsed UUID, so a golden's bytes round-trip unchanged; an
//! integer of the wire width for the numeric ids); the text an identifier displays, compares and
//! hashes as is exactly its wire string.

/// Whether a text identifier is the empty string, for `skip_serializing_if` on the fields the
/// API omits while they are blank.
pub fn identifier_is_empty<Id: AsRef<str>>(identifier: &Id) -> bool {
    identifier.as_ref().is_empty()
}

// Missions, their versions, compiled artifacts, reviews and deployments.

newtype_ids::string_id! {
    /// A mission's identifier.
    #[derive(Default)]
    pub struct MissionId;
}

newtype_ids::string_id! {
    /// One saved version of a mission.
    #[derive(Default)]
    pub struct MissionVersionId;
}

newtype_ids::string_id! {
    /// One compiled artifact of a mission version.
    #[derive(Default)]
    pub struct MissionArtifactId;
}

newtype_ids::string_id! {
    /// One review decision on a mission submission.
    #[derive(Default)]
    pub struct MissionReviewId;
}

newtype_ids::string_id! {
    /// One comment in a mission's review thread.
    #[derive(Default)]
    pub struct MissionReviewCommentId;
}

newtype_ids::string_id! {
    /// One deployment of a mission artifact to a game server.
    #[derive(Default)]
    pub struct MissionDeploymentId;
}

newtype_ids::string_id! {
    /// One item a mission's armoury offers.
    #[derive(Default)]
    pub struct MissionArmoryId;
}

newtype_ids::string_id! {
    /// The Enfusion mission header resource a game server boots.
    #[derive(Default)]
    pub struct ScenarioId;
}

newtype_ids::string_id! {
    /// The validation rule a compiler finding comes from.
    #[derive(Default)]
    pub struct ValidationRuleId;
}

newtype_ids::string_id! {
    /// The mission entity a compiler finding names (a slot, a vehicle, …).
    #[derive(Default)]
    pub struct ValidationSubjectId;
}

// The arsenal registry.

newtype_ids::string_id! {
    /// One item of the arsenal registry.
    #[derive(Default)]
    pub struct RegistryItemId;
}

newtype_ids::string_id! {
    /// One compatibility edge between two registry items.
    #[derive(Default)]
    pub struct RegistryCompatibilityId;
}

newtype_ids::string_id! {
    /// One faction a user owns.
    #[derive(Default)]
    pub struct UserFactionId;
}

// Operations: events, their missions, slots, groups, registrations and fire missions.

newtype_ids::string_id! {
    /// A scheduled event's identifier.
    #[derive(Default)]
    pub struct EventId;
}

newtype_ids::string_id! {
    /// One mission scheduled inside an event.
    #[derive(Default)]
    pub struct EventMissionId;
}

newtype_ids::string_id! {
    /// One slot of an event mission's ORBAT.
    #[derive(Default)]
    pub struct OrbatSlotId;
}

newtype_ids::string_id! {
    /// One access group of an event.
    #[derive(Default)]
    pub struct EventGroupId;
}

newtype_ids::string_id! {
    /// One registration of a member for an event mission.
    #[derive(Default)]
    pub struct EventRegistrationId;
}

newtype_ids::string_id! {
    /// One leave request a member filed.
    #[derive(Default)]
    pub struct LeaveRequestId;
}

newtype_ids::string_id! {
    /// One saved fire mission.
    #[derive(Default)]
    pub struct FireMissionId;
}

// Game servers, their credentials, fleet commands, runtime sessions and matches.

newtype_ids::string_id! {
    /// A registered game server.
    #[derive(Default)]
    pub struct ServerId;
}

newtype_ids::string_id! {
    /// One issued machine credential of a game server.
    #[derive(Default)]
    pub struct MachineCredentialId;
}

newtype_ids::string_id! {
    /// One fleet command queued for a game server.
    #[derive(Default)]
    pub struct FleetCommandId;
}

newtype_ids::string_id! {
    /// One game-runtime session of a game server.
    #[derive(Default)]
    pub struct RuntimeSessionId;
}

newtype_ids::string_id! {
    /// One match a game server played.
    #[derive(Default)]
    pub struct MatchId;
}

newtype_ids::string_id! {
    /// One event inside a match, unique within the match.
    #[derive(Default)]
    pub struct MatchEventId;
}

// Community content.

newtype_ids::string_id! {
    /// One announcement.
    #[derive(Default)]
    pub struct AnnouncementId;
}

newtype_ids::string_id! {
    /// One modpack.
    #[derive(Default)]
    pub struct ModpackId;
}

newtype_ids::string_id! {
    /// One mod inside a modpack.
    #[derive(Default)]
    pub struct ModpackModId;
}

newtype_ids::string_id! {
    /// One Arma Reforger Workshop item.
    #[derive(Default)]
    pub struct WorkshopItemId;
}

newtype_ids::string_id! {
    /// One row of the vehicle database.
    #[derive(Default)]
    pub struct VehicleDatabaseId;
}

newtype_ids::string_id! {
    /// One wiki article.
    #[derive(Default)]
    pub struct WikiPageId;
}

// Discord and Arma identities.

newtype_ids::string_id! {
    /// A Discord user's snowflake: a member, an author, an owner, an actor.
    #[derive(Default)]
    pub struct DiscordUserId;
}

newtype_ids::string_id! {
    /// One Discord chat message.
    #[derive(Default)]
    pub struct DiscordMessageId;
}

newtype_ids::string_id! {
    /// One Discord guild.
    #[derive(Default)]
    pub struct DiscordGuildId;
}

newtype_ids::string_id! {
    /// An Arma Reforger player's identity.
    #[derive(Default)]
    pub struct ArmaPlayerId;
}

// Administration.

newtype_ids::integer_id! {
    /// One audit log line, in allocation order.
    #[derive(Default)]
    pub struct AuditLogEntryId(i64);
}

newtype_ids::string_id! {
    /// The entity an audit log line names.
    #[derive(Default)]
    pub struct AuditTargetId;
}

newtype_ids::string_id! {
    /// One signed-in session of the account.
    #[derive(Default)]
    pub struct AuthenticationSessionId;
}

newtype_ids::string_id! {
    /// The id of a server-sent event frame, echoed as `Last-Event-ID` on a reconnect.
    #[derive(Default)]
    pub struct ServerSentEventId;
}

// Ballistics catalogs.

newtype_ids::string_id! {
    /// The lowercase slug naming a ballistics catalog across its versions.
    #[derive(Default)]
    pub struct BallisticsCatalogId;
}

newtype_ids::string_id! {
    /// One weapon of a ballistics catalog.
    #[derive(Default)]
    pub struct BallisticsWeaponId;
}

newtype_ids::string_id! {
    /// One shell of a ballistics catalog weapon.
    #[derive(Default)]
    pub struct BallisticsShellId;
}

newtype_ids::string_id! {
    /// The Enfusion export generation a ballistics catalog came from.
    #[derive(Default)]
    pub struct BallisticsExportGenerationId;
}

newtype_ids::string_id! {
    /// The calibration case or provenance check a catalog upload failure names.
    #[derive(Default)]
    pub struct CalibrationCaseId;
}

// The equipment data viewer.

newtype_ids::string_id! {
    /// One import generation of an equipment dataset.
    pub struct EquipmentGenerationId;
}

newtype_ids::string_id! {
    /// One resource of an equipment dataset generation.
    pub struct EquipmentResourceId;
}

newtype_ids::string_id! {
    /// One native container node of an equipment resource.
    pub struct EquipmentNodeId;
}

newtype_ids::integer_id! {
    /// One native field of the equipment field inventory.
    pub struct EquipmentFieldId(u64);
}
