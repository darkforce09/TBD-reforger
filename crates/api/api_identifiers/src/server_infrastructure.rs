//! The identifiers of the server infrastructure domain: game servers, their machine
//! credentials, runtime sessions and fleet commands.
//!
//! **Role:** declares one typed id per server infrastructure table key, plus the mission header
//! resource name a server boots.
//! **Position:** declared here, below every API crate; server infrastructure owns the tables,
//! and the missions, operations and telemetry code that names a server or a runtime session
//! takes the same type.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises, prints, parses and binds exactly as its inner UUID, text or
//! integer (transparent serde, `#[sqlx(transparent)]`), so the wire shapes and the SQL stay those
//! of the bare value.

newtype_ids::uuid_id! {
    sqlx,
    /// A registered game server: the key of `servers`.
    pub struct ServerId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A machine credential a game server's host signs in with: the key of
    /// `server_machine_credentials`.
    pub struct MachineCredentialId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// One boot of the game runtime on a server: the key of `server_runtime_sessions`.
    pub struct RuntimeSessionId;
}

newtype_ids::uuid_id! {
    sqlx,
    /// A command queued for a server's host agent: the key of `fleet_commands`.
    pub struct FleetCommandId;
}

newtype_ids::integer_id! {
    sqlx,
    /// One status sample of a server: the `bigint` sequence key of `server_status_history`.
    pub struct ServerStatusSampleId(i64);
}

newtype_ids::string_id! {
    sqlx,
    /// The Enfusion resource name of a mission header a game server boots (Reforger's
    /// `scenarioId`), such as `{GUID}Missions/Name.conf`.
    pub struct ScenarioId;
}
