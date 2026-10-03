//! Role: construction.
//! Position: the `rows` module of `mission_document`; public items re-exported at the root.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::ids::ClientId;

use super::Arc;
use super::CLIENT_ID_BITS;
use super::Cell;
use super::Doc;
use super::HashSet;
use super::LOCAL_ORIGIN;
use super::MissionDocCore;
use super::Origin;
use super::SideKeyMemo;
use super::UndoManager;

impl MissionDocCore {
    /// A fresh, empty document with the tracked root maps + an undo manager scoped to them, on a **randomized client id** — the constructor every peer uses.
    #[must_use]
    pub fn new() -> Self {
        Self::from_doc(Doc::new())
    }
}

impl MissionDocCore {
    /// A fresh, empty document that identifies as `client_id` — the **deterministic** constructor.
    #[must_use]
    pub fn with_client_id(client_id: impl Into<ClientId>) -> Self {
        let client_id: u64 = client_id.into().get();
        debug_assert!(
            client_id >> CLIENT_ID_BITS == 0,
            "yrs client ids are {CLIENT_ID_BITS}-bit; {client_id} would be truncated"
        );
        Self::from_doc(Doc::with_client_id(client_id))
    }
}

impl MissionDocCore {
    /// A document over `doc` with the default undo clock.
    pub(super) fn from_doc(doc: Doc) -> Self {
        Self::from_doc_with_clock(doc, default_undo_clock())
    }
}

impl MissionDocCore {
    /// A fresh, empty document whose undo history reads its time from `clock`: the host clock
    /// seam. The gesture window and explicit groups are measured on it (floored to 1 ms).
    #[must_use]
    pub fn with_undo_clock(clock: Arc<dyn time_source::Clock>) -> Self {
        Self::from_doc_with_clock(Doc::new(), clock)
    }
}

/// The inner undo clock of a document whose host injects none: the platform wall clock
/// (`Date.now()` in a browser). In this crate's unit tests, a clock that steps past the gesture
/// window on every read, so each transaction is its own undo step unless grouped.
fn default_undo_clock() -> Arc<dyn time_source::Clock> {
    #[cfg(test)]
    {
        Arc::new(crate::test_fixtures::SteppingClock::past_the_gesture_window())
    }
    #[cfg(not(test))]
    {
        mission_crdt::undo_groups::platform_clock()
    }
}

impl MissionDocCore {
    /// A document over `doc`: the tracked root maps, and an undo manager scoped to them whose
    /// timestamps come from `inner_clock` through a grouping clock.
    pub(super) fn from_doc_with_clock(doc: Doc, inner_clock: Arc<dyn time_source::Clock>) -> Self {
        let side_key_memo = SideKeyMemo::install(&doc);
        let slots = doc.get_or_insert_map("slots");
        let squads = doc.get_or_insert_map("squads");
        let factions = doc.get_or_insert_map("factions");
        let editor_layers = doc.get_or_insert_map("editorLayers");
        let meta = doc.get_or_insert_map("meta");
        let vehicles = doc.get_or_insert_map("vehicles");
        let entities = doc.get_or_insert_map("entities");
        let zones = doc.get_or_insert_map("zones");
        let compositions = doc.get_or_insert_map("compositions");
        let triggers = doc.get_or_insert_map("triggers");
        let comments = doc.get_or_insert_map("comments");
        let connections = doc.get_or_insert_map("connections");
        let loadouts = doc.get_or_insert_map("loadouts");
        let items = doc.get_or_insert_map("items");
        let objectives = doc.get_or_insert_map("objectives");
        let markers = doc.get_or_insert_map("markers");

        let undo_groups = mission_crdt::undo_groups::GroupingClock::wrap(inner_clock);
        let opts = mission_crdt::undo_groups::undo_options(
            undo_groups.clone(),
            HashSet::from([Origin::from(LOCAL_ORIGIN)]),
        );
        let mut undo_mgr = UndoManager::with_options(opts);
        undo_mgr.expand_scope(&doc, &slots);
        undo_mgr.expand_scope(&doc, &squads);
        undo_mgr.expand_scope(&doc, &factions);
        undo_mgr.expand_scope(&doc, &editor_layers);
        undo_mgr.expand_scope(&doc, &meta);
        undo_mgr.expand_scope(&doc, &vehicles);
        undo_mgr.expand_scope(&doc, &entities);
        undo_mgr.expand_scope(&doc, &zones);

        undo_mgr.expand_scope(&doc, &compositions);

        undo_mgr.expand_scope(&doc, &triggers);

        undo_mgr.expand_scope(&doc, &comments);

        undo_mgr.expand_scope(&doc, &connections);
        undo_mgr.expand_scope(&doc, &loadouts);
        undo_mgr.expand_scope(&doc, &items);
        undo_mgr.expand_scope(&doc, &objectives);
        undo_mgr.expand_scope(&doc, &markers);

        Self {
            doc,
            slots,
            squads,
            factions,
            editor_layers,
            meta,
            vehicles,
            entities,
            zones,
            compositions,
            triggers,
            comments,
            connections,
            loadouts,
            items,
            objectives,
            markers,
            init_mode: Cell::new(false),
            undo_mgr,
            undo_groups,
            undo_cap_hidden: Cell::new(0),
            side_key_resolutions: Cell::new(0),
            side_key_memo,
        }
    }
}

impl Default for MissionDocCore {
    fn default() -> Self {
        Self::new()
    }
}
