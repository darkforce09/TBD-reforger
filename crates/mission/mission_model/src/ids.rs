//! The identifiers a mission's compiled rows and authored blocks name.
//!
//! **Role:** declares one newtype identifier per referent: a mission, its template, a faction
//! preset, a zone, a radio net, a task, a trigger, a marker, a spawn module, an audio emitter, a
//! music cue and a tactical graphic. A slot's two ids, its derived wire id and its durable editor
//! identity, are `orbat_slot_ids`'s, which the rows name directly.
//! **Position:** the leaf of `mission_model`; every compiled row and authored block names its ids
//! through these types, and the payload compiler, the game-document compiler, the API and the
//! Mission Creator construct them from the strings they read.
//! **Signals & state:** none; plain data types.
//! **Invariants:** every id serialises and deserialises as its bare string (the `newtype_ids`
//! macros are serde-transparent), so the compiled document, the stored payloads, the API goldens
//! and every digest stay byte-identical; a field that names another row's id uses that row's id
//! type, so a slot's durable identity can never be passed where its derived wire id belongs.

newtype_ids::string_id! {
    /// A mission's identifier: the `meta.id` of the compiled document and the mission named by an
    /// export envelope.
    pub struct MissionId;
}

newtype_ids::string_id! {
    /// The identifier of the template a mission was authored from (`meta.templateId`).
    pub struct MissionTemplateId;
}

newtype_ids::string_id! {
    /// The identifier of the faction preset a compiled faction plays (`factions[].presetId`).
    pub struct FactionPresetId;
}

newtype_ids::string_id! {
    /// A zone's identifier (`zones[].id`), which spawn modules and the extraction win rule name.
    pub struct ZoneId;
}

newtype_ids::string_id! {
    /// A radio net's identifier (`net:` then lowercase letters, digits and underscores).
    pub struct NetId;
}

newtype_ids::string_id! {
    /// An authored task's identifier, unique across the `tasks` block.
    pub struct TaskId;
}

newtype_ids::string_id! {
    /// The identifier of a trigger a task, a spawn module or an audio emitter waits on.
    pub struct TriggerId;
}

newtype_ids::string_id! {
    /// The identifier of the map marker a task points at.
    pub struct MarkerId;
}

newtype_ids::string_id! {
    /// An authored spawn module's identifier, unique across the `spawnModules` block.
    pub struct SpawnModuleId;
}

newtype_ids::string_id! {
    /// An authored audio emitter's identifier, unique across the `audio.emitters` list.
    pub struct AudioEmitterId;
}

newtype_ids::string_id! {
    /// An authored music cue's identifier, unique across the `audio.musicCues` list.
    pub struct MusicCueId;
}

newtype_ids::string_id! {
    /// An authored tactical graphic's identifier, unique across the `tacticalGraphics` block.
    pub struct TacticalGraphicId;
}
