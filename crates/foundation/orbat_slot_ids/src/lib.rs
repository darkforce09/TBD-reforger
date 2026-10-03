//! The two identifiers of one ORBAT slot.
//!
//! **Role:** declares [`SlotUid`], a slot's durable editor id (`slots[].uid`), and [`SlotId`],
//! its derived wire id (`slots[].id`), so no caller can pass one where the other belongs.
//! **Position:** foundation tier 1, over `newtype_ids` only. The mission model's compiled rows,
//! the mission document's slot rows, the authoring operations, the game-document compiler and the
//! map's squad links name their slot ids through it.
//! **Signals & state:** none; plain data types.
//! **Invariants:** both ids serialise and deserialise as the bare string they wrap, so every
//! stored payload, compiled document, API golden and digest stays byte-identical whichever type
//! carries the string.

mod slot_ids;

pub mod prelude;

pub use slot_ids::{SlotId, SlotUid};
