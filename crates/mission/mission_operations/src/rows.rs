//! Role: rows.
//! Position: the `rows` module of `mission_operations`; hosted commands drive it.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use mission_document::ids::{CommentId, FactionId, LayerId, SquadId};
use mission_model::ids::SlotId;

/// An `editorLayers` row, as carried by the doc's `small_maps_json()` → `editorLayersById`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerRow {
    /// Id.
    pub id: LayerId,

    /// Name.
    pub name: String,

    /// Parent id.
    pub parent_id: Option<LayerId>,

    /// Entity ids.
    pub entity_ids: Vec<String>,

    /// Hidden.
    pub hidden: bool,

    /// Locked.
    pub locked: bool,
}

/// The two slot fields the tree needs, adapted from the materialized SoA.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotRow {
    /// Id.
    pub id: SlotId,

    /// Role.
    pub role: String,
}

/// Domain representation of comment row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommentRow {
    /// Id.
    pub id: CommentId,

    /// Title.
    pub title: String,

    /// Tooltip.
    pub tooltip: String,
}

/// A `factions` row from the doc's `small_maps_json()` → `factionsById`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactionRow {
    /// Id.
    pub id: FactionId,

    /// Key.
    pub key: String,

    /// Name.
    pub name: String,

    /// Ordered squad ids under this faction (`faction.squadIds`).
    pub squad_ids: Vec<String>,
}

/// A `squads` row from the doc's `small_maps_json()` → `squadsById`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SquadRow {
    /// Id.
    pub id: SquadId,

    /// Name.
    pub name: String,

    /// Faction id.
    pub faction_id: FactionId,

    /// Ordered slot ids in this squad (`squad.slotIds`).
    pub slot_ids: Vec<String>,

    /// Leader slot id.
    pub leader_slot_id: SlotId,

    /// Vehicle ids.
    pub vehicle_ids: Vec<String>,
}
