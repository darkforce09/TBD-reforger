//! Role: doc core.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Arc;
use super::Cell;
use super::Doc;
use super::MapRef;
use super::SideKeyMemo;
use super::UndoManager;

/// Authored leader, membership order, and faction side for one squad.
#[derive(Clone, Debug)]
pub struct SquadMembership {
    /// Leader slot id.
    pub leader_slot_id: String,

    /// Member slot ids.
    pub member_slot_ids: Vec<String>,

    /// Side.
    pub side: String,
}

/// Canonical client id bits value.
pub(super) const CLIENT_ID_BITS: u32 = 53;

/// Canonical local origin value.
pub(super) const LOCAL_ORIGIN: &str = "local-user";

/// Canonical init origin value.
pub(super) const INIT_ORIGIN: &str = "init";

/// `is_slot = true` rides the slot path (terrain bounds clamp, transform-lock refusal, the same z-policy as [`MissionDocCore::update_slot_position`]); `false` rides the vehicle path (no clamp, no lock, partial axes leave the others alone — rotation-only and translate-only both work). `None` on an axis means leave it. Unknown ids are skipped.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityTransformPatch {
    /// Id.
    pub id: String,

    /// Is slot.
    pub is_slot: bool,

    /// X.
    pub x: Option<f64>,

    /// Y.
    pub y: Option<f64>,

    /// Z.
    pub z: Option<f64>,

    /// Rotation.
    pub rotation: Option<f64>,
}

/// Own the mission CRDT roots, materialized projections, update exchange, and local undo history.
pub struct MissionDocCore {
    /// Doc.
    pub(super) doc: Doc,

    /// Slots.
    pub(super) slots: MapRef,

    /// Squads.
    pub(super) squads: MapRef,

    /// Factions.
    pub(super) factions: MapRef,

    /// Editor layers.
    pub(super) editor_layers: MapRef,

    /// Meta.
    pub(super) meta: MapRef,

    /// Vehicles.
    pub(super) vehicles: MapRef,

    /// Entities.
    pub(super) entities: MapRef,

    /// Zones.
    pub(super) zones: MapRef,

    /// Compositions.
    pub(super) compositions: MapRef,

    /// Triggers.
    pub(super) triggers: MapRef,

    /// Comments.
    pub(super) comments: MapRef,

    /// Connections.
    pub(super) connections: MapRef,

    /// Loadouts.
    pub(super) loadouts: MapRef,

    /// Items.
    pub(super) items: MapRef,

    /// Objectives.
    pub(super) objectives: MapRef,

    /// Markers.
    pub(super) markers: MapRef,

    /// Init mode.
    pub(super) init_mode: Cell<bool>,

    /// Undo mgr.
    pub(super) undo_mgr: UndoManager<()>,

    /// Undo groups.
    pub(super) undo_groups: Arc<crate::data::store::crdt::undo_groups::GroupingClock>,

    /// Undo cap hidden.
    pub(super) undo_cap_hidden: Cell<usize>,

    /// Side key resolutions.
    pub(super) side_key_resolutions: Cell<u64>,

    /// Side key memo.
    pub(super) side_key_memo: Option<SideKeyMemo>,
}
