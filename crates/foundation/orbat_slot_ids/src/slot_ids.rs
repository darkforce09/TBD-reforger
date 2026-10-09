//! The durable editor id and the derived wire id of one ORBAT slot.
//!
//! **Role:** declares [`SlotUid`] and [`SlotId`] with the `newtype_ids` string macro.
//! **Position:** the only module of `orbat_slot_ids`; re-exported at the crate root and in the
//! prelude.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each id serialises exactly as the string it wraps and is looked up by `&str`;
//! the editor id never changes over a slot's life, while the wire id is recomputed from the
//! slot's faction, callsign, role and occurrence on every compile.

newtype_ids::string_id! {
    /// A slot's derived wire identifier (`slots[].id`, `faction:callsign:role:occurrence`), which
    /// shifts under role renames and reorders.
    pub struct SlotId;
}

newtype_ids::string_id! {
    /// A slot's durable identity (`slots[].uid`): the editor's own slot id, which the mission
    /// document keys its slot rows by and which vehicle seats, squad leaders and members, and the
    /// VIP win rule reference.
    pub struct SlotUid;
}
