//! Role: construction.
//! Position: `doc/store` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

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
    pub fn with_client_id(client_id: u64) -> Self {
        debug_assert!(
            client_id >> CLIENT_ID_BITS == 0,
            "yrs client ids are {CLIENT_ID_BITS}-bit; {client_id} would be truncated"
        );
        Self::from_doc(Doc::with_client_id(client_id))
    }
}

impl MissionDocCore {
    /// From doc using the supplied domain data.
    pub(super) fn from_doc(doc: Doc) -> Self {
        Self::from_doc_with_clock(
            doc,
            crate::data::store::crdt::undo_groups::default_inner_clock(),
        )
    }
}

impl MissionDocCore {
    /// With undo clock using the supplied domain data.
    #[must_use]
    pub fn with_undo_clock(clock: Arc<dyn yrs::sync::Clock>) -> Self {
        Self::from_doc_with_clock(Doc::new(), clock)
    }
}

impl MissionDocCore {
    /// From doc with clock using the supplied domain data.
    pub(super) fn from_doc_with_clock(doc: Doc, inner_clock: Arc<dyn yrs::sync::Clock>) -> Self {
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

        let undo_groups = crate::data::store::crdt::undo_groups::GroupingClock::wrap(inner_clock);
        let opts = crate::data::store::crdt::undo_groups::undo_options(
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
