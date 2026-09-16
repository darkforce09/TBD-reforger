//! Role: rows.
//! Position: `doc/operations` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// An `editorLayers` row, as carried by the doc's `small_maps_json()` → `editorLayersById`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerRow {
    /// Id.
    pub id: String,

    /// Name.
    pub name: String,

    /// Parent id.
    pub parent_id: Option<String>,

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
    pub id: String,

    /// Role.
    pub role: String,
}

/// Domain representation of comment row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommentRow {
    /// Id.
    pub id: String,

    /// Title.
    pub title: String,

    /// Tooltip.
    pub tooltip: String,
}

/// A `factions` row from the doc's `small_maps_json()` → `factionsById`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactionRow {
    /// Id.
    pub id: String,

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
    pub id: String,

    /// Name.
    pub name: String,

    /// Faction id.
    pub faction_id: String,

    /// Ordered slot ids in this squad (`squad.slotIds`).
    pub slot_ids: Vec<String>,

    /// Leader slot id.
    pub leader_slot_id: String,

    /// Vehicle ids.
    pub vehicle_ids: Vec<String>,
}
