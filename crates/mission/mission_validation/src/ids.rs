//! The identifiers a validation finding and its facts name.
//!
//! **Role:** declares the newtype identifiers of the validator: the rule a finding comes from, the
//! document row it is about, and a placed asset the catalogue may or may not hold.
//! **Position:** a leaf of `mission_validation`; [`crate::Finding`], [`crate::SelfCheckFailure`]
//! and [`crate::EvalContext`] name their ids through these types, the game-document compiler of
//! `mission_compiler` reuses them for its compile findings and kit substitutions, and the API and
//! the Mission Creator read them back as strings.
//! **Signals & state:** none; plain data types.
//! **Invariants:** every id serialises and deserialises as its bare string (the `newtype_ids`
//! macros are serde-transparent), so a finding written into a JSON body, an artifact's
//! diagnostics or a log line keeps its exact bytes.

newtype_ids::string_id! {
    /// A validation rule's identifier, such as `ORBAT-CALLSIGN-UNIQUE` or a compile diagnostic's
    /// `COMPILE-DROP-SLOT-RANK`: the stable half of a finding a consumer filters and groups on.
    pub struct RuleId;
}

newtype_ids::string_id! {
    /// The editor id of the document row a finding is about (a slot, squad, faction, vehicle,
    /// entity or zone), which the Mission Creator selects when the author clicks the finding.
    pub struct SubjectId;
}

newtype_ids::string_id! {
    /// A placed asset's identifier: an Enfusion resource name such as
    /// `{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et`, or a
    /// registry alias such as `veh:m151a2`.
    pub struct AssetId;
}
