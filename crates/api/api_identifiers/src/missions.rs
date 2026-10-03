//! The identifiers of the missions domain: missions, their versions and artifacts, reviews,
//! deployments and the registry.
//!
//! **Role:** declares one typed id per missions table key, plus the text keys of the domain: a
//! compiled slot's uid, a loadout export's modpack id and a submitted, unparsed mission id.
//! **Position:** declared here, below every API crate; the missions domain owns the tables, and
//! the operations, telemetry and server code that names a mission or an artifact takes the same
//! type.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises, prints, parses and binds exactly as its inner UUID, text or
//! integer (transparent serde, `#[sqlx(transparent)]`), so the wire shapes and the SQL stay those
//! of the bare value.

newtype_ids::uuid_id! {
    sqlx,
    /// A mission of the library: the key of `missions`.
    pub struct MissionId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// One saved version of a mission: the key of `mission_versions`.
    pub struct MissionVersionId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A compiled, content-addressed build of a mission version: the key of `mission_artifacts`.
    pub struct MissionArtifactId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A review of a mission artifact: the key of `mission_reviews`.
    pub struct MissionReviewId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// One comment of a mission's review thread: the key of `mission_review_comments`.
    pub struct MissionReviewCommentId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A request to run a mission artifact on a game server: the key of `mission_deployments`.
    pub struct MissionDeploymentId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// One line of a mission's armory (a faction's item and quantity): the key of
    /// `mission_armories`.
    pub struct MissionArmoryId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// One prefab of the equipment registry: the key of `registry_items`.
    pub struct RegistryItemId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A compatibility edge between two registry nodes of a modpack: the key of
    /// `registry_compat`.
    pub struct RegistryCompatibilityId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A faction a member authored for the Mission Creator: the key of `user_factions`.
    pub struct UserFactionId;
}

newtype_ids::string_id! {
    sqlx,
    /// A slot's uid in a compiled mission artifact: the key the game runtime deploys a player
    /// into and the slot bindings of a deployment carry.
    pub struct MissionSlotUid;
}

newtype_ids::string_id! {
    /// The modpack identifier a loadout export names, as the exporter wrote it; free text, not
    /// necessarily a key of `modpacks`.
    pub struct LoadoutModpackId;
}

newtype_ids::string_id! {
    /// A mission id as a client submitted it, unparsed: the handler that reads it parses it into
    /// a [`MissionId`] and answers its own refusal when it is not one.
    pub struct SubmittedMissionId;
}
