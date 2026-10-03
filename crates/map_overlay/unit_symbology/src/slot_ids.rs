//! The identifier of one ORBAT slot as the squad links read it.
//!
//! **Role:** declares [`SlotId`], the id of the slot a squad leader or member occupies.
//! **Position:** `unit_symbology`; held by [`crate::squad_links::SquadLinkInput`], built by the
//! map engine's editing layer from the mission document's slot rows.
//! **Signals & state:** none; a plain data type.
//! **Invariants:** a slot id serialises exactly as the string it wraps and is looked up by
//! `&str`.

newtype_ids::string_id! {
    /// The id of one ORBAT slot, as the mission document spells it.
    pub struct SlotId;
}

#[cfg(test)]
#[path = "tests/slot_ids_tests.rs"]
mod tests;
