//! `MissionDocCore` — owns a `yrs` document + its tracked root maps. It applies Yjs-wire update
//! byte-streams (criterion 2), encodes/decodes the update stream (criterion 3), materializes the slot
//! SoA (criterion 1), and drives undo/redo (criterion 4). The write mutators (`add_slot` /
//! `set_slot_position` / `remove_slot`) exist to exercise the `UndoManager`; the full `state/ydoc.ts`
//! mutator surface is ported at the 3.1 cutover, not in the spike.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use yrs::sync::{Clock, Timestamp};
use yrs::types::ToJson;
use yrs::undo::{Options as UndoOptions, UndoManager};
use yrs::updates::decoder::Decode;
use yrs::{
    Any, Doc, Map, MapPrelim, MapRef, Origin, Out, ReadTxn, StateVector, Transact, TransactionMut,
    Update,
};

use super::soa::{Interner, NONE_IDX, STANCE_CROUCH, STANCE_PRONE, STANCE_STAND, SlotSoa};
use crate::squad_links::SquadLinkInput;

/// Fixed, deterministic client id — so `encode_state` and the undo/redo sequence are reproducible
/// (parity for criteria 3/4). A client-id clash with an incoming peer update is harmless: `yrs`
/// keys blocks by the *originating* client, and the spike doc never co-authors a slot with a peer.
const CLIENT_ID: u64 = 1;

/// Transaction origins for undo scoping — mirror `ydoc.ts`'s `LOCAL_ORIGIN` / `INIT_ORIGIN`. Only
/// `LOCAL` is undo-tracked (in `tracked_origins`); `INIT` (seed / hydrate / persistence restore) is
/// not. yrs captures a transaction when its origin is in `tracked_origins`; with more than the undo
/// manager's own origin present, an un-stamped (no-origin) transaction is also skipped (undo.rs
/// `should_skip`), which is why every mutator stamps an origin.
const LOCAL_ORIGIN: &str = "local-user";
const INIT_ORIGIN: &str = "init";

/// A constant clock. With `capture_timeout_millis = 0` the undo manager never extends a stack item,
/// so the timestamp value is irrelevant — and building `undo::Options` explicitly (rather than via
/// `Options::default()`, which is `#[cfg(not(target_family = "wasm"))]` because its default
/// `SystemClock` needs std time) is what lets the core compile for `wasm32-unknown-unknown`.
struct ZeroClock;

impl Clock for ZeroClock {
    fn now(&self) -> Timestamp {
        0
    }
}

/// The `yrs`-backed document core. `slots` is a root map of nested per-slot maps; `editor_layers` is
/// the root map whose `entityIds` arrays give each slot its Outliner folder — the `state/ydoc.ts`
/// shape, materialized into a [`SlotSoa`].
pub struct MissionDocCore {
    doc: Doc,
    slots: MapRef,
    squads: MapRef,
    factions: MapRef,
    editor_layers: MapRef,
    meta: MapRef,
    /// Root `vehicles` map (`vehiclesById` in [`Self::small_maps_json`]) — undo-scoped (T-180.2).
    vehicles: MapRef,
    /// Root `entities` map (`entitiesById` in [`Self::small_maps_json`]) — undo-scoped (T-254).
    /// Mission-placed world objects (props/crates/compositions) for schema `entities[]`.
    entities: MapRef,
    /// When true, mutators stamp `INIT` (untracked) instead of `LOCAL` — set around boot / hydrate /
    /// default-seeding so a load is not an undo step. Interior mutability: mutators take `&self`.
    init_mode: Cell<bool>,
    /// `M = ()`: no per-stack-item metadata needed.
    undo_mgr: UndoManager<()>,
}

impl MissionDocCore {
    /// A fresh, empty document with the two tracked root maps + an undo manager scoped to both.
    #[must_use]
    pub fn new() -> Self {
        let doc = Doc::with_client_id(CLIENT_ID);
        let slots = doc.get_or_insert_map("slots");
        let squads = doc.get_or_insert_map("squads");
        let factions = doc.get_or_insert_map("factions");
        let editor_layers = doc.get_or_insert_map("editorLayers");
        let meta = doc.get_or_insert_map("meta");
        let vehicles = doc.get_or_insert_map("vehicles");
        let entities = doc.get_or_insert_map("entities");

        // capture_timeout_millis = 0 → every transaction is its own undo step. yrs extends the last
        // stack item only when `last_change > 0 && now - last_change < capture_timeout_millis`
        // (undo.rs `handle_after_transaction`); `u64 < 0` is never true, and ZeroClock pins
        // `last_change` to 0 besides — so no same-millisecond merge, on either guard. This matches
        // driving the JS `Y.UndoManager` with `{ captureTimeout: 0 }` (Yjs uses the same `<`), the
        // basis for criterion-4 parity.
        //
        // T-159.22.1 pinned this empirically after T-159.22 reported it violated: see
        // `two_local_moves_are_two_undo_steps` / `two_local_places_are_two_undo_steps` below, which
        // assert `undo_depth()` across a step boundary on native AND (via the editor's undo gate) on
        // wasm. The report was a gate-driver artifact, not a core defect.
        let opts = UndoOptions::<()> {
            capture_timeout_millis: 0,
            // Track only LOCAL — user gestures are undoable; INIT (seed / hydrate / restore) is not.
            // `expand_scope` also adds the manager's own origin, so no-origin txns are skipped too.
            tracked_origins: HashSet::from([Origin::from(LOCAL_ORIGIN)]),
            capture_transaction: None,
            timestamp: Arc::new(ZeroClock),
            init_undo_stack: Vec::new(),
            init_redo_stack: Vec::new(),
        };
        let mut undo_mgr = UndoManager::with_options(opts);
        undo_mgr.expand_scope(&doc, &slots);
        undo_mgr.expand_scope(&doc, &squads);
        undo_mgr.expand_scope(&doc, &factions);
        undo_mgr.expand_scope(&doc, &editor_layers);
        undo_mgr.expand_scope(&doc, &meta);
        undo_mgr.expand_scope(&doc, &vehicles);
        undo_mgr.expand_scope(&doc, &entities);

        Self {
            doc,
            slots,
            squads,
            factions,
            editor_layers,
            meta,
            vehicles,
            entities,
            init_mode: Cell::new(false),
            undo_mgr,
        }
    }

    /// Toggle init-mode: while true, mutators stamp `INIT` (untracked). The JS wrapper brackets boot /
    /// hydrate / default-seeding with `set_origin_init(true)` … `set_origin_init(false)` so a load is
    /// never an undo step (mirrors `ydoc.ts` running those under `INIT_ORIGIN`).
    pub fn set_origin_init(&self, on: bool) {
        self.init_mode.set(on);
    }

    /// Open a write transaction stamped with the current origin (`INIT` in init-mode, else `LOCAL`).
    /// Every mutator uses this so undo tracks exactly the local user gestures.
    fn begin(&self) -> yrs::TransactionMut<'_> {
        let origin = if self.init_mode.get() {
            INIT_ORIGIN
        } else {
            LOCAL_ORIGIN
        };
        self.doc.transact_mut_with(origin)
    }

    /// Apply a Yjs-wire (v1) update byte-stream — the exact bytes `Y.encodeStateAsUpdate(doc)` emits.
    ///
    /// # Errors
    /// Returns a message on a malformed update or an integration failure.
    pub fn apply_update(&self, bytes: &[u8]) -> Result<(), String> {
        let update = Update::decode_v1(bytes).map_err(|e| e.to_string())?;
        // Always INIT (untracked) regardless of mode — a persistence restore / peer sync is never an
        // undo step. (`begin()` would honor init-mode, but forcing INIT keeps this correct off-boot.)
        let mut txn = self.doc.transact_mut_with(INIT_ORIGIN);
        txn.apply_update(update).map_err(|e| e.to_string())
    }

    /// Encode the whole document as a Yjs-wire (v1) update stream — the persistence blob (criterion 3)
    /// and the seed a fresh peer replays. Deterministic given the fixed client id.
    #[must_use]
    pub fn encode_state(&self) -> Vec<u8> {
        self.doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default())
    }

    /// Serialize the 8 small root maps + `meta` to one JSON object shaped like the store's
    /// `MapSnapshot` minus `slotsById` (slots ride the fast SoA getters). The 367k-slot hot path never
    /// runs this — these maps hold hundreds of entities. `meta` is `null` when empty (matching
    /// `docToSnapshot`). Enables migrating every non-render reader (compile, Outliner, Attributes) onto
    /// the shadow (Phase 3.2.2).
    ///
    /// When hydrate parked unknown top-level payload keys (T-219), they appear here as
    /// `payloadExtras` — a compile side-channel, never a wire key itself.
    ///
    /// **T-220 — `entityOrder`:** hydrate records authored array id-order here (yrs maps do not),
    /// so `compile_payload` can emit `editor.slots` / factions / … in the original sequence.
    #[must_use]
    pub fn small_maps_json(&self) -> String {
        // Grab the root handles before opening the read txn (`get_or_insert_map` takes `&self`).
        let meta = self.doc.get_or_insert_map("meta");
        let payload_extras = self.doc.get_or_insert_map("payloadExtras");
        let entity_order = self.doc.get_or_insert_map("entityOrder");
        let named: [(&str, MapRef); 9] = [
            ("factionsById", self.doc.get_or_insert_map("factions")),
            ("squadsById", self.doc.get_or_insert_map("squads")),
            ("loadoutsById", self.doc.get_or_insert_map("loadouts")),
            ("itemsById", self.doc.get_or_insert_map("items")),
            ("objectivesById", self.doc.get_or_insert_map("objectives")),
            ("vehiclesById", self.doc.get_or_insert_map("vehicles")),
            ("entitiesById", self.doc.get_or_insert_map("entities")),
            ("markersById", self.doc.get_or_insert_map("markers")),
            (
                "editorLayersById",
                self.doc.get_or_insert_map("editorLayers"),
            ),
        ];

        let txn = self.doc.transact();
        let mut root: HashMap<String, Any> = HashMap::new();
        root.insert(
            "meta".to_string(),
            if meta.len(&txn) == 0 {
                Any::Null
            } else {
                meta.to_json(&txn)
            },
        );
        for (key, map) in &named {
            root.insert((*key).to_string(), map.to_json(&txn));
        }
        // Omit when empty so a clean doc's snapshot shape stays unchanged.
        if payload_extras.len(&txn) > 0 {
            root.insert("payloadExtras".to_string(), payload_extras.to_json(&txn));
        }
        if entity_order.len(&txn) > 0 {
            root.insert("entityOrder".to_string(), entity_order.to_json(&txn));
        }

        let mut buf = String::new();
        Any::Map(Arc::new(root)).to_json(&mut buf);
        buf
    }

    /// The `slots` map as a JSON object (`slotsById`) — full, **exact-f64** `Slot`s for the non-render
    /// readers (compile / persistence / the store mirror). Together with `small_maps_json` this
    /// reproduces the entire `MapSnapshot`. O(n) JSON — a one-shot (save), never the render hot path,
    /// which reads the f32 SoA (positions there are f32-truncated, fine for pixels, lossy for compile).
    #[must_use]
    pub fn slots_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.slots.to_json(&txn).to_json(&mut buf);
        buf
    }

    /// Materialize every slot into the columnar [`SlotSoa`] (criterion 1). Keyed by `ids[row]`.
    #[must_use]
    pub fn materialize(&self) -> SlotSoa {
        let txn = self.doc.transact();

        // slotId -> layerId: the first Outliner folder whose `entityIds` lists the slot.
        let mut slot_layer: HashMap<String, String> = HashMap::new();
        for (layer_id, out) in self.editor_layers.iter(&txn) {
            if let Out::YMap(layer) = out
                && let Some(Out::Any(Any::Array(arr))) = layer.get(&txn, "entityIds")
            {
                for a in arr.iter() {
                    if let Any::String(sid) = a {
                        slot_layer
                            .entry(sid.to_string())
                            .or_insert_with(|| layer_id.to_string());
                    }
                }
            }
        }

        let mut soa = SlotSoa::default();
        let mut roles = Interner::new();
        let mut tags = Interner::new();
        let mut squads = Interner::new();
        let mut layers = Interner::new();

        for (id, out) in self.slots.iter(&txn) {
            let Out::YMap(slot) = out else { continue };
            let (x, y, z, rot) = read_position(&txn, &slot);
            soa.ids.push(id.to_string());
            soa.xs.push(x as f32);
            soa.ys.push(y as f32);
            soa.xy.push(x as f32);
            soa.xy.push(y as f32);
            soa.zs.push(z as f32);
            soa.rotations.push(rot as f32);
            soa.stance.push(read_stance(&txn, &slot));
            soa.role_idx
                .push(roles.intern(read_str(&txn, &slot, "role").as_deref().unwrap_or("")));
            soa.tag_idx.push(match read_str(&txn, &slot, "tag") {
                Some(t) => tags.intern(&t),
                None => NONE_IDX,
            });
            let squad_id = read_str(&txn, &slot, "squadId").unwrap_or_default();
            soa.squad_idx.push(squads.intern(&squad_id));
            soa.layer_idx.push(match slot_layer.get(id) {
                Some(l) => layers.intern(l),
                None => NONE_IDX,
            });
            // T-180.3 — slot → squad.factionId → faction.key (missing hop → BLUFOR).
            soa.side_keys.push(resolve_slot_side_key(
                &txn,
                &self.squads,
                &self.factions,
                &squad_id,
            ));
        }

        soa.roles = roles.words;
        soa.tags = tags.words;
        soa.squads = squads.words;
        soa.layers = layers.words;
        soa
    }

    /// T-180.4 — collect per-squad leader / members / side for [`crate::squad_links::build_squad_link_segments`].
    /// No segment math here — geometry stays in `squad_links`.
    #[must_use]
    pub fn squad_link_inputs(&self) -> Vec<SquadLinkInput> {
        let txn = self.doc.transact();
        let mut out = Vec::new();
        for (squad_id, out_v) in self.squads.iter(&txn) {
            let Out::YMap(sq) = out_v else {
                continue;
            };
            let leader_slot_id = read_str(&txn, &sq, "leaderSlotId").unwrap_or_default();
            let member_slot_ids: Vec<String> =
                read_id_array(&txn, &self.squads, squad_id, "slotIds")
                    .iter()
                    .filter_map(|a| match a {
                        Any::String(s) => Some(s.to_string()),
                        _ => None,
                    })
                    .collect();
            let side = resolve_slot_side_key(&txn, &self.squads, &self.factions, squad_id);
            out.push(SquadLinkInput {
                leader_slot_id,
                member_slot_ids,
                side,
            });
        }
        out
    }

    /// Add a slot with full fidelity — the complete `Slot` map, appended to `squad.slotIds` and
    /// filed under `layer.entityIds`. Mirrors `ydoc.addSlot` @139. The `ensureDefaultSquad` /
    /// `ensureDefaultLayer` orchestration stays JS-side (JS mints the faction/squad/layer ids and
    /// creates them via `add_faction`/`add_squad`/`add_editor_layer`), so this receives concrete
    /// `squad_id`/`layer_id` + `index` (the squad's current slot count). `tag`/`asset_id` write only
    /// when present (non-empty), matching ydoc's `...(x ? {x} : {})` spread (key omitted otherwise).
    /// The squad/layer appends are guarded so a slot with a not-yet-created container still stores.
    #[allow(clippy::too_many_arguments)]
    pub fn add_slot(
        &self,
        id: &str,
        squad_id: &str,
        layer_id: &str,
        index: u32,
        role: &str,
        tag: Option<String>,
        asset_id: Option<String>,
        x: f64,
        y: f64,
        z: f64,
        rotation: f64,
    ) {
        let mut txn = self.begin();
        let slot = self
            .slots
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        slot.insert(&mut txn, "squadId", squad_id);
        slot.insert(&mut txn, "index", Any::BigInt(i64::from(index)));
        slot.insert(&mut txn, "role", role);
        if let Some(t) = tag.filter(|s| !s.is_empty()) {
            slot.insert(&mut txn, "tag", t);
        }
        if let Some(a) = asset_id.filter(|s| !s.is_empty()) {
            slot.insert(&mut txn, "assetId", a);
        }
        slot.insert(&mut txn, "position", position_any(x, y, z, rotation));
        slot.insert(&mut txn, "stance", "stand");
        slot.insert(&mut txn, "loadoutId", Any::Null);
        append_id(&mut txn, &self.squads, squad_id, "slotIds", id);
        append_id(&mut txn, &self.editor_layers, layer_id, "entityIds", id);
    }

    /// Create a faction (mirrors `ydoc.addFaction` and `ensureDefaultSquad`'s faction — JS supplies
    /// `key`/`name`). Writes `{id, key, name, squadIds:[]}`.
    pub fn add_faction(&self, id: &str, key: &str, name: &str) {
        let mut txn = self.begin();
        let f = self
            .factions
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        f.insert(&mut txn, "key", key);
        f.insert(&mut txn, "name", name);
        f.insert(&mut txn, "squadIds", Any::Array(Vec::new().into()));
    }

    /// Overwrite a faction's display `name` (T-180.8 Apply — library name onto `faction-{SIDE}`).
    pub fn set_faction_name(&self, faction_id: &str, name: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(f)) = self.factions.get(&txn, faction_id) {
            f.insert(&mut txn, "name", name);
        }
    }

    /// Create a squad under a faction (mirrors `ydoc.addSquad` and `ensureDefaultSquad`'s squad).
    /// Writes `{id, factionId, name, slotIds:[], vehicleIds:[]}` + `callsign` only when `Some`;
    /// appends `id` to `faction.squadIds` if the faction exists. Does **not** set `leaderSlotId` —
    /// callers use [`Self::set_leader`] after the first slot joins (T-180.1).
    pub fn add_squad(&self, id: &str, faction_id: &str, name: &str, callsign: Option<String>) {
        let mut txn = self.begin();
        let sq = self
            .squads
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        sq.insert(&mut txn, "factionId", faction_id);
        if let Some(c) = callsign {
            sq.insert(&mut txn, "callsign", c);
        }
        sq.insert(&mut txn, "name", name);
        sq.insert(&mut txn, "slotIds", Any::Array(Vec::new().into()));
        sq.insert(&mut txn, "vehicleIds", Any::Array(Vec::new().into()));
        append_id(&mut txn, &self.factions, faction_id, "squadIds", id);
    }

    /// Set (or overwrite) a squad's `leaderSlotId` when `slot_id ∈ squad.slotIds` (T-180.1 / T-180.2
    /// B-L1). No-op if the squad is missing or the slot is not a member.
    pub fn set_leader(&self, squad_id: &str, slot_id: &str) {
        let mut txn = self.begin();
        set_leader_in_txn(&mut txn, &self.squads, squad_id, slot_id);
    }

    /// Rename a squad (T-180.2).
    pub fn rename_squad(&self, squad_id: &str, name: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(sq)) = self.squads.get(&txn, squad_id) {
            sq.insert(&mut txn, "name", name);
        }
    }

    /// Replace `faction.squadIds` with `squad_ids` filtered to squads that exist and belong to
    /// that faction (T-180.2). Unknown / wrong-faction ids are dropped.
    pub fn reorder_squads(&self, faction_id: &str, squad_ids: &[String]) {
        let mut txn = self.begin();
        if self.factions.get(&txn, faction_id).is_none() {
            return;
        }
        let mut next: Vec<Any> = Vec::with_capacity(squad_ids.len());
        for sid in squad_ids {
            if let Some(Out::YMap(sq)) = self.squads.get(&txn, sid.as_str())
                && let Some(Out::Any(Any::String(fid))) = sq.get(&txn, "factionId")
                && fid.as_ref() == faction_id
            {
                next.push(Any::String(sid.as_str().into()));
            }
        }
        if let Some(Out::YMap(f)) = self.factions.get(&txn, faction_id) {
            f.insert(&mut txn, "squadIds", Any::Array(next.into()));
        }
    }

    /// Move a slot from its current squad into `dest_squad_id` (T-180.2). Updates both `slotIds`
    /// arrays, rewrites dense `index` 0..n-1, promotes/GC source leader, and ensures dest leader.
    /// No-op if slot/dest missing or already in dest. **Not** [`Self::move_slot_to_layer`].
    pub fn move_slot_to_squad(&self, slot_id: &str, dest_squad_id: &str) {
        let mut txn = self.begin();
        if self.squads.get(&txn, dest_squad_id).is_none() {
            return;
        }
        let Some(Out::YMap(slot)) = self.slots.get(&txn, slot_id) else {
            return;
        };
        let Some(Out::Any(Any::String(src))) = slot.get(&txn, "squadId") else {
            return;
        };
        let source_squad_id = src.to_string();
        if source_squad_id == dest_squad_id {
            return;
        }

        let source_ids = read_id_array(&txn, &self.squads, &source_squad_id, "slotIds");
        if !source_ids
            .iter()
            .any(|a| matches!(a, Any::String(s) if s.as_ref() == slot_id))
        {
            return;
        }

        let was_leader = matches!(
            self.squads.get(&txn, source_squad_id.as_str()).and_then(|o| match o {
                Out::YMap(sq) => sq.get(&txn, "leaderSlotId"),
                _ => None,
            }),
            Some(Out::Any(Any::String(l))) if l.as_ref() == slot_id
        );

        let kept: Vec<Any> = source_ids
            .iter()
            .filter(|a| !matches!(a, Any::String(s) if s.as_ref() == slot_id))
            .cloned()
            .collect();
        if let Some(Out::YMap(src_sq)) = self.squads.get(&txn, source_squad_id.as_str()) {
            src_sq.insert(&mut txn, "slotIds", Any::Array(kept.clone().into()));
        }

        append_id(&mut txn, &self.squads, dest_squad_id, "slotIds", slot_id);
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, slot_id) {
            slot.insert(&mut txn, "squadId", dest_squad_id);
        }

        rewrite_slot_indices(&mut txn, &self.slots, &self.squads, &source_squad_id);
        rewrite_slot_indices(&mut txn, &self.slots, &self.squads, dest_squad_id);

        if kept.is_empty() {
            garbage_collect_squad_in_txn(
                &mut txn,
                &self.squads,
                &self.factions,
                &self.vehicles,
                &source_squad_id,
            );
        } else if was_leader && let Some(Any::String(next)) = kept.first() {
            set_leader_in_txn(&mut txn, &self.squads, &source_squad_id, next.as_ref());
        }

        ensure_leader_invariant_in_txn(
            &mut txn,
            &self.squads,
            &self.factions,
            &self.vehicles,
            dest_squad_id,
        );
    }

    /// Delete a squad and cascade its slots + attached vehicles (T-180.2).
    pub fn remove_squad(&self, squad_id: &str) {
        let mut txn = self.begin();
        let slot_ids: Vec<String> = read_id_array(&txn, &self.squads, squad_id, "slotIds")
            .iter()
            .filter_map(|a| match a {
                Any::String(s) => Some(s.to_string()),
                _ => None,
            })
            .collect();
        remove_slots_in_txn(
            &mut txn,
            &self.slots,
            &self.squads,
            &self.editor_layers,
            &slot_ids,
        );
        garbage_collect_squad_in_txn(
            &mut txn,
            &self.squads,
            &self.factions,
            &self.vehicles,
            squad_id,
        );
    }

    /// Insert a vehicle row into `vehiclesById` (T-180.2 B-L8). Minimal shape:
    /// `{id, resourceName}` + optional `position`.
    ///
    /// **T-215 — this shape is the contract floor.** `{id, resourceName, position, squadId}` is what
    /// every reader of `vehiclesById` is written against, so nothing here may be renamed, retyped or
    /// dropped; new information goes on as *new* keys. The two this ticket adds are
    /// [`Self::set_vehicle_faction`] (`factionId`) and [`Self::set_vehicle_cargo`] (`cargo`), each
    /// written by its own mutator so a caller that wants only the floor emits only the floor.
    ///
    /// `position` is written **only when both `x` and `y` are given** — a vehicle with no map
    /// position is a legitimate row (that is what the ORBAT-only path produced before map placement
    /// existed), and a `position` of `{0,0,0,0}` would put it at the terrain's south-west corner
    /// rather than nowhere.
    pub fn add_vehicle(
        &self,
        id: &str,
        resource_name: &str,
        x: Option<f64>,
        y: Option<f64>,
        z: Option<f64>,
        rotation: Option<f64>,
    ) {
        let mut txn = self.begin();
        let v = self
            .vehicles
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        v.insert(&mut txn, "resourceName", resource_name);
        if let (Some(x), Some(y)) = (x, y) {
            v.insert(
                &mut txn,
                "position",
                position_any(x, y, z.unwrap_or(0.0), rotation.unwrap_or(0.0)),
            );
        }
    }

    /// T-215 — record which Eden **side** owns a map-placed vehicle, as `factionId`.
    ///
    /// **Why this is not [`Self::attach_vehicle`].** A map placement deliberately does not join the
    /// side's squad. `place_orbat::is_open_for_placement` counts a squad holding *any* vehicle as
    /// authored, so attaching one would close the side's current squad, and the next character
    /// placement would mint a fresh one — reintroducing the one-squad-per-click defect T-321 was
    /// written to remove. `factionId` records the same authored intent (which side this vehicle
    /// belongs to) without touching `squadIds` / `vehicleIds`, so the placement rule is unaffected.
    ///
    /// Purely **additive** to the T-180.2 row: `squadId` keeps its exact meaning and is untouched
    /// here (written by `attach_vehicle`, cleared by `detach_vehicle`). A squad-attached vehicle
    /// therefore carries both, and a map-placed one carries only `factionId` — a reader that
    /// resolves the side through `squadId` alone still gets the right answer for every row that has
    /// one, and gets `None` (not a wrong side) for the rows that do not.
    pub fn set_vehicle_faction(&self, vehicle_id: &str, faction_id: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            v.insert(&mut txn, "factionId", faction_id);
        }
    }

    /// T-215 — what a vehicle is carrying: `vehicle.cargo = [{item, qty}]`.
    ///
    /// The row shape is `mission.schema.json` `$defs/entityInventory` **verbatim**: `item` is an
    /// Enfusion ResourceName, `qty` is a UNIT COUNT (not a stack size), and there is deliberately no
    /// `container` key — for an entity the container *is* the entity, which is exactly why that def
    /// is not a `$defs/cargoContainer` row. That contract already landed on main at T-198
    /// (`2070eecd`, "vehicle and crate inventory gets a home on $defs/entity"), so authoring vehicle
    /// cargo needs **no** schema change; only a compiled-document emitter is still missing.
    ///
    /// Malformed rows are **dropped, not written**: an empty `item` violates `minLength: 1` and a
    /// `qty < 1` violates `minimum: 1`, so writing either would produce a document that cannot
    /// validate — strictly worse than one that lost a row it could never have shipped.
    ///
    /// An empty result **removes** the key rather than writing `[]`. `$defs/entityInventory` defines
    /// absent and `[]` to mean the same thing ("leave the prefab's own contents alone"), and absent
    /// is the smaller document.
    ///
    /// `qty` is written as `Any::BigInt` because that is the variant [`value_to_any`] produces for
    /// an integral JSON number on the way back in; writing `Any::Number` would make the same JSON
    /// round-trip to a different `Any`, which is the class of drift `any_to_f64` exists to absorb
    /// for coordinates and which is avoidable outright here.
    pub fn set_vehicle_cargo(&self, vehicle_id: &str, rows: &[(String, i64)]) {
        let mut txn = self.begin();
        let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) else {
            return;
        };
        let kept: Vec<Any> = rows
            .iter()
            .filter(|(item, qty)| !item.trim().is_empty() && *qty >= 1)
            .map(|(item, qty)| {
                Any::Map(Arc::new(HashMap::from([
                    ("item".to_string(), Any::String(item.as_str().into())),
                    ("qty".to_string(), Any::BigInt(*qty)),
                ])))
            })
            .collect();
        if kept.is_empty() {
            v.remove(&mut txn, "cargo");
        } else {
            v.insert(&mut txn, "cargo", Any::Array(kept.into()));
        }
    }

    /// Attach an existing vehicle to a squad's `vehicleIds` and set `vehicle.squadId` (T-180.2).
    pub fn attach_vehicle(&self, squad_id: &str, vehicle_id: &str) {
        let mut txn = self.begin();
        if self.squads.get(&txn, squad_id).is_none()
            || self.vehicles.get(&txn, vehicle_id).is_none()
        {
            return;
        }
        let existing = read_id_array(&txn, &self.squads, squad_id, "vehicleIds");
        if existing
            .iter()
            .any(|a| matches!(a, Any::String(s) if s.as_ref() == vehicle_id))
        {
            // already attached — still ensure squadId is set
        } else {
            append_id(&mut txn, &self.squads, squad_id, "vehicleIds", vehicle_id);
        }
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            v.insert(&mut txn, "squadId", squad_id);
        }
    }

    /// B2 — delete a vehicle row entirely: detach from its squad (if any) and remove
    /// it from `vehiclesById` ([`Self::detach_vehicle`] alone leaves the row orphaned).
    pub fn remove_vehicle(&self, vehicle_id: &str) {
        let mut txn = self.begin();
        let squad_id = match self.vehicles.get(&txn, vehicle_id) {
            Some(Out::YMap(v)) => match v.get(&txn, "squadId") {
                Some(Out::Any(Any::String(s))) => Some(s.to_string()),
                _ => None,
            },
            _ => return,
        };
        if let Some(sid) = squad_id
            && let Some(Out::YMap(sq)) = self.squads.get(&txn, &sid)
        {
            let arr = read_id_array(&txn, &self.squads, &sid, "vehicleIds");
            let remove: HashSet<&str> = HashSet::from([vehicle_id]);
            let kept = retain_ids(&arr, &remove);
            sq.insert(&mut txn, "vehicleIds", Any::Array(kept.into()));
        }
        self.vehicles.remove(&mut txn, vehicle_id);
    }

    /// Detach a vehicle from a squad's `vehicleIds` and clear `vehicle.squadId` (T-180.2).
    /// The vehicle row remains in `vehiclesById`.
    pub fn detach_vehicle(&self, squad_id: &str, vehicle_id: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(sq)) = self.squads.get(&txn, squad_id) {
            let arr = read_id_array(&txn, &self.squads, squad_id, "vehicleIds");
            let remove: HashSet<&str> = HashSet::from([vehicle_id]);
            let kept = retain_ids(&arr, &remove);
            sq.insert(&mut txn, "vehicleIds", Any::Array(kept.into()));
        }
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            v.remove(&mut txn, "squadId");
        }
    }

    /// T-254 — insert a mission-placed world object into `entitiesById`.
    ///
    /// Editor row shape: `{id, alias, resourceName, position}`. `alias` is the schema
    /// `#/$defs/entity.alias` (`prop:`/`comp:`/…). `resourceName` is the registry-items
    /// ResourceName the Objects palette dropped — kept for editor display/reload; flatten
    /// drops it when emitting the game-server `entities[]` (schema `additionalProperties: false`).
    /// Position is always written (Objects placement is map-only — there is no ORBAT-only path).
    #[allow(clippy::too_many_arguments)]
    pub fn add_entity(
        &self,
        id: &str,
        alias: &str,
        resource_name: &str,
        x: f64,
        y: f64,
        z: f64,
        rotation: f64,
    ) {
        let mut txn = self.begin();
        let e = self
            .entities
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        e.insert(&mut txn, "alias", alias);
        e.insert(&mut txn, "resourceName", resource_name);
        e.insert(&mut txn, "position", position_any(x, y, z, rotation));
    }

    /// T-254 — schema `entity.faction` (factionKey slug, e.g. `blufor`). Stored under the schema
    /// key name so compile can pass the row through after stripping editor-only fields.
    pub fn set_entity_faction(&self, entity_id: &str, faction: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(e)) = self.entities.get(&txn, entity_id) {
            e.insert(&mut txn, "faction", faction);
        }
    }

    /// T-254 — delete a placed world-object row from `entitiesById`.
    pub fn remove_entity(&self, entity_id: &str) {
        let mut txn = self.begin();
        self.entities.remove(&mut txn, entity_id);
    }

    /// Flat `[x0,y0,x1,y1,…]` for every vehicle that has a `position` (T-180.8 map bind).
    /// Order is map-iteration order (not pick-indexed — vehicles stay off the slot SoA).
    #[must_use]
    pub fn vehicle_xy_flat(&self) -> Vec<f32> {
        let txn = self.doc.transact();
        let mut out = Vec::new();
        for (_id, out_v) in self.vehicles.iter(&txn) {
            let Out::YMap(v) = out_v else {
                continue;
            };
            if v.get(&txn, "position").is_none() {
                continue;
            }
            let (x, y, _, _) = read_position(&txn, &v);
            #[allow(clippy::cast_possible_truncation)]
            {
                out.push(x as f32);
                out.push(y as f32);
            }
        }
        out
    }

    /// T-425 — overwrite a placed vehicle's `position` (x/y/z/rotation). No-op when the id is
    /// missing. Creates `position` if the vehicle was previously unplaced (ORBAT-only).
    ///
    /// Does **not** attach a squad — map placement keeps `squadId` absent (T-321 / place_orbat).
    pub fn set_vehicle_position(&self, vehicle_id: &str, x: f64, y: f64, z: f64, rotation: f64) {
        let mut txn = self.begin();
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            let existing = read_position_map(&txn, &v);
            v.insert(
                &mut txn,
                "position",
                position_any_merged(existing, x, y, z, rotation),
            );
        }
    }

    /// T-425 — move placed vehicles by a shared world delta (drag release). Vehicles whose id is
    /// missing or that still have no `position` are skipped. Rotation is preserved.
    pub fn move_vehicles(&self, ids: &[String], dx: f64, dy: f64) {
        let mut txn = self.begin();
        for id in ids {
            let Some(Out::YMap(v)) = self.vehicles.get(&txn, id) else {
                continue;
            };
            if v.get(&txn, "position").is_none() {
                continue;
            }
            let (px, py, pz, prot) = read_position(&txn, &v);
            let existing = read_position_map(&txn, &v);
            v.insert(
                &mut txn,
                "position",
                position_any_merged(existing, px + dx, py + dy, pz, prot),
            );
        }
    }

    /// Overwrite a slot's `position` (mirrors `slot.set('position', {...})`).
    /// T-220 — merges into any existing position map so unknown sub-keys survive the edit.
    pub fn set_slot_position(&self, id: &str, x: f64, y: f64, z: f64, rotation: f64) {
        let mut txn = self.begin();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            let existing = read_position_map(&txn, &slot);
            slot.insert(
                &mut txn,
                "position",
                position_any_merged(existing, x, y, z, rotation),
            );
        }
    }

    /// Remove one slot (mirrors `slots.delete(id)`; layer detach is out of the spike mutator set).
    pub fn remove_slot(&self, id: &str) {
        let mut txn = self.begin();
        self.slots.remove(&mut txn, id);
    }

    /// Bulk-seed `n` random slots in ONE transaction — the browser-harness generator for the
    /// criterion-6 fps/zero-copy test. Deterministic LCG positions in `[0,w)×[0,h)`; not
    /// undo-granular (the whole seed is one step).
    pub fn seed_random(&self, n: u32, w: f64, h: f64, seed: u64) {
        let mut s = seed | 1;
        let mut txn = self.begin();
        for i in 0..n {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let x = (s >> 33) as f64 / f64::from(1u32 << 31) * w;
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let y = (s >> 33) as f64 / f64::from(1u32 << 31) * h;
            let id = format!("s{i}");
            let slot = self.slots.insert(
                &mut txn,
                id.as_str(),
                MapPrelim::from([("id", id.as_str())]),
            );
            slot.insert(&mut txn, "squadId", "sq");
            slot.insert(&mut txn, "role", "Rifleman");
            slot.insert(&mut txn, "stance", "stand");
            slot.insert(&mut txn, "position", position_any(x, y, 0.0, 0.0));
        }
    }

    // ── Batch-1 mutators (full-fidelity ports of `ydoc.ts`; operate on existing ids) ────────────

    /// Patch scalar slot fields; `None` leaves a field unchanged. Mirrors `ydoc.updateSlot`.
    pub fn update_slot(
        &self,
        id: &str,
        role: Option<String>,
        tag: Option<String>,
        stance: Option<String>,
    ) {
        let mut txn = self.begin();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            if let Some(r) = role {
                slot.insert(&mut txn, "role", r);
            }
            if let Some(t) = tag {
                slot.insert(&mut txn, "tag", t);
            }
            if let Some(s) = stance {
                slot.insert(&mut txn, "stance", s);
            }
        }
    }

    /// B2 — mutate an existing slot's role/tag/character in place (ORBAT Apply mutate
    /// semantics: the slot id — and with it every downstream `uid` reference — survives
    /// the re-apply). `tag` / `asset_id`: `Some(non-empty)` sets, `None`/empty clears
    /// (library rows are authoritative on Apply). Position, stance and identity fields
    /// are deliberately untouched — an operator-moved slot stays where it was moved.
    pub fn update_slot_role_character(
        &self,
        id: &str,
        role: &str,
        tag: Option<String>,
        asset_id: Option<String>,
    ) {
        let mut txn = self.begin();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            slot.insert(&mut txn, "role", role);
            match tag.filter(|s| !s.is_empty()) {
                Some(t) => {
                    slot.insert(&mut txn, "tag", t);
                }
                None => {
                    slot.remove(&mut txn, "tag");
                }
            }
            match asset_id.filter(|s| !s.is_empty()) {
                Some(a) => {
                    slot.insert(&mut txn, "assetId", a);
                }
                None => {
                    slot.remove(&mut txn, "assetId");
                }
            }
        }
    }

    /// Set or clear optional slot identity fields `callsign` / `rank` (T-180.1). `Some(non-empty)`
    /// inserts; `None` or empty string removes the key (same omit semantics as `tag` on
    /// [`Self::add_slot`]).
    pub fn update_slot_identity(&self, id: &str, callsign: Option<String>, rank: Option<String>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            match callsign.filter(|s| !s.is_empty()) {
                Some(c) => {
                    slot.insert(&mut txn, "callsign", c);
                }
                None => {
                    slot.remove(&mut txn, "callsign");
                }
            }
            match rank.filter(|s| !s.is_empty()) {
                Some(r) => {
                    slot.insert(&mut txn, "rank", r);
                }
                None => {
                    slot.remove(&mut txn, "rank");
                }
            }
        }
    }

    /// Set or clear a slot's embedded `loadout` (Smart Forge picks — T-068.10). `Some(json)` parses
    /// through the same JSON→`Any` machinery as `hydrate` rows, so the object stays opaque to the
    /// core; `None`/empty clears the key. One transaction = one undo step. The pre-existing
    /// `loadoutId` (shared-template ref, unused) is deliberately untouched.
    pub fn update_slot_loadout(&self, id: &str, loadout_json: Option<String>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            match loadout_json.filter(|s| !s.is_empty()) {
                Some(json) => {
                    slot.insert(&mut txn, "loadout", json_str_to_any(&json));
                }
                None => {
                    slot.remove(&mut txn, "loadout");
                }
            }
        }
    }

    /// Edit a slot's transform (Attributes Transform tab). `x`/`y` clamp to `[0,width]×[0,height]`,
    /// `rotation` normalizes to `[0,360)`, and the z-policy matches `ydoc.updateSlotPosition` (manual
    /// z sticks; an x/y edit terrain-follows → 0 here, DEM sampled JS-side). `None` = leave the axis.
    #[allow(clippy::too_many_arguments)]
    pub fn update_slot_position(
        &self,
        id: &str,
        x: Option<f64>,
        y: Option<f64>,
        z: Option<f64>,
        rotation: Option<f64>,
        width: f64,
        height: f64,
    ) {
        let mut txn = self.begin();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            let (mut px, mut py, mut pz, mut prot) = read_position(&txn, &slot);
            if let Some(nx) = x.filter(|v| v.is_finite()) {
                px = nx.clamp(0.0, width);
            }
            if let Some(ny) = y.filter(|v| v.is_finite()) {
                py = ny.clamp(0.0, height);
            }
            if let Some(nr) = rotation.filter(|v| v.is_finite()) {
                prot = ((nr % 360.0) + 360.0) % 360.0;
            }
            if let Some(nz) = z.filter(|v| v.is_finite()) {
                pz = nz;
            } else if x.is_some() || y.is_some() {
                pz = 0.0; // terrain-follow; DEM z is sampled on the JS side
            }
            let existing = read_position_map(&txn, &slot);
            slot.insert(
                &mut txn,
                "position",
                position_any_merged(existing, px, py, pz, prot),
            );
        }
    }

    /// Move several slots by a shared world delta (drag release). `zs[i]` is the JS-sampled DEM
    /// elevation at slot `ids[i]`'s new position (0 when the DEM is not ready — the vitest case, which
    /// keeps byte-parity). Mirrors `ydoc.moveEntities` (`z = terrainZ(newX, newY)`).
    pub fn move_entities(&self, ids: Vec<String>, dx: f64, dy: f64, zs: Vec<f64>) {
        let mut txn = self.begin();
        for (i, id) in ids.iter().enumerate() {
            if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
                let (px, py, _pz, prot) = read_position(&txn, &slot);
                let z = zs.get(i).copied().unwrap_or(0.0);
                let existing = read_position_map(&txn, &slot);
                slot.insert(
                    &mut txn,
                    "position",
                    position_any_merged(existing, px + dx, py + dy, z, prot),
                );
            }
        }
    }

    /// Remove several slots and detach them from their squad's `slotIds` and every layer's
    /// `entityIds` (batched cascade). Mirrors `ydoc.removeEntities` (slots path). The cascade body
    /// lives in [`remove_slots_in_txn`] so `remove_editor_layer` can reuse it inside its own txn.
    pub fn remove_slots(&self, ids: Vec<String>) {
        let mut txn = self.begin();
        remove_slots_in_txn(
            &mut txn,
            &self.slots,
            &self.squads,
            &self.editor_layers,
            &ids,
        );
    }

    // ── Batch-3b bulk paste (port of `ydoc.pasteSlots`) ─────────────────────────────────────────

    /// Paste `k` copied slots in ONE transaction (mirrors `ydoc.pasteSlots` @180). JS mints the ids
    /// and resolves each slot's target squad/layer (both already existing — `ensureDefault*` runs
    /// JS-side), so the parallel arrays are index-aligned per slot. Positions translate so the clip's
    /// centroid lands at `(anchor_x, anchor_y)`, or nudge `+PASTE_NUDGE` on x/y when no anchor; x/y
    /// clamp to `[0,width]×[0,height]`; `zs[i]` is the JS-sampled DEM elevation at the clamped paste
    /// position (0 when the DEM is not ready — the vitest case, byte-parity-preserving); rotation
    /// carries from the source. `index` accumulates per squad (seeded from the squad's current
    /// `slotIds`). `""` tag/asset → key omitted. Appends are batched (each squad's `slotIds` / each
    /// layer's `entityIds` written once) — the T-059 O(k) shape.
    ///
    /// **T-220 — `extras_json`:** per-slot JSON objects of fields the parallel arrays do not carry
    /// (unknown keys, and unknown `position` sub-keys). Known paste keys in an extra object are
    /// ignored so the parallel arrays stay authoritative for role/tag/position/loadout/….
    #[allow(clippy::too_many_arguments)]
    pub fn paste_slots(
        &self,
        ids: Vec<String>,
        squad_ids: Vec<String>,
        layer_ids: Vec<String>,
        src_x: Vec<f64>,
        src_y: Vec<f64>,
        src_rot: Vec<f64>,
        zs: Vec<f64>,
        roles: Vec<String>,
        tags: Vec<String>,
        asset_ids: Vec<String>,
        stances: Vec<String>,
        loadouts: Vec<String>,
        extras_json: Vec<String>,
        anchor_x: Option<f64>,
        anchor_y: Option<f64>,
        width: f64,
        height: f64,
    ) {
        let n = ids.len();
        if n == 0 {
            return;
        }
        // Centroid in the JS reduce order (left-to-right f64 sum) → byte-identical translate.
        let cx = src_x.iter().sum::<f64>() / n as f64;
        let cy = src_y.iter().sum::<f64>() / n as f64;
        let (dx, dy) = match (anchor_x, anchor_y) {
            (Some(ax), Some(ay)) => (ax - cx, ay - cy),
            _ => (PASTE_NUDGE, PASTE_NUDGE),
        };

        let mut txn = self.begin();
        // Per-squad `slotIds` + per-layer `entityIds` append accumulators, seeded once from the doc.
        let mut squad_slot_ids: HashMap<String, Vec<Any>> = HashMap::new();
        let mut layer_entity_ids: HashMap<String, Vec<Any>> = HashMap::new();
        for i in 0..n {
            let squad_id = &squad_ids[i];
            let layer_id = &layer_ids[i];
            let index = {
                let arr = squad_slot_ids
                    .entry(squad_id.clone())
                    .or_insert_with(|| read_id_array(&txn, &self.squads, squad_id, "slotIds"));
                arr.len() as i64
            };
            let px = (src_x[i] + dx).clamp(0.0, width);
            let py = (src_y[i] + dy).clamp(0.0, height);
            let id = ids[i].as_str();
            let slot = self
                .slots
                .insert(&mut txn, id, MapPrelim::from([("id", id)]));
            slot.insert(&mut txn, "squadId", squad_id.as_str());
            slot.insert(&mut txn, "index", Any::BigInt(index));
            slot.insert(&mut txn, "role", roles[i].as_str());
            if !tags[i].is_empty() {
                slot.insert(&mut txn, "tag", tags[i].as_str());
            }
            if !asset_ids[i].is_empty() {
                slot.insert(&mut txn, "assetId", asset_ids[i].as_str());
            }
            let z = zs.get(i).copied().unwrap_or(0.0);
            // Seed position from the parallel arrays; extras may merge unknown sub-keys below.
            let mut pos = HashMap::new();
            pos.insert("x".to_string(), Any::Number(px));
            pos.insert("y".to_string(), Any::Number(py));
            pos.insert("z".to_string(), Any::Number(z));
            pos.insert("rotation".to_string(), Any::Number(src_rot[i]));
            slot.insert(&mut txn, "stance", stances[i].as_str());
            slot.insert(&mut txn, "loadoutId", Any::Null);
            // `""` = source slot had no loadout (same omit convention as tag/assetId above).
            if let Some(lj) = loadouts.get(i).filter(|s| !s.is_empty()) {
                slot.insert(&mut txn, "loadout", json_str_to_any(lj));
            }
            // T-220 — merge unknown fields (and unknown position sub-keys) from the clipboard row.
            if let Some(extra) = extras_json.get(i).filter(|s| !s.is_empty())
                && let Any::Map(fields) = json_str_to_any(extra)
            {
                for (k, v) in fields.iter() {
                    if PASTE_KNOWN_SLOT_KEYS.contains(&k.as_str()) {
                        if k == "position"
                            && let Any::Map(sub) = v
                        {
                            for (pk, pv) in sub.iter() {
                                if !matches!(pk.as_str(), "x" | "y" | "z" | "rotation") {
                                    pos.insert(pk.clone(), pv.clone());
                                }
                            }
                        }
                        continue;
                    }
                    slot.insert(&mut txn, k.as_str(), v.clone());
                }
            }
            slot.insert(&mut txn, "position", Any::Map(Arc::new(pos)));
            if let Some(arr) = squad_slot_ids.get_mut(squad_id) {
                arr.push(Any::String(id.into()));
            }
            layer_entity_ids
                .entry(layer_id.clone())
                .or_insert_with(|| read_id_array(&txn, &self.editor_layers, layer_id, "entityIds"))
                .push(Any::String(id.into()));
        }

        for (sid, arr) in squad_slot_ids {
            if let Some(Out::YMap(squad)) = self.squads.get(&txn, &sid) {
                squad.insert(&mut txn, "slotIds", Any::Array(arr.into()));
            }
        }
        for (lid, arr) in layer_entity_ids {
            if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, &lid) {
                layer.insert(&mut txn, "entityIds", Any::Array(arr.into()));
            }
        }
    }

    // ── Batch-3c layer removal + meta (ports of `ydoc.ts`) ──────────────────────────────────────

    /// Delete an Outliner folder AND its whole subtree — every nested folder plus all filed slots —
    /// in one transaction (mirrors `ydoc.removeEditorLayer` @500). No-op if the folder is absent or
    /// it is the only layer (keep ≥1). If the subtree was every layer, a fresh default layer is
    /// reseeded (JS mints `reseed_id`) so the editor is never layer-less.
    pub fn remove_editor_layer(&self, id: &str, reseed_id: &str) {
        let mut txn = self.begin();
        if self.editor_layers.get(&txn, id).is_none() || self.editor_layers.len(&txn) <= 1 {
            return;
        }
        // Collect the subtree: `id` plus every layer whose parent chain reaches it (fixpoint).
        let mut subtree: HashSet<String> = HashSet::new();
        subtree.insert(id.to_string());
        loop {
            let parents: Vec<(String, Option<String>)> = self
                .editor_layers
                .iter(&txn)
                .map(|(lid, out)| {
                    let pid = match out {
                        Out::YMap(l) => match l.get(&txn, "parentId") {
                            Some(Out::Any(Any::String(p))) => Some(p.to_string()),
                            _ => None,
                        },
                        _ => None,
                    };
                    (lid.to_string(), pid)
                })
                .collect();
            let mut added = false;
            for (lid, pid) in parents {
                if let Some(p) = pid
                    && subtree.contains(&p)
                    && !subtree.contains(&lid)
                {
                    subtree.insert(lid);
                    added = true;
                }
            }
            if !added {
                break;
            }
        }
        // Gather every slot filed in a subtree layer, cascade-remove them, then delete the layers.
        let mut slot_ids: Vec<String> = Vec::new();
        for lid in &subtree {
            if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, lid)
                && let Some(Out::Any(Any::Array(arr))) = layer.get(&txn, "entityIds")
            {
                for a in arr.iter() {
                    if let Any::String(s) = a {
                        slot_ids.push(s.to_string());
                    }
                }
            }
        }
        remove_slots_in_txn(
            &mut txn,
            &self.slots,
            &self.squads,
            &self.editor_layers,
            &slot_ids,
        );
        for lid in &subtree {
            self.editor_layers.remove(&mut txn, lid);
        }
        if self.editor_layers.len(&txn) == 0 {
            let layer = self.editor_layers.insert(
                &mut txn,
                reseed_id,
                MapPrelim::from([("id", reseed_id)]),
            );
            layer.insert(&mut txn, "name", "Default Layer");
            layer.insert(&mut txn, "parentId", Any::Null);
            layer.insert(&mut txn, "entityIds", Any::Array(Vec::new().into()));
        }
    }

    /// Set the mission title (mirrors `ydoc.setTitle`).
    pub fn set_title(&self, title: &str) {
        let mut txn = self.begin();
        self.meta.insert(&mut txn, "title", title);
    }

    /// Merge an environment patch (a JSON object) onto the existing `meta.environment`, mirroring
    /// `ydoc.updateEnvironment` (`{...env, ...patch}`). Absent env → the patch becomes the env.
    pub fn update_environment(&self, patch_json: &str) {
        let mut txn = self.begin();
        let mut env = read_env_map(&txn, &self.meta);
        if let Any::Map(patch) = json_str_to_any(patch_json) {
            for (k, v) in patch.iter() {
                env.insert(k.clone(), v.clone());
            }
        }
        self.meta
            .insert(&mut txn, "environment", Any::Map(Arc::new(env)));
    }

    /// Apply mission-row fields from `GET /missions/:id` (mirrors `ydoc.applyMissionRowMeta`): title
    /// if non-blank after trim (T-505 — whitespace-only is not a title); terrain only if valid;
    /// `time`/`weather` merged onto the existing environment.
    ///
    /// Callers that also hydrate a compiled payload (T-375 emits top-level `title`) must prefer the
    /// payload title over a stale row — see `mission_hydrate::adopt_payload`. This mutator writes
    /// whatever non-blank title it is given.
    ///
    /// `briefing` is the mission **row** library blurb (`missions.briefing` STRING) — not the
    /// per-faction `factionsById[].briefing` object. T-418 threads it so `compile_export` can emit
    /// a real envelope `briefing` instead of permanently `""`. Whitespace-only is not authored
    /// content (same trim rule as title / featured briefing).
    pub fn apply_row_meta(
        &self,
        title: &str,
        terrain: &str,
        time_of_day: Option<String>,
        weather: Option<String>,
        briefing: Option<String>,
    ) {
        let mut txn = self.begin();
        let title = title.trim();
        if !title.is_empty() {
            self.meta.insert(&mut txn, "title", title);
        }
        if matches!(terrain, "everon" | "arland" | "custom") {
            self.meta.insert(&mut txn, "terrain", terrain);
        }
        if time_of_day.is_some() || weather.is_some() {
            let mut env = read_env_map(&txn, &self.meta);
            if let Some(t) = time_of_day {
                env.insert("time".to_string(), Any::String(t.as_str().into()));
            }
            if let Some(w) = weather {
                env.insert("weather".to_string(), Any::String(w.as_str().into()));
            }
            self.meta
                .insert(&mut txn, "environment", Any::Map(Arc::new(env)));
        }
        if let Some(b) = briefing {
            let b = b.trim();
            if !b.is_empty() {
                self.meta.insert(&mut txn, "briefing", b);
            }
        }
    }

    /// Seed default meta if empty (mirrors `ydoc.seedMeta` + `DEFAULT_META`). No-op if meta exists.
    pub fn seed_meta(&self, id: &str, title: &str) {
        let mut txn = self.begin();
        if self.meta.len(&txn) > 0 {
            return;
        }
        self.meta.insert(&mut txn, "id", id);
        self.meta.insert(&mut txn, "title", title);
        self.meta.insert(&mut txn, "terrain", "everon");
        let mut env: HashMap<String, Any> = HashMap::new();
        env.insert("time".to_string(), Any::String("06:00".into()));
        env.insert("weather".to_string(), Any::String("clear".into()));
        self.meta
            .insert(&mut txn, "environment", Any::Map(Arc::new(env)));
    }

    // ── Batch-3d hydrate (lossless loader; port of `ydoc.hydrateMissionDoc`) ─────────────────────

    /// Repopulate the doc from a compiled `json_payload` — the **lossless** dict-load half of
    /// `ydoc.hydrateMissionDoc` @535: clear every entity map (meta kept), set `environment` +
    /// `map.terrain`, then load `objectives`/`vehicles`/`markers`, `loadouts` (object → values), and
    /// the `editor.{factions,squads,slots,editorLayers}` graph **verbatim** (each row → a nested map;
    /// nested objects like `position` stay opaque, exactly like `entityToYMap`). The **lossy**
    /// `orbat[]` rebuild stays JS-side (it mints ids); the flip wrapper transforms lossy → an
    /// `editor`-shaped payload and calls this. If no layers were loaded, a default layer is reseeded
    /// with the JS-minted `default_layer_id` (mirrors `ensureDefaultLayer`).
    ///
    /// **T-219 — unknown top-level keys.** Keys that neither this loader nor `compile_payload`
    /// author (`schemaVersion` / `map` / `environment` / `loadouts` / `objectives` / `vehicles` /
    /// `markers` / `editor` / `orbat`) are parked in the `payloadExtras` root map and re-emitted on
    /// the next Save. Without that, a server-first or migration field appears to persist, then
    /// vanishes on the next hydrate→compile cycle.
    ///
    /// **T-432 — reserved side-channel name.** The key `payloadExtras` itself is treated as known
    /// (reserved): an authored top-level `payloadExtras` object is **not** nested into the
    /// side-channel and is **not** re-emitted onto the wire. Nested contents under that collision
    /// are dropped (reserved-key policy), not renamed. Unrelated unknown keys still park.
    ///
    /// **T-220 — known top-level fields that used to be drop-on-sight:**
    /// - `schemaVersion` is stored on `meta` (compile re-emits it; schema allows any integer).
    /// - the whole `map` object is stored on `meta.map` so non-`terrain` keys (and authored
    ///   `bounds`) survive; `meta.terrain` still tracks the live terrain id for the editor.
    ///
    /// **T-505 — top-level `title` (T-375 wire emit).** Non-blank trimmed string → `meta.title`.
    /// Absent / blank / whitespace-only leaves `meta.title` cleared for this hydrate (no sticky
    /// ghost from a prior doc); `apply_row_meta` / `adopt_payload` can still supply the row.
    pub fn hydrate(&self, payload_json: &str, default_layer_id: &str) {
        let Any::Map(payload) = json_str_to_any(payload_json) else {
            return;
        };
        // Grab the non-tracked map handles before opening the txn (`get_or_insert_map` takes &self).
        let loadouts = self.doc.get_or_insert_map("loadouts");
        let items = self.doc.get_or_insert_map("items");
        let objectives = self.doc.get_or_insert_map("objectives");
        let vehicles = self.doc.get_or_insert_map("vehicles");
        let entities = self.doc.get_or_insert_map("entities");
        let markers = self.doc.get_or_insert_map("markers");
        let payload_extras = self.doc.get_or_insert_map("payloadExtras");
        let entity_order = self.doc.get_or_insert_map("entityOrder");

        let mut txn = self.begin();
        for m in [
            &self.slots,
            &self.squads,
            &self.factions,
            &self.editor_layers,
            &loadouts,
            &items,
            &objectives,
            &vehicles,
            &entities,
            &markers,
            &payload_extras,
            &entity_order,
        ] {
            m.clear(&mut txn);
        }
        // Drop prior authored map / schemaVersion / title so a second hydrate cannot leave sticky ghosts.
        self.meta.remove(&mut txn, "map");
        self.meta.remove(&mut txn, "schemaVersion");
        self.meta.remove(&mut txn, "title");

        if let Some(env) = payload.get("environment") {
            self.meta.insert(&mut txn, "environment", env.clone());
        }
        if let Some(sv) = payload.get("schemaVersion") {
            self.meta.insert(&mut txn, "schemaVersion", sv.clone());
        }
        // T-505 — load T-375's top-level authored title into meta (trim-aware, non-blank).
        if let Some(Any::String(title)) = payload.get("title") {
            let trimmed = title.trim();
            if !trimmed.is_empty() {
                self.meta.insert(&mut txn, "title", trimmed);
            }
        }
        if let Some(map_val) = payload.get("map") {
            // Whole object — compile merges terrain + preserves other keys / authored bounds.
            self.meta.insert(&mut txn, "map", map_val.clone());
            if let Any::Map(map) = map_val
                && let Some(Any::String(terrain)) = map.get("terrain")
            {
                self.meta.insert(&mut txn, "terrain", terrain.as_ref());
            }
        }

        load_rows_ordered(
            &mut txn,
            &objectives,
            payload.get("objectives"),
            &entity_order,
            "objectives",
        );
        load_rows_ordered(
            &mut txn,
            &vehicles,
            payload.get("vehicles"),
            &entity_order,
            "vehicles",
        );
        load_rows_ordered(
            &mut txn,
            &entities,
            payload.get("entities"),
            &entity_order,
            "entities",
        );
        load_rows_ordered(
            &mut txn,
            &markers,
            payload.get("markers"),
            &entity_order,
            "markers",
        );
        if let Some(Any::Map(lo)) = payload.get("loadouts") {
            for v in lo.values() {
                load_row(&mut txn, &loadouts, v);
            }
        }

        if let Some(Any::Map(editor)) = payload.get("editor") {
            load_rows_ordered(
                &mut txn,
                &self.factions,
                editor.get("factions"),
                &entity_order,
                "factions",
            );
            load_rows_ordered(
                &mut txn,
                &self.squads,
                editor.get("squads"),
                &entity_order,
                "squads",
            );
            load_rows_ordered(
                &mut txn,
                &self.slots,
                editor.get("slots"),
                &entity_order,
                "slots",
            );
            load_rows_ordered(
                &mut txn,
                &self.editor_layers,
                editor.get("editorLayers"),
                &entity_order,
                "editorLayers",
            );
        }

        // T-219 — park every top-level key this loader does not understand. Nested values stay
        // opaque `Any` (same as `load_row`), so objects/arrays round-trip through yrs untouched.
        for (k, v) in payload.iter() {
            if is_known_editor_payload_top_level(k) {
                continue;
            }
            payload_extras.insert(&mut txn, k.as_str(), v.clone());
        }

        if self.editor_layers.len(&txn) == 0 {
            let layer = self.editor_layers.insert(
                &mut txn,
                default_layer_id,
                MapPrelim::from([("id", default_layer_id)]),
            );
            layer.insert(&mut txn, "name", "Default Layer");
            layer.insert(&mut txn, "parentId", Any::Null);
            layer.insert(&mut txn, "entityIds", Any::Array(Vec::new().into()));
        }
    }

    // ── Batch-2 editor-layer mutators (ports of `ydoc.ts`) ──────────────────────────────────────

    /// Create an Outliner folder (id + name computed JS-side). Mirrors `ydoc.addEditorLayer`.
    pub fn add_editor_layer(&self, id: &str, name: &str, parent_id: Option<String>) {
        let mut txn = self.begin();
        let layer = self
            .editor_layers
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        layer.insert(&mut txn, "name", name);
        match parent_id {
            Some(p) => layer.insert(&mut txn, "parentId", p),
            None => layer.insert(&mut txn, "parentId", Any::Null),
        };
        layer.insert(&mut txn, "entityIds", Any::Array(Vec::new().into()));
    }

    /// Rename an Outliner folder. Mirrors `ydoc.renameEditorLayer`.
    pub fn rename_editor_layer(&self, id: &str, name: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, id) {
            layer.insert(&mut txn, "name", name);
        }
    }

    /// Reparent an Outliner folder; rejects cycles (dropping it into its own subtree). Mirrors
    /// `ydoc.reparentEditorLayer`.
    pub fn reparent_editor_layer(&self, id: &str, new_parent_id: Option<String>) {
        let mut txn = self.begin();
        if self.editor_layers.get(&txn, id).is_none() {
            return;
        }
        if let Some(p) = new_parent_id.as_deref()
            && (p == id || self.is_layer_descendant(&txn, id, p))
        {
            return;
        }
        if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, id) {
            match new_parent_id {
                Some(p) => layer.insert(&mut txn, "parentId", p),
                None => layer.insert(&mut txn, "parentId", Any::Null),
            };
        }
    }

    /// Refile a slot into a different Outliner folder (workflow-only; squad unchanged): detach from
    /// every folder holding it, then append to the target. Mirrors `ydoc.moveSlotToLayer`.
    pub fn move_slot_to_layer(&self, slot_id: &str, target_layer_id: &str) {
        let mut txn = self.begin();
        if self.editor_layers.get(&txn, target_layer_id).is_none() {
            return;
        }
        let layer_ids: Vec<String> = self
            .editor_layers
            .iter(&txn)
            .map(|(k, _)| k.to_string())
            .collect();
        for lid in &layer_ids {
            if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, lid)
                && let Some(Out::Any(Any::Array(arr))) = layer.get(&txn, "entityIds")
                && arr
                    .iter()
                    .any(|a| matches!(a, Any::String(s) if s.as_ref() == slot_id))
            {
                let kept: Vec<Any> = arr
                    .iter()
                    .filter(|a| !matches!(a, Any::String(s) if s.as_ref() == slot_id))
                    .cloned()
                    .collect();
                layer.insert(&mut txn, "entityIds", Any::Array(kept.into()));
            }
        }
        if let Some(Out::YMap(target)) = self.editor_layers.get(&txn, target_layer_id)
            && let Some(Out::Any(Any::Array(arr))) = target.get(&txn, "entityIds")
        {
            let mut next: Vec<Any> = arr.iter().cloned().collect();
            next.push(Any::String(slot_id.into()));
            target.insert(&mut txn, "entityIds", Any::Array(next.into()));
        }
    }

    /// Is `node_id` inside `ancestor_id`'s subtree (or equal)? Walks up via `parentId`. Mirrors
    /// `ydoc.isLayerDescendant`.
    fn is_layer_descendant<T: ReadTxn>(&self, txn: &T, ancestor_id: &str, node_id: &str) -> bool {
        let mut cur = Some(node_id.to_string());
        while let Some(c) = cur {
            if c == ancestor_id {
                return true;
            }
            cur = match self.editor_layers.get(txn, &c) {
                Some(Out::YMap(layer)) => match layer.get(txn, "parentId") {
                    Some(Out::Any(Any::String(p))) => Some(p.to_string()),
                    _ => None,
                },
                _ => None,
            };
        }
        false
    }

    // ── T-345 per-faction briefing markers (the authoring half of T-202's `briefings` emitter) ───

    /// Place or move one map marker on a faction's briefing — the mutator that was missing between
    /// T-202's shipped emitter and a working marker.
    ///
    /// Writes `factionsById[faction_id].briefing.markers[]`. That is the ONLY placement that reaches
    /// a game server: `mission.schema.json` `$defs/briefing` declares `markers`, and the compiled
    /// document has **no top-level `markers` property at all**, so a marker anywhere else is a marker
    /// no mod subsystem can read. Markers are per-faction because they are side-scoped intelligence —
    /// `bridgehead-at-levie` gives both sides different orders at the same coordinates.
    ///
    /// ## The doc row carries an `id`; the wire row does not
    ///
    /// Doc: `{id, x, z, icon, label}`. Wire (`$defs/marker`): `{x, z, icon, label}` with
    /// `additionalProperties: false`. **The doc shape and the wire shape deliberately differ by
    /// exactly this one key** — the first place in this document that they do, which is why it is
    /// recorded here, on the writer, rather than left to be inferred by whoever reads next.
    ///
    /// Nothing has to strip the `id`: the serde boundary already does, for free. `flatten.rs`
    /// `MarkerIn` is a `#[serde(default)]` struct of the four schema fields with no
    /// `deny_unknown_fields`, so it ignores the extra key, and `derive_briefings` re-emits through
    /// `ModMarker`, which has only those four. The compiled document therefore satisfies
    /// `additionalProperties: false` with **no change to T-202's emitter**, and the id exists on the
    /// authoring side only.
    ///
    /// What the id buys is ADDRESSING, and — stated plainly, because the overclaim is tempting — not
    /// more than that. The editor must move, re-caption and delete a marker it placed, and an array
    /// index is not a stable handle: removing a sibling renumbers every marker after it, so a queued
    /// drag would land on the wrong row. Every other entity here is addressed by id, and a marker
    /// being the one exception would earn the frontend a special case for nothing.
    ///
    /// It does **not** buy CRDT merge granularity. `briefing` rides the faction row as an OPAQUE
    /// `Any::Map` — that is precisely why T-214's prose round-trips with no `store.rs` change, since
    /// [`load_row`] re-inserts a nested object verbatim without descending into it — so the
    /// finest-grained key yrs sees is `briefing` itself. Two concurrent marker edits on ONE faction
    /// are last-write-wins over the whole briefing no matter what ids sit inside it. Splitting markers
    /// into their own tracked root map to win that granularity would move them off the only
    /// schema-legal placement, so the trade is settled this way on purpose.
    ///
    /// ## §authority — the root `markers` map is NOT this surface
    ///
    /// `MissionDocCore` also has a `markers` ROOT map ([`Self::hydrate`] clears and reloads it,
    /// [`Self::small_maps_json`] emits it as `markersById`). It is authoritative for nothing that
    /// reaches a game server, and that is checkable rather than a matter of taste:
    /// `compile_payload` puts it at the EDITOR payload's `markers` root, and
    /// `flatten_to_mod_document` deserialises `EditorPayload { editor: EditorGraph { factions,
    /// squads, slots } }` — which declares no root key whatsoever, so that lane is never compiled.
    /// It is a closed hydrate→emit loop. **Author here, not there.**
    ///
    /// The root map is left standing rather than deleted: [`Self::has_content`] counts it for the
    /// warm-session conflict gate, and removing a root map is a migration, not a marker ticket.
    ///
    /// ## Semantics
    ///
    /// Upsert by `marker_id`, replacing IN PLACE so a drag cannot reorder the list — the mod renders
    /// in array order (`derive_briefings` pushes in order into the parallel arrays
    /// `TBD_MarkerService.Build` sends). No-op on an unknown `faction_id`: a marker cannot exist
    /// without a side to be told about it.
    ///
    /// `label` is stored VERBATIM. The mod caps it (`TBD_MarkerService.CapLabel`, silently) and the
    /// emitter applies that cap when it compiles, so capping here as well would destroy the authored
    /// value in the one place the author could still see and fix it.
    pub fn set_faction_briefing_marker(
        &self,
        faction_id: &str,
        marker_id: &str,
        x: f64,
        z: f64,
        icon: &str,
        label: &str,
    ) {
        let mut txn = self.begin();
        let Some(Out::YMap(f)) = self.factions.get(&txn, faction_id) else {
            return;
        };
        let mut briefing = read_any_map(&txn, &f, "briefing");
        let mut markers = briefing_markers(&briefing);
        let row = marker_any(marker_id, x, z, icon, label);
        match markers
            .iter()
            .position(|m| marker_row_id(m) == Some(marker_id))
        {
            Some(i) => markers[i] = row,
            None => markers.push(row),
        }
        briefing.insert("markers".to_string(), Any::Array(markers.into()));
        f.insert(&mut txn, "briefing", Any::Map(Arc::new(briefing)));
    }

    /// Delete one marker from a faction's briefing by its doc-internal id, leaving the prose fields
    /// and the sibling markers untouched.
    ///
    /// Removing the last marker writes an empty `markers` array rather than dropping the key; the
    /// emitter cannot tell the difference (`ModBriefing::markers` is
    /// `skip_serializing_if = "Vec::is_empty"`, so an emptied list omits the key in the compiled
    /// document exactly as an absent one does).
    ///
    /// No-op when nothing matches — including on a faction with no briefing at all. That guard is
    /// load-bearing, not defensive tidiness: writing `briefing: {markers: []}` onto an unauthored
    /// faction would flip `FactionIn::briefing` from `None` to `Some`, and `derive_briefings` would
    /// start emitting a `briefings` entry for a side that authored nothing — a compiled-output change
    /// produced by a delete that deleted nothing.
    pub fn remove_faction_briefing_marker(&self, faction_id: &str, marker_id: &str) {
        let mut txn = self.begin();
        let Some(Out::YMap(f)) = self.factions.get(&txn, faction_id) else {
            return;
        };
        let mut briefing = read_any_map(&txn, &f, "briefing");
        let mut markers = briefing_markers(&briefing);
        let before = markers.len();
        markers.retain(|m| marker_row_id(m) != Some(marker_id));
        if markers.len() == before {
            return;
        }
        briefing.insert("markers".to_string(), Any::Array(markers.into()));
        f.insert(&mut txn, "briefing", Any::Map(Arc::new(briefing)));
    }

    /// Write one faction's briefing PROSE — the last mutator missing between T-202's emitter and a
    /// briefing an author can actually type (T-344).
    ///
    /// Writes `factionsById[faction_id].briefing.{situation,mission,execution}`, the three prose keys
    /// `mission.schema.json` `$defs/briefing` declares beside `markers`. `additionalProperties: false`
    /// there makes those four the only legal shape, so this is the whole prose surface. No-op on an
    /// unknown `faction_id`, exactly as the marker mutators: orders need a side to be given to.
    ///
    /// T-214 proved this prose round-trips the document with **no `store.rs` change at all**, using
    /// `hydrate` as the writer — [`load_row`] re-inserts a nested object verbatim without descending,
    /// so the storage side was never the gap. What was missing was a writer the editor can call, and
    /// that is all this is.
    ///
    /// ## Prose is NOT sanitised, and that is a schema-level decision
    ///
    /// An embedded newline goes in VERBATIM. `$defs/wireSafeString` bans the C0 block for values that
    /// ride a tab/newline-delimited wire, and its final paragraph EXCLUDES briefing prose by name:
    /// `TBD_BriefingService` ships prose as parallel `array<string>` RPC parameters and the mod SPLITS
    /// on newlines to get display paragraphs (`TBD_BriefingData.AppendParagraphs` → `SplitLines`). A
    /// multi-paragraph situation report is therefore the FEATURE. Folding newlines to spaces here
    /// would silently collapse an author's paragraphs into one wall of text —
    /// `authored_prose_round_trips_through_compile_and_hydrate` pins the paragraph break end to end
    /// so a future "tidy the input" reflex fails loudly instead.
    ///
    /// ## Empty means UNAUTHORED, so an empty field is removed rather than blanked
    ///
    /// `ModBriefing`'s three prose fields are `Option<String>` precisely so "the author wrote nothing"
    /// and "the author wrote an empty string" stay distinguishable in the compiled bytes. A
    /// three-textbox editor cannot express that difference — an empty box is the DEFAULT state, not a
    /// deliberate blanking — so this mutator resolves it the only way that matches what the caller can
    /// say: `""` REMOVES the key. Inserting `Some("")` instead would stamp "somebody deliberately
    /// blanked this" onto every box the author simply never filled. Clearing a box therefore returns
    /// that field to unauthored, which is what clearing it means. `Some("")` stays reachable through
    /// `hydrate` for a document that genuinely carries one; this just never mints one.
    ///
    /// ## The read-modify-write is WHOLE, and the no-op guard is T-345's in reverse
    ///
    /// `briefing` rides the faction row as an opaque `Any::Map`, so there is no sub-key to insert into
    /// — [`read_any_map`] reads it out entire and the whole value goes back. A naive
    /// `insert("briefing", <fresh prose>)` would DELETE the markers T-345 just made authorable.
    /// `markers` is never touched here; `prose_and_markers_do_not_eat_each_other` pins both
    /// directions.
    ///
    /// Returning early when the map is UNCHANGED is the same load-bearing guard as
    /// [`Self::remove_faction_briefing_marker`]'s, arrived at from the other side. Setting all-empty
    /// prose on a faction with no briefing would otherwise write `briefing: {}`, flipping
    /// `FactionIn::briefing` from `None` to `Some` and making `derive_briefings` emit a `briefings`
    /// entry for a side that authored nothing — a compiled-output change produced by a write that
    /// wrote nothing, plus an undo step with nothing in it. Comparing the map subsumes that case and
    /// also swallows a redundant re-set of identical prose. It deliberately does NOT no-op merely
    /// because the new prose is empty: clearing every box on a briefing that HAS markers is a real
    /// edit, and correctly leaves a legal `{markers: [...]}` behind.
    pub fn set_faction_briefing(
        &self,
        faction_id: &str,
        situation: &str,
        mission: &str,
        execution: &str,
    ) {
        let mut txn = self.begin();
        let Some(Out::YMap(f)) = self.factions.get(&txn, faction_id) else {
            return;
        };
        let before = read_any_map(&txn, &f, "briefing");
        let mut briefing = before.clone();
        for (key, text) in [
            ("situation", situation),
            ("mission", mission),
            ("execution", execution),
        ] {
            if text.is_empty() {
                briefing.remove(key);
            } else {
                briefing.insert(key.to_string(), Any::String(text.into()));
            }
        }
        if briefing == before {
            return;
        }
        f.insert(&mut txn, "briefing", Any::Map(Arc::new(briefing)));
    }

    /// How many undo steps are stacked. The capture side of the T-159.22.1 invariant (one LOCAL txn
    /// = one step) — `can_undo` only says "≥ 1", which is what let the granularity defect hide.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo_mgr.undo_stack().len()
    }

    /// Undo the most recent tracked transaction; `true` if anything was undone.
    pub fn undo(&mut self) -> bool {
        self.undo_mgr.undo_blocking()
    }

    /// Redo the most recently undone transaction; `true` if anything was redone.
    pub fn redo(&mut self) -> bool {
        self.undo_mgr.redo_blocking()
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.undo_mgr.can_undo()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.undo_mgr.can_redo()
    }

    /// Number of slots currently in the document.
    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slots.len(&self.doc.transact()) as usize
    }

    /// True if the doc holds authored content beyond seeded defaults — any faction / slot / objective
    /// / vehicle / entity / marker. Backs `useMissionEditor.hasLocalContent` (the warm-session / conflict gate).
    #[must_use]
    pub fn has_content(&self) -> bool {
        let objectives = self.doc.get_or_insert_map("objectives");
        let vehicles = self.doc.get_or_insert_map("vehicles");
        let entities = self.doc.get_or_insert_map("entities");
        let markers = self.doc.get_or_insert_map("markers");
        let txn = self.doc.transact();
        self.factions.len(&txn) > 0
            || self.slots.len(&txn) > 0
            || objectives.len(&txn) > 0
            || vehicles.len(&txn) > 0
            || entities.len(&txn) > 0
            || markers.len(&txn) > 0
    }
}

impl Default for MissionDocCore {
    fn default() -> Self {
        Self::new()
    }
}

/// A `{x,y,z,rotation}` plain object as a `yrs` `Any::Map` (how Yjs stores `Slot.position`).
fn position_any(x: f64, y: f64, z: f64, rotation: f64) -> Any {
    position_any_merged(HashMap::new(), x, y, z, rotation)
}

/// T-220 — write position coords while keeping any unknown sub-keys already on the map
/// (`heading`, `source`, …). Replacing the whole map with only the four known keys was the
/// "position sub-keys die on first edit" loss.
fn position_any_merged(
    mut existing: HashMap<String, Any>,
    x: f64,
    y: f64,
    z: f64,
    rotation: f64,
) -> Any {
    existing.insert("x".to_string(), Any::Number(x));
    existing.insert("y".to_string(), Any::Number(y));
    existing.insert("z".to_string(), Any::Number(z));
    existing.insert("rotation".to_string(), Any::Number(rotation));
    Any::Map(Arc::new(existing))
}

/// Slot keys the paste parallel arrays already author — extras must not override these
/// (except unknown `position` sub-keys, merged separately).
const PASTE_KNOWN_SLOT_KEYS: &[&str] = &[
    "id",
    "squadId",
    "index",
    "role",
    "tag",
    "assetId",
    "position",
    "stance",
    "loadoutId",
    "loadout",
];

/// Keep every element of `arr` except `Any::String`s present in `remove` (removed slot ids). Used by
/// the `remove_slots` cross-ref cascade to filter a `slotIds`/`entityIds` array.
fn retain_ids(arr: &[Any], remove: &HashSet<&str>) -> Vec<Any> {
    arr.iter()
        .filter(|a| !matches!(a, Any::String(s) if remove.contains(s.as_ref())))
        .cloned()
        .collect()
}

/// Append `id` to `map[key].field` (an `Any::Array` of string ids), if that container map exists.
/// Mirrors ydoc's `container.set(field, [...(container.get(field)), id])` cross-ref append.
fn append_id(txn: &mut TransactionMut, map: &MapRef, key: &str, field: &str, id: &str) {
    if let Some(Out::YMap(container)) = map.get(txn, key) {
        let mut next: Vec<Any> = match container.get(txn, field) {
            Some(Out::Any(Any::Array(arr))) => arr.iter().cloned().collect(),
            _ => Vec::new(),
        };
        next.push(Any::String(id.into()));
        container.insert(txn, field, Any::Array(next.into()));
    }
}

/// Write `leaderSlotId` only when `slot_id` is in the squad's `slotIds` (T-180.2 B-L1).
fn set_leader_in_txn(txn: &mut TransactionMut, squads: &MapRef, squad_id: &str, slot_id: &str) {
    let ids = read_id_array(txn, squads, squad_id, "slotIds");
    if !ids
        .iter()
        .any(|a| matches!(a, Any::String(s) if s.as_ref() == slot_id))
    {
        return;
    }
    if let Some(Out::YMap(sq)) = squads.get(txn, squad_id) {
        sq.insert(txn, "leaderSlotId", slot_id);
    }
}

/// Rewrite each member slot's `index` to dense `0..n-1` matching `slotIds` order (T-180.2 B-L3).
fn rewrite_slot_indices(txn: &mut TransactionMut, slots: &MapRef, squads: &MapRef, squad_id: &str) {
    let ids = read_id_array(txn, squads, squad_id, "slotIds");
    for (i, any) in ids.iter().enumerate() {
        let Any::String(sid) = any else {
            continue;
        };
        if let Some(Out::YMap(slot)) = slots.get(txn, sid.as_ref()) {
            slot.insert(txn, "index", Any::BigInt(i as i64));
        }
    }
}

/// Delete vehicles listed on the squad, detach the squad from its faction, and remove the squad row.
fn garbage_collect_squad_in_txn(
    txn: &mut TransactionMut,
    squads: &MapRef,
    factions: &MapRef,
    vehicles: &MapRef,
    squad_id: &str,
) {
    if squads.get(txn, squad_id).is_none() {
        return;
    }
    let faction_id = match squads.get(txn, squad_id).and_then(|o| match o {
        Out::YMap(sq) => sq.get(txn, "factionId"),
        _ => None,
    }) {
        Some(Out::Any(Any::String(f))) => f.to_string(),
        _ => String::new(),
    };
    let vehicle_ids = read_id_array(txn, squads, squad_id, "vehicleIds");
    for vid in &vehicle_ids {
        if let Any::String(id) = vid {
            vehicles.remove(txn, id.as_ref());
        }
    }
    if !faction_id.is_empty()
        && let Some(Out::YMap(f)) = factions.get(txn, faction_id.as_str())
    {
        let arr = read_id_array(txn, factions, faction_id.as_str(), "squadIds");
        let remove: HashSet<&str> = HashSet::from([squad_id]);
        let kept = retain_ids(&arr, &remove);
        f.insert(txn, "squadIds", Any::Array(kept.into()));
    }
    squads.remove(txn, squad_id);
}

/// After mutations: empty squad → GC; else ensure `leaderSlotId ∈ slotIds` (promote to `[0]`).
fn ensure_leader_invariant_in_txn(
    txn: &mut TransactionMut,
    squads: &MapRef,
    factions: &MapRef,
    vehicles: &MapRef,
    squad_id: &str,
) {
    let ids = read_id_array(txn, squads, squad_id, "slotIds");
    if ids.is_empty() {
        garbage_collect_squad_in_txn(txn, squads, factions, vehicles, squad_id);
        return;
    }
    let leader_ok = match squads.get(txn, squad_id).and_then(|o| match o {
        Out::YMap(sq) => sq.get(txn, "leaderSlotId"),
        _ => None,
    }) {
        Some(Out::Any(Any::String(l))) => ids
            .iter()
            .any(|a| matches!(a, Any::String(s) if s.as_ref() == l.as_ref())),
        _ => false,
    };
    if !leader_ok && let Some(Any::String(first)) = ids.first() {
        set_leader_in_txn(txn, squads, squad_id, first.as_ref());
    }
}

/// Distance (m) a paste is offset from its originals when the cursor is off-map (`ydoc.PASTE_NUDGE`).
const PASTE_NUDGE: f64 = 20.0;

/// Read `map[key].field` (an `Any::Array` of string ids) as an owned `Vec<Any>`; empty when the
/// container map or the array field is absent. Seeds the `paste_slots` append accumulators and backs
/// [`append_id`].
fn read_id_array<T: ReadTxn>(txn: &T, map: &MapRef, key: &str, field: &str) -> Vec<Any> {
    match map.get(txn, key) {
        Some(Out::YMap(container)) => match container.get(txn, field) {
            Some(Out::Any(Any::Array(arr))) => arr.iter().cloned().collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Read `meta.environment` (an opaque `Any::Map`) as an owned `HashMap`; empty when absent. Backs
/// the `update_environment` / `apply_row_meta` `{...env, ...patch}` merges.
fn read_env_map<T: ReadTxn>(txn: &T, meta: &MapRef) -> HashMap<String, Any> {
    match meta.get(txn, "environment") {
        Some(Out::Any(Any::Map(m))) => (*m).clone(),
        _ => HashMap::new(),
    }
}

/// Delete `ids` (slots) and detach them from their squads' `slotIds` + every layer's `entityIds`,
/// inside an existing transaction. The `remove_slots` cascade, shared with `remove_editor_layer`.
fn remove_slots_in_txn(
    txn: &mut TransactionMut,
    slots: &MapRef,
    squads: &MapRef,
    editor_layers: &MapRef,
    ids: &[String],
) {
    if ids.is_empty() {
        return;
    }
    let id_set: HashSet<&str> = ids.iter().map(String::as_str).collect();

    // Affected squads (one filter each, not per slot).
    let mut affected: HashSet<String> = HashSet::new();
    for id in ids {
        if let Some(Out::YMap(slot)) = slots.get(&*txn, id.as_str())
            && let Some(Out::Any(Any::String(sid))) = slot.get(&*txn, "squadId")
        {
            affected.insert(sid.to_string());
        }
    }
    for sid in &affected {
        if let Some(Out::YMap(squad)) = squads.get(&*txn, sid)
            && let Some(Out::Any(Any::Array(arr))) = squad.get(&*txn, "slotIds")
        {
            let kept = retain_ids(&arr, &id_set);
            squad.insert(&mut *txn, "slotIds", Any::Array(kept.into()));
        }
    }

    // Each layer that held a removed id (collect ids first — can't mutate while iterating).
    let layer_ids: Vec<String> = editor_layers
        .iter(&*txn)
        .map(|(k, _)| k.to_string())
        .collect();
    for lid in &layer_ids {
        if let Some(Out::YMap(layer)) = editor_layers.get(&*txn, lid)
            && let Some(Out::Any(Any::Array(arr))) = layer.get(&*txn, "entityIds")
            && arr
                .iter()
                .any(|a| matches!(a, Any::String(s) if id_set.contains(s.as_ref())))
        {
            let kept = retain_ids(&arr, &id_set);
            layer.insert(&mut *txn, "entityIds", Any::Array(kept.into()));
        }
    }

    for id in ids {
        slots.remove(&mut *txn, id.as_str());
    }
}

/// Keys `hydrate` / `compile_payload` already understand at the payload root (T-219), plus the
/// reserved `payloadExtras` side-channel name (T-432 — never nest that key into itself / never
/// re-emit it as a wire key). Must match `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS` in
/// `mission/compile.rs` — duplicated here so the `doc` feature does not depend on `mission`.
fn is_known_editor_payload_top_level(key: &str) -> bool {
    matches!(
        key,
        "schemaVersion"
            | "map"
            | "environment"
            // T-505 — hydrate loads top-level `title` into meta (T-375 wire emit). Keep lockstep
            // with `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS` in `mission/compile.rs` when that list
            // is updated (out of this slice's owns if edited separately).
            | "title"
            | "loadouts"
            | "objectives"
            | "vehicles"
            | "entities"
            | "markers"
            | "editor"
            | "orbat"
            | "payloadExtras"
    )
}

/// Parse a JSON string to a `yrs` `Any` (JSON object → `Any::Map`, integer-valued numbers →
/// `Any::BigInt` to match Yjs's own integer encoding). `Any::Null` on a parse error. Backs the
/// `update_environment` patch merge + `hydrate` payload load without a yrs-version-specific
/// `Any::from_json`.
fn json_str_to_any(s: &str) -> Any {
    serde_json::from_str::<serde_json::Value>(s).map_or(Any::Null, |v| value_to_any(&v))
}

/// `serde_json::Value` → `yrs::Any`, recursively. Integer-valued numbers become `Any::BigInt`
/// (Yjs's integer encoding); other numbers `Any::Number`.
fn value_to_any(v: &serde_json::Value) -> Any {
    match v {
        serde_json::Value::Null => Any::Null,
        serde_json::Value::Bool(b) => Any::Bool(*b),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map_or_else(|| Any::Number(n.as_f64().unwrap_or(0.0)), Any::BigInt),
        serde_json::Value::String(s) => Any::String(s.as_str().into()),
        serde_json::Value::Array(arr) => {
            Any::Array(arr.iter().map(value_to_any).collect::<Vec<_>>().into())
        }
        serde_json::Value::Object(map) => {
            let m: HashMap<String, Any> = map
                .iter()
                .map(|(k, v)| (k.clone(), value_to_any(v)))
                .collect();
            Any::Map(Arc::new(m))
        }
    }
}

/// T-220 — load an array of entity rows into `map` and record the authored id sequence on
/// `entity_order[order_key]` so compile can rebuild arrays in hydrate order (yrs maps do not
/// preserve insertion order).
fn load_rows_ordered(
    txn: &mut TransactionMut,
    map: &MapRef,
    rows: Option<&Any>,
    entity_order: &MapRef,
    order_key: &str,
) {
    let mut ids: Vec<Any> = Vec::new();
    if let Some(Any::Array(arr)) = rows {
        for row in arr.iter() {
            if let Any::Map(fields) = row
                && let Some(Any::String(id)) = fields.get("id")
            {
                ids.push(Any::String(id.clone()));
            }
            load_row(txn, map, row);
        }
    }
    if !ids.is_empty() {
        entity_order.insert(txn, order_key, Any::Array(ids.into()));
    }
}

/// Load one entity row (an `Any::Map` with a string `id`) into `map` as a nested `MapRef`: create the
/// entity keyed by `id`, then insert every other field as its `Any` value — nested objects (e.g.
/// `position`) stay opaque `Any::Map`s, exactly like `ydoc.entityToYMap`. No-op on a missing id.
fn load_row(txn: &mut TransactionMut, map: &MapRef, row: &Any) {
    let Any::Map(fields) = row else { return };
    let Some(Any::String(id)) = fields.get("id") else {
        return;
    };
    let id = id.as_ref();
    let entity = map.insert(&mut *txn, id, MapPrelim::from([("id", id)]));
    for (k, v) in fields.iter() {
        if k != "id" {
            entity.insert(&mut *txn, k.as_str(), v.clone());
        }
    }
}

/// Coerce a `yrs` `Any` scalar to f64. **Yjs encodes integer-valued numbers as `Any::BigInt`** and
/// non-integers as `Any::Number`, so a position component can arrive as either — accept both.
fn any_to_f64(a: &Any) -> f64 {
    match a {
        Any::Number(n) => *n,
        Any::BigInt(i) => *i as f64,
        Any::Bool(true) => 1.0,
        Any::Bool(false) => 0.0,
        _ => 0.0,
    }
}

/// Read an opaque nested object off an entity row (`briefing`, `position`, …) as an OWNED map.
/// Missing or non-map reads as empty, so a caller can insert into the result and write the whole
/// value back — which is the only way to edit these: [`load_row`] stores nested objects as opaque
/// `Any::Map`s, not as tracked `YMap`s, so there is no sub-key to insert into (T-345).
///
/// A non-map `briefing` (the `"briefing": "some prose"` mistake T-367's precheck exists to reject)
/// reads as empty and is overwritten by a well-formed one, which is the repair the author wants.
fn read_any_map<T: ReadTxn>(txn: &T, row: &MapRef, key: &str) -> HashMap<String, Any> {
    match row.get(txn, key) {
        Some(Out::Any(Any::Map(m))) => (*m).clone(),
        _ => HashMap::new(),
    }
}

/// `briefing.markers` as an owned vec; missing or non-array reads as empty (T-345).
fn briefing_markers(briefing: &HashMap<String, Any>) -> Vec<Any> {
    match briefing.get("markers") {
        Some(Any::Array(arr)) => arr.iter().cloned().collect(),
        _ => Vec::new(),
    }
}

/// A marker row's doc-internal `id`, when it has a string one (T-345). `None` for a row written by
/// something that did not stamp one — such a row is still emitted, it just cannot be addressed.
fn marker_row_id(row: &Any) -> Option<&str> {
    let Any::Map(fields) = row else { return None };
    match fields.get("id") {
        Some(Any::String(s)) => Some(s.as_ref()),
        _ => None,
    }
}

/// One `{id, x, z, icon, label}` marker row (T-345). `x`/`z` go in as `Any::Number` (an f64
/// coordinate); a hydrate round-trip may bring an integral one back as `Any::BigInt`, because
/// [`value_to_any`] encodes integer-valued JSON numbers that way. Harmless in both directions — the
/// two encode to the same JSON number and [`any_to_f64`] accepts either.
///
/// The `id` is the doc-only key; see [`MissionDocCore::set_faction_briefing_marker`] for why it is
/// here and why the emitter needs no change to keep it off the wire.
fn marker_any(id: &str, x: f64, z: f64, icon: &str, label: &str) -> Any {
    Any::Map(Arc::new(HashMap::from([
        ("id".to_string(), Any::String(id.into())),
        ("x".to_string(), Any::Number(x)),
        ("z".to_string(), Any::Number(z)),
        ("icon".to_string(), Any::String(icon.into())),
        ("label".to_string(), Any::String(label.into())),
    ])))
}

/// Read `position` (`Any::Map`) → `(x, y, z, rotation)`; missing map/keys read as 0.
fn read_position<T: ReadTxn>(txn: &T, slot: &MapRef) -> (f64, f64, f64, f64) {
    if let Some(Out::Any(Any::Map(m))) = slot.get(txn, "position") {
        let g = |k: &str| m.get(k).map_or(0.0, any_to_f64);
        (g("x"), g("y"), g("z"), g("rotation"))
    } else {
        (0.0, 0.0, 0.0, 0.0)
    }
}

/// Owned clone of the slot's `position` map (empty when absent) — for T-220 merge-on-edit.
fn read_position_map<T: ReadTxn>(txn: &T, slot: &MapRef) -> HashMap<String, Any> {
    match slot.get(txn, "position") {
        Some(Out::Any(Any::Map(m))) => (*m).clone(),
        _ => HashMap::new(),
    }
}

/// Read a string-valued slot field (`role`/`tag`/`squadId`/`stance`), or `None` if absent/non-string.
fn read_str<T: ReadTxn>(txn: &T, slot: &MapRef, key: &str) -> Option<String> {
    match slot.get(txn, key) {
        Some(Out::Any(Any::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

/// Map `stance` string → dense code (default `stand`).
fn read_stance<T: ReadTxn>(txn: &T, slot: &MapRef) -> u8 {
    match read_str(txn, slot, "stance").as_deref() {
        Some("crouch") => STANCE_CROUCH,
        Some("prone") => STANCE_PRONE,
        _ => STANCE_STAND,
    }
}

/// T-180.3 — resolve faction side key for a squad id (`BLUFOR` when any hop is missing).
fn resolve_slot_side_key<T: ReadTxn>(
    txn: &T,
    squads: &MapRef,
    factions: &MapRef,
    squad_id: &str,
) -> String {
    if squad_id.is_empty() {
        return String::from("BLUFOR");
    }
    let Some(Out::YMap(sq)) = squads.get(txn, squad_id) else {
        return String::from("BLUFOR");
    };
    let Some(faction_id) = read_str(txn, &sq, "factionId") else {
        return String::from("BLUFOR");
    };
    if faction_id.is_empty() {
        return String::from("BLUFOR");
    }
    let Some(Out::YMap(f)) = factions.get(txn, faction_id.as_str()) else {
        return String::from("BLUFOR");
    };
    match read_str(txn, &f, "key") {
        Some(k) if !k.is_empty() => k,
        _ => String::from("BLUFOR"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids_sorted(soa: &SlotSoa) -> Vec<String> {
        let mut v = soa.ids.clone();
        v.sort();
        v
    }

    /// Row lookup by id — parity is set-equality, not row order.
    fn row_of(soa: &SlotSoa, id: &str) -> usize {
        soa.ids.iter().position(|s| s == id).expect("id present")
    }

    /// The in-crate twin of the browser gate's `__missionPersist.slots_digest()`: sorted
    /// `(id, x.to_bits(), y.to_bits())` rows. Byte equality, not tolerance — yrs restores the prior
    /// values rather than recomputing them, so an undo lands on the exact f32 bits.
    fn slots_digest(soa: &SlotSoa) -> Vec<(String, u32, u32)> {
        let mut rows: Vec<(String, u32, u32)> = soa
            .ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                (
                    id.clone(),
                    soa.xy[i * 2].to_bits(),
                    soa.xy[i * 2 + 1].to_bits(),
                )
            })
            .collect();
        rows.sort();
        rows
    }

    /// The browser boot: 8 slots written under `INIT` (untracked), exactly like
    /// `mission_doc::new_seeded` — so the undo stack starts empty and every step below is a LOCAL
    /// user gesture.
    fn seeded_core() -> MissionDocCore {
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.seed_random(8, 12800.0, 12800.0, 42);
        doc.set_origin_init(false);
        assert!(!doc.can_undo(), "the INIT seed must not be an undo step");
        doc
    }

    /// T-159.22.1 — **one LOCAL transaction = one undo step**, across a step *boundary*.
    ///
    /// The case `undo_redo_sequence` never covered and `smoke_undo_editor` could not see (it made
    /// exactly one mutation, so it never proved that undoing the 2nd move leaves the 1st standing).
    /// Two `move_entities` on the same slot must be two stack items: the first undo lands on `d1`,
    /// not `d0`.
    ///
    /// **This test was green the day it was written** — T-159.22 reported the invariant broken, but
    /// the mechanism was a double-fired Ctrl+Z in the gate driver, not the core (root cause:
    /// `.ai/artifacts/t159_22_1_verify_log.md`). It is kept as the regression pin the doubt earned:
    /// `undo_depth()` is asserted, so a future capture-side merge fails here rather than in a browser.
    #[test]
    fn two_local_moves_are_two_undo_steps() {
        let mut doc = seeded_core();
        let d0 = slots_digest(&doc.materialize());

        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        let d1 = slots_digest(&doc.materialize());
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        let d2 = slots_digest(&doc.materialize());
        assert_ne!(d0, d1, "move 1 changed the doc");
        assert_ne!(d1, d2, "move 2 changed the doc");
        assert_eq!(doc.undo_depth(), 2, "two LOCAL txns = two stack items");

        assert!(doc.undo());
        assert_eq!(
            slots_digest(&doc.materialize()),
            d1,
            "undo 1 reverts ONLY move 2"
        );
        assert!(doc.can_undo(), "move 1 is still on the stack");

        assert!(doc.undo());
        assert_eq!(
            slots_digest(&doc.materialize()),
            d0,
            "undo 2 reverts move 1"
        );
        assert!(!doc.can_undo(), "the stack is now empty");
    }

    /// The same boundary over the place path (`add_slot`), the second shape from the T-159.22 repro
    /// (two places → one undo removed BOTH slots, 8 → 9 → 10 → 8). Also green on baseline — same
    /// root cause.
    #[test]
    fn two_local_places_are_two_undo_steps() {
        let mut doc = seeded_core();
        assert_eq!(doc.materialize().len(), 8);

        doc.add_slot(
            "p1", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 100.0, 0.0, 0.0,
        );
        doc.add_slot(
            "p2", "sq", "lyr", 1, "Rifleman", None, None, 200.0, 200.0, 0.0, 0.0,
        );
        assert_eq!(doc.materialize().len(), 10);

        assert!(doc.undo());
        assert_eq!(doc.materialize().len(), 9, "undo 1 removes ONLY p2");
        assert!(doc.can_undo());

        assert!(doc.undo());
        assert_eq!(doc.materialize().len(), 8, "undo 2 removes p1");
    }

    #[test]
    fn add_slot_materializes_soa() {
        let doc = MissionDocCore::new();
        doc.add_slot(
            "s1", "sq1", "lyr", 0, "Rifleman", None, None, 100.5, 200.25, 0.0, 0.0,
        );
        doc.add_slot(
            "s2",
            "sq1",
            "lyr",
            1,
            "Squad Leader",
            None,
            None,
            300.0,
            400.0,
            5.0,
            90.0,
        );

        let soa = doc.materialize();
        assert_eq!(soa.len(), 2);
        assert_eq!(ids_sorted(&soa), vec!["s1".to_string(), "s2".to_string()]);

        let r1 = row_of(&soa, "s1");
        assert_eq!(soa.xs[r1], 100.5_f32);
        assert_eq!(soa.ys[r1], 200.25_f32);
        assert_eq!(soa.stance[r1], STANCE_STAND);
        assert_eq!(soa.squads[soa.squad_idx[r1] as usize], "sq1");
        assert_eq!(soa.roles[soa.role_idx[r1] as usize], "Rifleman");
        assert_eq!(soa.tag_idx[r1], NONE_IDX);

        let r2 = row_of(&soa, "s2");
        assert_eq!(soa.rotations[r2], 90.0_f32);
        assert_eq!(soa.roles[soa.role_idx[r2] as usize], "Squad Leader");
    }

    #[test]
    fn apply_update_from_peer_with_bigint_position() {
        // A second yrs doc plays the "JS Y.Doc" peer; integer-valued z/rotation take the Any::BigInt
        // path to prove the position reader accepts BigInt as well as Number.
        let peer = Doc::with_client_id(999);
        let pslots = peer.get_or_insert_map("slots");
        {
            let mut txn = peer.transact_mut();
            let slot = pslots.insert(&mut txn, "p1", MapPrelim::from([("id", "p1")]));
            slot.insert(&mut txn, "role", "Medic");
            slot.insert(&mut txn, "squadId", "sq9");
            slot.insert(&mut txn, "stance", "prone");
            let mut pos: HashMap<String, Any> = HashMap::new();
            pos.insert("x".to_string(), Any::Number(12.5));
            pos.insert("y".to_string(), Any::Number(34.75));
            pos.insert("z".to_string(), Any::BigInt(0));
            pos.insert("rotation".to_string(), Any::BigInt(180));
            slot.insert(&mut txn, "position", Any::Map(Arc::new(pos)));
        }
        let update = peer
            .transact()
            .encode_state_as_update_v1(&StateVector::default());

        let doc = MissionDocCore::new();
        doc.apply_update(&update).expect("apply ok");
        let soa = doc.materialize();
        assert_eq!(soa.len(), 1);

        let r = row_of(&soa, "p1");
        assert_eq!(soa.xs[r], 12.5_f32);
        assert_eq!(soa.ys[r], 34.75_f32);
        assert_eq!(soa.zs[r], 0.0_f32);
        assert_eq!(soa.rotations[r], 180.0_f32);
        assert_eq!(soa.stance[r], STANCE_PRONE);
        assert_eq!(soa.roles[soa.role_idx[r] as usize], "Medic");
    }

    #[test]
    fn encode_decode_roundtrip_is_stable() {
        let a = MissionDocCore::new();
        a.add_slot(
            "s1", "sq1", "lyr", 0, "Rifleman", None, None, 1.0, 2.0, 3.0, 4.0,
        );
        a.add_slot(
            "s2", "sq1", "lyr", 1, "Medic", None, None, 5.0, 6.0, 7.0, 8.0,
        );
        let bytes = a.encode_state();

        let b = MissionDocCore::new();
        b.apply_update(&bytes).expect("apply ok");
        let sa = a.materialize();
        let sb = b.materialize();
        assert_eq!(ids_sorted(&sa), ids_sorted(&sb));
        for id in &sa.ids {
            let ra = row_of(&sa, id);
            let rb = row_of(&sb, id);
            assert_eq!(sa.xs[ra], sb.xs[rb]);
            assert_eq!(sa.rotations[ra], sb.rotations[rb]);
        }
        // Re-encoding the same document twice is byte-identical (deterministic v1 encode + fixed id).
        assert_eq!(a.encode_state(), bytes);
    }

    #[test]
    fn undo_redo_sequence() {
        let mut doc = MissionDocCore::new();
        assert!(!doc.can_undo());
        doc.add_slot(
            "s1", "sq1", "lyr", 0, "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
        );
        doc.add_slot(
            "s2", "sq1", "lyr", 1, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
        );
        doc.add_slot(
            "s3", "sq1", "lyr", 2, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
        );
        assert_eq!(doc.materialize().len(), 3);

        assert!(doc.undo()); // one step = one add_slot → removes s3
        assert_eq!(
            ids_sorted(&doc.materialize()),
            vec!["s1".to_string(), "s2".to_string()]
        );
        assert!(doc.undo()); // removes s2
        assert_eq!(ids_sorted(&doc.materialize()), vec!["s1".to_string()]);
        assert!(doc.redo()); // restores s2
        assert_eq!(
            ids_sorted(&doc.materialize()),
            vec!["s1".to_string(), "s2".to_string()]
        );
    }

    #[test]
    fn init_mode_transactions_are_not_undoable() {
        // The flip's undo-origin split: LOCAL user gestures are undoable; INIT (seed / hydrate /
        // restore) is not. Proves `set_origin_init` + the `tracked_origins = {LOCAL}` scoping.
        let mut doc = MissionDocCore::new();
        doc.add_editor_layer("l1", "Alpha", None); // LOCAL
        assert!(doc.can_undo(), "a LOCAL op is undoable");
        assert!(doc.undo());
        assert!(!doc.can_undo());

        doc.set_origin_init(true);
        doc.seed_meta("m1", "Op");
        doc.add_editor_layer("l2", "Bravo", None); // INIT
        doc.set_origin_init(false);
        assert!(!doc.can_undo(), "INIT ops must not push undo steps");

        doc.add_editor_layer("l3", "Charlie", None); // LOCAL again
        assert!(doc.can_undo());
        assert!(doc.undo()); // undoes only the LOCAL Charlie
        assert!(!doc.can_undo());
        // The INIT-seeded l2 + Bravo survive the undo (never tracked).
        assert!(doc.small_maps_json().contains("\"l2\""));
    }

    #[test]
    fn small_maps_json_shape_on_empty_doc() {
        let doc = MissionDocCore::new();
        let json = doc.small_maps_json();
        assert!(json.contains("\"meta\":null"), "{json}"); // empty meta → null (matches docToSnapshot)
        for key in [
            "factionsById",
            "squadsById",
            "loadoutsById",
            "itemsById",
            "objectivesById",
            "vehiclesById",
            "entitiesById",
            "markersById",
            "editorLayersById",
        ] {
            assert!(
                json.contains(&format!("\"{key}\":")),
                "missing {key} in {json}"
            );
        }
    }

    #[test]
    fn small_maps_json_includes_applied_entities() {
        // A peer doc authors a faction + meta title; applying its update must surface both.
        let peer = Doc::with_client_id(7);
        let factions = peer.get_or_insert_map("factions");
        let meta = peer.get_or_insert_map("meta");
        {
            let mut txn = peer.transact_mut();
            let f = factions.insert(&mut txn, "f1", MapPrelim::from([("id", "f1")]));
            f.insert(&mut txn, "name", "BLUFOR");
            meta.insert(&mut txn, "title", "Op Test");
        }
        let update = peer
            .transact()
            .encode_state_as_update_v1(&StateVector::default());

        let doc = MissionDocCore::new();
        doc.apply_update(&update).expect("apply ok");
        let json = doc.small_maps_json();
        assert!(json.contains("\"f1\""), "{json}");
        assert!(json.contains("BLUFOR"), "{json}");
        assert!(json.contains("Op Test"), "{json}");
        assert!(
            !json.contains("\"meta\":null"),
            "meta should be populated: {json}"
        );
    }

    #[test]
    fn slots_json_roundtrips_a_slot() {
        let doc = MissionDocCore::new();
        doc.add_slot(
            "s1", "sq1", "lyr", 0, "Rifleman", None, None, 100.5, 200.25, 0.0, 90.0,
        );
        let json = doc.slots_json();
        assert!(json.contains("\"s1\""), "{json}");
        assert!(json.contains("Rifleman"), "{json}");
        assert!(json.contains("100.5"), "{json}"); // exact f64 position (not the f32 SoA)
    }

    #[test]
    fn update_slot_loadout_roundtrips_and_clears() {
        let mut doc = MissionDocCore::new();
        doc.add_slot(
            "s1", "sq1", "lyr", 0, "Rifleman", None, None, 1.0, 2.0, 0.0, 0.0,
        );
        let json = r#"{"primary":"{AAA}Rifle_M16A2.et","uniform":null,"vest":null,"helmet":null,"optic":"{BBB}Optic_Acog.et","magazine":null,"summary":"M16A2 · ACOG"}"#;
        doc.update_slot_loadout("s1", Some(json.to_string()));

        let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
        let lo = &v["s1"]["loadout"];
        assert_eq!(lo["primary"], "{AAA}Rifle_M16A2.et");
        assert_eq!(lo["optic"], "{BBB}Optic_Acog.et");
        assert_eq!(lo["summary"], "M16A2 · ACOG");
        assert!(lo["uniform"].is_null(), "explicit null survives: {lo}");

        // One LOCAL transaction = one undo step: undo removes only the loadout.
        assert!(doc.undo());
        let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
        assert!(v["s1"].get("loadout").is_none(), "undo cleared loadout");
        assert_eq!(v["s1"]["role"], "Rifleman", "slot itself survives");

        // Explicit clear path (None removes the key), and a missing slot is a no-op.
        doc.update_slot_loadout("s1", Some(json.to_string()));
        doc.update_slot_loadout("s1", None);
        let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
        assert!(v["s1"].get("loadout").is_none(), "None clears the key");
        doc.update_slot_loadout("missing", Some(json.to_string()));
    }

    #[test]
    fn paste_slots_copies_loadout() {
        let doc = MissionDocCore::new();
        doc.add_editor_layer("lyr", "Default", None);
        doc.paste_slots(
            vec!["p1".into(), "p2".into()],
            vec!["sq1".into(), "sq1".into()],
            vec!["lyr".into(), "lyr".into()],
            vec![10.0, 20.0],
            vec![10.0, 20.0],
            vec![0.0, 0.0],
            vec![0.0, 0.0],
            vec!["Rifleman".into(), "Medic".into()],
            vec![String::new(), "MED".into()],
            vec![String::new(), String::new()],
            vec!["stand".into(), "prone".into()],
            vec![
                r#"{"primary":"{AAA}Rifle_M16A2.et","optic":null}"#.into(),
                String::new(),
            ],
            vec![String::new(), String::new()],
            Some(100.0),
            Some(100.0),
            12800.0,
            12800.0,
        );
        let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
        assert_eq!(v["p1"]["loadout"]["primary"], "{AAA}Rifle_M16A2.et");
        assert!(v["p1"]["loadout"]["optic"].is_null());
        assert!(
            v["p2"].get("loadout").is_none(),
            "empty string = no loadout copied"
        );
    }

    #[test]
    fn remove_editor_layer_reseeds_when_subtree_is_all_layers() {
        // root + child-of-root are the only layers; a slot filed in child. Removing root deletes the
        // whole subtree (= every layer) → a default layer is reseeded with the JS-minted id, and the
        // filed slot cascades away. Structural (the reseed path has no ydoc byte-parity twin to gate).
        let doc = MissionDocCore::new();
        doc.add_editor_layer("root", "Root", None);
        doc.add_editor_layer("child", "Child", Some("root".to_string()));
        doc.add_slot(
            "s1", "sq1", "child", 0, "Rifleman", None, None, 1.0, 2.0, 0.0, 0.0,
        );

        doc.remove_editor_layer("root", "reseed-1");

        assert_eq!(doc.slot_count(), 0, "the filed slot cascaded away");
        let json = doc.small_maps_json();
        assert!(json.contains("reseed-1"), "reseeded default id: {json}");
        assert!(json.contains("Default Layer"), "{json}");
        assert!(!json.contains("\"root\""), "root deleted: {json}");
        assert!(!json.contains("\"child\""), "child deleted: {json}");
    }

    // ── T-180.2 ORBAT graph mutators (B1–B7) ───────────────────────────────────────────────────

    fn orbat_fixture() -> MissionDocCore {
        let doc = MissionDocCore::new();
        doc.add_editor_layer("lyr", "Layer", None);
        doc.add_faction("faction-BLUFOR", "BLUFOR", "BLUFOR");
        doc.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
        doc.add_squad("sq-b", "faction-BLUFOR", "Bravo", None);
        doc
    }

    fn small_maps(doc: &MissionDocCore) -> serde_json::Value {
        serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json")
    }

    fn slots_map(doc: &MissionDocCore) -> serde_json::Value {
        serde_json::from_str(&doc.slots_json()).expect("slots_json")
    }

    /// B1 — set_leader(B) after leader A ⇒ leaderSlotId=B only.
    #[test]
    fn set_leader_exclusive() {
        let doc = orbat_fixture();
        doc.add_slot("a", "sq-a", "lyr", 0, "SL", None, None, 1.0, 1.0, 0.0, 0.0);
        doc.add_slot(
            "b", "sq-a", "lyr", 1, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
        );
        doc.set_leader("sq-a", "a");
        doc.set_leader("sq-a", "b");
        let root = small_maps(&doc);
        assert_eq!(root["squadsById"]["sq-a"]["leaderSlotId"], "b");
        assert_ne!(root["squadsById"]["sq-a"]["leaderSlotId"], "a");
    }

    /// B2 — move last slot away ⇒ source squad key absent from squads map.
    #[test]
    fn empty_squad_garbage_collected() {
        let doc = orbat_fixture();
        doc.add_slot(
            "solo", "sq-a", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
        );
        doc.set_leader("sq-a", "solo");
        doc.add_vehicle("v1", "Prefab/Vehicle.et", None, None, None, None);
        doc.attach_vehicle("sq-a", "v1");
        doc.move_slot_to_squad("solo", "sq-b");
        let root = small_maps(&doc);
        assert!(
            root["squadsById"].get("sq-a").is_none(),
            "empty source must be GC'd: {}",
            root["squadsById"]
        );
        assert!(
            root["vehiclesById"].get("v1").is_none(),
            "vehicles attached only to GC'd squad must be deleted"
        );
        assert_eq!(slots_map(&doc)["solo"]["squadId"], "sq-b");
    }

    /// B3 — move_slot: source without id; dest with id; slot.squadId=dest.
    #[test]
    fn move_slot_bidirectional() {
        let doc = orbat_fixture();
        doc.add_slot(
            "m", "sq-a", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
        );
        doc.add_slot(
            "keep", "sq-a", "lyr", 1, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
        );
        doc.set_leader("sq-a", "keep");
        doc.move_slot_to_squad("m", "sq-b");
        let root = small_maps(&doc);
        let src_ids = root["squadsById"]["sq-a"]["slotIds"]
            .as_array()
            .expect("slotIds");
        let dst_ids = root["squadsById"]["sq-b"]["slotIds"]
            .as_array()
            .expect("slotIds");
        assert!(!src_ids.iter().any(|v| v == "m"));
        assert!(dst_ids.iter().any(|v| v == "m"));
        assert_eq!(slots_map(&doc)["m"]["squadId"], "sq-b");
    }

    /// B4 — after mutator fixture, every remaining squad has leader ∈ slotIds.
    #[test]
    fn leader_invariant_holds() {
        let doc = orbat_fixture();
        doc.add_slot("a1", "sq-a", "lyr", 0, "SL", None, None, 1.0, 1.0, 0.0, 0.0);
        doc.add_slot(
            "a2", "sq-a", "lyr", 1, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
        );
        doc.set_leader("sq-a", "a1");
        doc.add_slot(
            "b1", "sq-b", "lyr", 0, "Rifleman", None, None, 3.0, 3.0, 0.0, 0.0,
        );
        doc.move_slot_to_squad("a1", "sq-b");
        doc.set_leader("sq-b", "a1");
        let root = small_maps(&doc);
        let squads = root["squadsById"].as_object().expect("squadsById");
        for (sid, sq) in squads {
            let slot_ids = sq["slotIds"].as_array().expect("slotIds");
            assert!(
                !slot_ids.is_empty(),
                "empty squad {sid} should have been GC'd"
            );
            let leader = sq["leaderSlotId"].as_str().expect("leaderSlotId");
            assert!(
                slot_ids.iter().any(|v| v.as_str() == Some(leader)),
                "squad {sid}: leader {leader} not in {slot_ids:?}"
            );
        }
    }

    /// B5 — move leader away with members left ⇒ remaining[0] is leader.
    #[test]
    fn move_leader_promotes_next() {
        let doc = orbat_fixture();
        doc.add_slot(
            "lead", "sq-a", "lyr", 0, "SL", None, None, 1.0, 1.0, 0.0, 0.0,
        );
        doc.add_slot(
            "next", "sq-a", "lyr", 1, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
        );
        doc.add_slot(
            "tail", "sq-a", "lyr", 2, "Medic", None, None, 3.0, 3.0, 0.0, 0.0,
        );
        doc.set_leader("sq-a", "lead");
        doc.move_slot_to_squad("lead", "sq-b");
        let root = small_maps(&doc);
        assert_eq!(root["squadsById"]["sq-a"]["leaderSlotId"], "next");
        let ids = root["squadsById"]["sq-a"]["slotIds"]
            .as_array()
            .expect("slotIds");
        assert_eq!(ids[0], "next");
    }

    /// B6 — add_vehicle + attach then detach vehicleIds.
    #[test]
    fn attach_vehicle_roundtrip() {
        let doc = orbat_fixture();
        doc.add_slot(
            "s", "sq-a", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
        );
        doc.set_leader("sq-a", "s");
        doc.add_vehicle(
            "veh-1",
            "Prefabs/Vehicles/Wheeled/M113/M113.et",
            Some(10.0),
            Some(20.0),
            Some(0.0),
            Some(90.0),
        );
        doc.attach_vehicle("sq-a", "veh-1");
        let root = small_maps(&doc);
        let vids = root["squadsById"]["sq-a"]["vehicleIds"]
            .as_array()
            .expect("vehicleIds");
        assert!(vids.iter().any(|v| v == "veh-1"));
        assert_eq!(root["vehiclesById"]["veh-1"]["squadId"], "sq-a");
        assert_eq!(
            root["vehiclesById"]["veh-1"]["resourceName"],
            "Prefabs/Vehicles/Wheeled/M113/M113.et"
        );
        doc.detach_vehicle("sq-a", "veh-1");
        let root = small_maps(&doc);
        let vids = root["squadsById"]["sq-a"]["vehicleIds"]
            .as_array()
            .expect("vehicleIds");
        assert!(!vids.iter().any(|v| v == "veh-1"));
        assert!(
            root["vehiclesById"].get("veh-1").is_some(),
            "detach must keep the vehicle row"
        );
        assert!(root["vehiclesById"]["veh-1"].get("squadId").is_none());
    }

    /// T-254 — `add_entity` writes alias + resourceName + position into `entitiesById`, and
    /// `set_entity_faction` stamps the schema factionKey. Undo removes the row.
    #[test]
    fn add_entity_materializes_entities_by_id_and_is_undoable() {
        let mut doc = MissionDocCore::new();
        doc.add_entity(
            "e1",
            "prop:ammo_crate",
            "{FA}Prefabs/Props/Military/AmmoBox.et",
            100.0,
            200.0,
            0.0,
            90.0,
        );
        doc.set_entity_faction("e1", "blufor");
        let root = small_maps(&doc);
        let row = &root["entitiesById"]["e1"];
        assert_eq!(row["alias"], "prop:ammo_crate");
        assert_eq!(row["resourceName"], "{FA}Prefabs/Props/Military/AmmoBox.et");
        assert_eq!(row["faction"], "blufor");
        assert_eq!(row["position"]["x"], 100.0);
        assert_eq!(row["position"]["y"], 200.0);
        assert_eq!(row["position"]["rotation"], 90.0);
        // add_entity and set_entity_faction are separate LOCAL txns (same pattern as vehicles).
        assert!(doc.undo()); // faction
        assert!(doc.undo()); // row
        let root = small_maps(&doc);
        assert!(
            root["entitiesById"].get("e1").is_none(),
            "two undos must remove the placed entity"
        );
    }

    /// The vehicle rows of a doc, keyed by id, straight off `small_maps_json` (T-215).
    fn vehicles_of(doc: &MissionDocCore) -> serde_json::Value {
        small_maps(doc)["vehiclesById"].clone()
    }

    /// Save → reload, exactly as the editor does it: compile the doc to a `json_payload` and
    /// `hydrate` a fresh core from it (T-215).
    ///
    /// `#[cfg(feature = "mission")]` — the reason the suite must run `--features doc,mission`;
    /// `--features doc` alone compiles the compiler out and silently skips its callers.
    #[cfg(feature = "mission")]
    fn save_and_reload(doc: &MissionDocCore) -> MissionDocCore {
        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&payload.to_string(), "lyr");
        reloaded
    }

    /// **T-219 — unknown top-level keys must survive hydrate → compile → hydrate → compile.**
    ///
    /// Before this ticket, `hydrate` only read known paths and `compile_payload` rebuilt from a
    /// `json!` of known fields, so a server-first / migration key appeared to persist then vanished
    /// on the next Save. The fixture uses a non-integral nested number so an `Any::BigInt` vs
    /// `Any::Number` round-trip cannot silently paper over a drop.
    #[cfg(feature = "mission")]
    #[test]
    fn unknown_top_level_keys_survive_compile_hydrate_compile() {
        let incoming = serde_json::json!({
            "schemaVersion": 1,
            "map": { "terrain": "everon" },
            "environment": {},
            "serverMigrationToken": "keep-me-v2",
            "featureFlags": { "alpha": true, "n": 42.5 },
            "editor": {
                "factions": [],
                "squads": [],
                "slots": [],
                "editorLayers": []
            }
        });

        let doc = MissionDocCore::new();
        doc.hydrate(&incoming.to_string(), "lyr");

        let small = small_maps(&doc);
        assert_eq!(
            small["payloadExtras"]["serverMigrationToken"],
            serde_json::json!("keep-me-v2"),
            "hydrate must park unknown keys in payloadExtras"
        );
        assert_eq!(
            small["payloadExtras"]["featureFlags"]["n"],
            serde_json::json!(42.5)
        );

        let compiled = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        assert_eq!(
            compiled["serverMigrationToken"],
            serde_json::json!("keep-me-v2"),
            "compile must re-emit parked unknown keys onto the wire payload"
        );
        assert_eq!(
            compiled["featureFlags"],
            serde_json::json!({ "alpha": true, "n": 42.5 })
        );
        assert!(
            compiled.get("payloadExtras").is_none(),
            "payloadExtras is a small_maps side-channel, never a wire key"
        );

        let reloaded = save_and_reload(&doc);
        let recompiled = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        assert_eq!(
            recompiled["serverMigrationToken"],
            serde_json::json!("keep-me-v2"),
            "unknown keys must survive a full Save→reload→Save cycle"
        );
        assert_eq!(
            recompiled["featureFlags"],
            serde_json::json!({ "alpha": true, "n": 42.5 })
        );
        assert_eq!(recompiled["map"]["terrain"], serde_json::json!("everon"));
        assert_eq!(recompiled["schemaVersion"], serde_json::json!(1));
    }

    /// T-432 — Class R: an authored top-level wire key literally named `payloadExtras` must not
    /// nest into the side-channel or reappear on the compiled wire. Policy: **reserved-key
    /// collision** — nested contents are dropped (not renamed / re-parked). Unrelated unknown
    /// keys still park and re-emit. Empty `payloadExtras` remains omitted from `small_maps_json`.
    #[cfg(feature = "mission")]
    #[test]
    fn authored_payload_extras_key_is_reserved_not_reemitted() {
        let incoming = serde_json::json!({
            "schemaVersion": 1,
            "map": { "terrain": "everon" },
            "environment": {},
            "payloadExtras": { "nested": true },
            "serverMigrationToken": "keep-me-v2",
            "editor": {
                "factions": [],
                "squads": [],
                "slots": [],
                "editorLayers": []
            }
        });

        let doc = MissionDocCore::new();
        doc.hydrate(&incoming.to_string(), "lyr");

        let small = small_maps(&doc);
        assert!(
            small
                .get("payloadExtras")
                .and_then(|e| e.get("payloadExtras"))
                .is_none(),
            "hydrate must not nest the reserved side-channel name into itself; got {:?}",
            small.get("payloadExtras")
        );
        assert_eq!(
            small["payloadExtras"]["serverMigrationToken"],
            serde_json::json!("keep-me-v2"),
            "unrelated unknown keys must still park"
        );

        let compiled = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        assert!(
            compiled.get("payloadExtras").is_none(),
            "compiled wire must not carry the side-channel name; got {:?}",
            compiled.get("payloadExtras")
        );
        assert_eq!(
            compiled["serverMigrationToken"],
            serde_json::json!("keep-me-v2")
        );
        // Nested reserved contents dropped — not silently renamed under another key.
        assert!(
            compiled
                .as_object()
                .map(|o| !o
                    .values()
                    .any(|v| v == &serde_json::json!({ "nested": true })))
                .unwrap_or(false),
            "reserved nested object must not be re-emitted under another wire key; got {compiled:?}"
        );
    }

    /// T-219 — a second hydrate without the extras must clear the parked map (no sticky ghosts).
    #[cfg(feature = "mission")]
    #[test]
    fn hydrate_without_unknown_keys_clears_payload_extras() {
        let with = serde_json::json!({
            "schemaVersion": 1,
            "map": { "terrain": "everon" },
            "serverMigrationToken": "ghost",
            "editor": {
                "factions": [],
                "squads": [],
                "slots": [],
                "editorLayers": []
            }
        });
        let without = serde_json::json!({
            "schemaVersion": 1,
            "map": { "terrain": "everon" },
            "editor": {
                "factions": [],
                "squads": [],
                "slots": [],
                "editorLayers": []
            }
        });

        let doc = MissionDocCore::new();
        doc.hydrate(&with.to_string(), "lyr");
        assert!(
            small_maps(&doc)["payloadExtras"]
                .get("serverMigrationToken")
                .is_some()
        );

        doc.hydrate(&without.to_string(), "lyr");
        let small = small_maps(&doc);
        assert!(
            small.get("payloadExtras").is_none(),
            "empty extras must be omitted from small_maps_json; got {small:?}"
        );
        let compiled = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        assert!(compiled.get("serverMigrationToken").is_none());
    }

    /// T-505 Class R — hydrate loads T-375 top-level `title` into meta (trim-aware).
    ///
    /// Simulates cold adopt: hydrate payload, then `apply_row_meta` with the **preferred** title
    /// (payload wins over stale row) — the same prefer rule `mission_hydrate::adopt_payload` uses.
    ///
    /// RED (perturbation): stop loading `title` in `hydrate` **and** pass only the stale row title
    /// into `apply_row_meta` → assert equals `"Authored Bridgehead"`.
    #[test]
    fn t505_hydrate_and_prefer_payload_title_over_stale_row() {
        let payload = serde_json::json!({
            "schemaVersion": 1,
            "title": "  Authored Bridgehead  ",
            "map": { "terrain": "everon" },
            "environment": {},
            "editor": {
                "factions": [],
                "squads": [],
                "slots": [],
                "editorLayers": []
            }
        });
        let doc = MissionDocCore::new();
        doc.set_title("Stale Library Title");
        doc.hydrate(&payload.to_string(), "lyr");
        assert_eq!(
            small_maps(&doc)["meta"]["title"],
            "Authored Bridgehead",
            "hydrate must load trimmed payload title into meta"
        );
        // Prefer-payload (adopt_payload rule): never hand the stale row title through when the
        // payload carries a non-blank title.
        let preferred = payload
            .get("title")
            .and_then(|t| t.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Stale Library Title");
        doc.apply_row_meta(preferred, "everon", None, None, None);
        assert_eq!(
            small_maps(&doc)["meta"]["title"],
            "Authored Bridgehead",
            "prefer-payload title must survive apply_row_meta; got {:?}",
            small_maps(&doc)["meta"]["title"]
        );
        assert!(
            small_maps(&doc).get("payloadExtras").is_none()
                || small_maps(&doc)["payloadExtras"].get("title").is_none(),
            "title is a known hydrate key — must not park in payloadExtras"
        );
    }

    /// T-505 — whitespace-only / absent payload title does not invent a meta title; row can fill.
    #[test]
    fn t505_hydrate_whitespace_title_ignored_row_can_fill() {
        let payload = serde_json::json!({
            "schemaVersion": 1,
            "title": "   ",
            "map": { "terrain": "everon" },
            "environment": {},
            "editor": {
                "factions": [],
                "squads": [],
                "slots": [],
                "editorLayers": []
            }
        });
        let doc = MissionDocCore::new();
        doc.set_title("Preexisting");
        doc.hydrate(&payload.to_string(), "lyr");
        assert!(
            small_maps(&doc)["meta"].get("title").is_none(),
            "whitespace-only payload title must clear sticky meta.title, not keep Preexisting"
        );
        doc.apply_row_meta("  Row Title  ", "everon", None, None, None);
        assert_eq!(
            small_maps(&doc)["meta"]["title"],
            "Row Title",
            "apply_row_meta must trim and accept a real row title when payload title is blank"
        );
    }

    /// T-418 — row library blurb lands in `meta.briefing` so `compile_export` is not permanently "".
    #[test]
    fn t418_apply_row_meta_threads_briefing_into_meta() {
        let doc = MissionDocCore::new();
        doc.apply_row_meta(
            "Op",
            "everon",
            None,
            None,
            Some("  Hold the bridge.\nWait for extract.  ".into()),
        );
        assert_eq!(
            small_maps(&doc)["meta"]["briefing"],
            "Hold the bridge.\nWait for extract.",
            "non-blank row briefing must trim and land in meta.briefing"
        );
        // Whitespace-only is not authored — leave the key absent (compile_export → "").
        let empty = MissionDocCore::new();
        empty.apply_row_meta("Op", "everon", None, None, Some("   \n\t  ".into()));
        assert!(
            small_maps(&empty)["meta"].get("briefing").is_none(),
            "whitespace-only briefing must not invent meta.briefing"
        );
    }

    // ── T-220 — five silent hydrate→compile / edit / paste losses ───────────────────────────────

    /// Fixture payload that exercises every T-220 loss class at once.
    fn t220_lossy_payload() -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": 2,
            "map": {
                "terrain": "everon",
                "bounds": [100, 200, 300, 400],
                "center": [6400.5, 6400.25],
                "label": "ops-sector"
            },
            "environment": {},
            "editor": {
                "factions": [],
                "squads": [],
                // Non-alphabetical id order — without preserve_order, compile re-sorts to z-a, z-b, z-c.
                "slots": [
                    {
                        "id": "z-b", "squadId": "sq", "index": 0, "role": "Rifleman",
                        "stance": "stand",
                        "position": {
                            "x": 10.5, "y": 20.5, "z": 1.25, "rotation": 45.0,
                            "heading": 90.5, "source": "authored"
                        },
                        "customFlag": "keep-me",
                        "doctrineTag": "assault"
                    },
                    {
                        "id": "z-a", "squadId": "sq", "index": 1, "role": "Medic",
                        "stance": "stand",
                        "position": { "x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0 }
                    },
                    {
                        "id": "z-c", "squadId": "sq", "index": 2, "role": "SL",
                        "stance": "stand",
                        "position": { "x": 3.0, "y": 4.0, "z": 0.0, "rotation": 180.0 }
                    }
                ],
                "editorLayers": []
            }
        })
    }

    /// **T-220 Class R — hydrate→compile must not silently rewrite authored payload fields.**
    ///
    /// Covers loss classes 1–3 (schemaVersion, map.* / bounds, array order). Perturb by forcing
    /// `schemaVersion: 1`, dropping map extras, or sorting slot ids — each must fail this test.
    #[cfg(feature = "mission")]
    #[test]
    fn t220_hydrate_compile_preserves_schema_map_and_slot_order() {
        let incoming = t220_lossy_payload();
        let doc = MissionDocCore::new();
        doc.hydrate(&incoming.to_string(), "lyr");

        let compiled = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );

        assert_eq!(
            compiled["schemaVersion"],
            serde_json::json!(2),
            "authored schemaVersion must not downgrade to literal 1"
        );
        assert_eq!(
            compiled["map"]["bounds"],
            serde_json::json!([100, 200, 300, 400]),
            "authored map.bounds must not be recomputed"
        );
        assert_eq!(
            compiled["map"]["center"],
            serde_json::json!([6400.5, 6400.25]),
            "other map.* keys must survive"
        );
        assert_eq!(compiled["map"]["label"], serde_json::json!("ops-sector"));
        assert_eq!(compiled["map"]["terrain"], serde_json::json!("everon"));

        let ids: Vec<&str> = compiled["editor"]["slots"]
            .as_array()
            .expect("slots")
            .iter()
            .map(|s| s["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            vec!["z-b", "z-a", "z-c"],
            "editor.slots array order must follow hydrate insertion, not id-sort"
        );

        // Full Save→reload→Save still holds.
        let reloaded = save_and_reload(&doc);
        let again = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        assert_eq!(again["schemaVersion"], serde_json::json!(2));
        assert_eq!(
            again["map"]["bounds"],
            serde_json::json!([100, 200, 300, 400])
        );
        assert_eq!(again["map"]["center"], serde_json::json!([6400.5, 6400.25]));
    }

    /// **T-220 Class R — position sub-keys survive the first edit.**
    ///
    /// Perturb by restoring `position_any` to a four-key-only write — this fails.
    #[cfg(feature = "mission")]
    #[test]
    fn t220_position_subkeys_survive_first_edit() {
        let doc = MissionDocCore::new();
        doc.hydrate(&t220_lossy_payload().to_string(), "lyr");

        let before: serde_json::Value =
            serde_json::from_str(&doc.slots_json()).expect("slots json");
        assert_eq!(
            before["z-b"]["position"]["heading"],
            serde_json::json!(90.5)
        );
        assert_eq!(
            before["z-b"]["position"]["source"],
            serde_json::json!("authored")
        );

        doc.set_slot_position("z-b", 11.0, 21.0, 1.25, 45.0);
        let after: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots json");
        assert_eq!(after["z-b"]["position"]["x"].as_f64(), Some(11.0));
        assert_eq!(after["z-b"]["position"]["y"].as_f64(), Some(21.0));
        assert_eq!(
            after["z-b"]["position"]["heading"].as_f64(),
            Some(90.5),
            "unknown position sub-keys must survive set_slot_position"
        );
        assert_eq!(
            after["z-b"]["position"]["source"],
            serde_json::json!("authored")
        );

        doc.update_slot_position("z-b", Some(12.0), None, None, None, 12800.0, 12800.0);
        let after2: serde_json::Value =
            serde_json::from_str(&doc.slots_json()).expect("slots json");
        assert_eq!(after2["z-b"]["position"]["x"].as_f64(), Some(12.0));
        assert_eq!(
            after2["z-b"]["position"]["heading"].as_f64(),
            Some(90.5),
            "unknown position sub-keys must survive update_slot_position"
        );
    }

    /// **T-220 Class R — paste must carry unknown slot fields (and position sub-keys).**
    ///
    /// Perturb by dropping `extras_json` merge — this fails.
    #[cfg(feature = "mission")]
    #[test]
    fn t220_paste_preserves_unknown_slot_fields() {
        let doc = MissionDocCore::new();
        doc.add_editor_layer("lyr", "Default", None);
        let extras = serde_json::json!({
            "customFlag": "keep-me",
            "doctrineTag": "assault",
            "position": { "heading": 90.5, "source": "authored" }
        })
        .to_string();
        doc.paste_slots(
            vec!["p-new".into()],
            vec!["sq1".into()],
            vec!["lyr".into()],
            vec![10.0],
            vec![20.0],
            vec![45.0],
            vec![1.25],
            vec!["Rifleman".into()],
            vec![String::new()],
            vec![String::new()],
            vec!["stand".into()],
            vec![String::new()],
            vec![extras],
            Some(100.0),
            Some(200.0),
            12800.0,
            12800.0,
        );
        let v: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots json");
        assert_eq!(v["p-new"]["customFlag"], serde_json::json!("keep-me"));
        assert_eq!(v["p-new"]["doctrineTag"], serde_json::json!("assault"));
        // Known coords still come from the parallel arrays (anchor translate).
        assert_eq!(v["p-new"]["position"]["x"].as_f64(), Some(100.0));
        assert_eq!(v["p-new"]["position"]["y"].as_f64(), Some(200.0));
        assert_eq!(v["p-new"]["position"]["heading"].as_f64(), Some(90.5));
        assert_eq!(
            v["p-new"]["position"]["source"],
            serde_json::json!("authored")
        );
    }

    /// **T-215 — the round trip map placement is worthless without.**
    ///
    /// Before this ticket a vehicle's position was *derived* (leader ±30 m), so no test could tell a
    /// surviving position from a re-derived one. Place one at an authored map point instead, save,
    /// reload, and assert every component comes back bit-exact.
    ///
    /// The coordinates are deliberately **non-integral**: `value_to_any` encodes integral JSON
    /// numbers as `Any::BigInt` and the rest as `Any::Number`, so an integer fixture could pass
    /// while a real 2-decimal map click silently lost its fraction.
    #[cfg(feature = "mission")]
    #[test]
    fn map_placed_vehicle_position_round_trips_through_compile_and_hydrate() {
        let doc = orbat_fixture();
        doc.add_vehicle(
            "veh-map",
            "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et",
            Some(4870.25),
            Some(7760.5),
            Some(12.75),
            Some(137.5),
        );
        doc.set_vehicle_faction("veh-map", "faction-BLUFOR");

        let authored = vehicles_of(&doc)["veh-map"].clone();
        assert_eq!(authored["position"]["x"], serde_json::json!(4870.25));
        assert_eq!(authored["position"]["y"], serde_json::json!(7760.5));
        assert_eq!(authored["position"]["z"], serde_json::json!(12.75));
        assert_eq!(authored["position"]["rotation"], serde_json::json!(137.5));
        assert_eq!(authored["factionId"], serde_json::json!("faction-BLUFOR"));

        // A map placement must NOT join a squad — see `set_vehicle_faction` for why (T-321).
        assert!(
            authored.get("squadId").is_none(),
            "map placement must not attach to a squad: {authored}"
        );
        let vids = small_maps(&doc)["squadsById"]["sq-a"]
            .get("vehicleIds")
            .and_then(|v| v.as_array())
            .map_or(0, Vec::len);
        assert_eq!(vids, 0, "no squad may acquire a map-placed vehicle");

        let survived = vehicles_of(&save_and_reload(&doc))["veh-map"].clone();
        assert_eq!(
            survived, authored,
            "the vehicle row must survive save → reload whole"
        );
    }

    /// **T-215 — authored vehicle cargo survives the same round trip.**
    ///
    /// The emitted rows are `mission.schema.json` `$defs/entityInventory` verbatim — `{item, qty}`
    /// and nothing else. Asserting the exact row (not just its presence) is what keeps a stray
    /// `container` key, the character-side spelling, from creeping in: an entity's container is the
    /// entity, and that def is closed (`additionalProperties: false`).
    #[cfg(feature = "mission")]
    #[test]
    fn map_placed_vehicle_cargo_round_trips_as_entity_inventory_rows() {
        let doc = orbat_fixture();
        doc.add_vehicle(
            "veh-map",
            "{AAAA}Prefabs/Vehicles/T.et",
            Some(1.5),
            Some(2.5),
            Some(0.0),
            Some(0.0),
        );
        doc.set_vehicle_cargo(
            "veh-map",
            &[
                ("{BBBB}Prefabs/Weapons/M16.et".to_string(), 4),
                ("{CCCC}Prefabs/Items/Bandage.et".to_string(), 12),
            ],
        );

        let authored = vehicles_of(&doc)["veh-map"]["cargo"].clone();
        assert_eq!(
            authored,
            serde_json::json!([
                { "item": "{BBBB}Prefabs/Weapons/M16.et", "qty": 4 },
                { "item": "{CCCC}Prefabs/Items/Bandage.et", "qty": 12 },
            ]),
            "cargo rows must be $defs/entityInventory verbatim"
        );

        let survived = vehicles_of(&save_and_reload(&doc))["veh-map"]["cargo"].clone();
        assert_eq!(survived, authored, "cargo must survive save → reload whole");
    }

    /// T-215 — rows the schema cannot represent are dropped, and an empty result removes the key
    /// rather than writing `[]` (`$defs/entityInventory`: absent and `[]` mean the same thing).
    #[test]
    fn set_vehicle_cargo_drops_unrepresentable_rows_and_clears_on_empty() {
        let doc = orbat_fixture();
        doc.add_vehicle(
            "v",
            "{AAAA}P.et",
            Some(1.0),
            Some(1.0),
            Some(0.0),
            Some(0.0),
        );

        doc.set_vehicle_cargo(
            "v",
            &[
                ("   ".to_string(), 3),         // empty item — minLength: 1
                ("{BBBB}P.et".to_string(), 0),  // qty 0 — minimum: 1
                ("{CCCC}P.et".to_string(), -1), // negative qty
                ("{DDDD}P.et".to_string(), 1),  // the only representable row
            ],
        );
        assert_eq!(
            vehicles_of(&doc)["v"]["cargo"],
            serde_json::json!([{ "item": "{DDDD}P.et", "qty": 1 }]),
        );

        doc.set_vehicle_cargo("v", &[]);
        assert!(
            vehicles_of(&doc)["v"].get("cargo").is_none(),
            "an empty result must REMOVE the key, not write []"
        );
    }

    /// B7 — dense index rewrite 0..n-1 after move.
    #[test]
    fn slot_indices_dense_after_move() {
        let doc = orbat_fixture();
        doc.add_slot(
            "a0", "sq-a", "lyr", 0, "Rifleman", None, None, 1.0, 1.0, 0.0, 0.0,
        );
        doc.add_slot(
            "a1", "sq-a", "lyr", 1, "Rifleman", None, None, 2.0, 2.0, 0.0, 0.0,
        );
        doc.add_slot(
            "a2", "sq-a", "lyr", 2, "Rifleman", None, None, 3.0, 3.0, 0.0, 0.0,
        );
        doc.set_leader("sq-a", "a0");
        doc.add_slot(
            "b0", "sq-b", "lyr", 0, "Rifleman", None, None, 4.0, 4.0, 0.0, 0.0,
        );
        doc.set_leader("sq-b", "b0");
        // Move middle member out of sq-a → remaining must reindex densely.
        doc.move_slot_to_squad("a1", "sq-b");
        let slots = slots_map(&doc);
        let root = small_maps(&doc);
        for (sq_id, key) in [("sq-a", "sq-a"), ("sq-b", "sq-b")] {
            let ids = root["squadsById"][key]["slotIds"]
                .as_array()
                .unwrap_or_else(|| panic!("{sq_id} slotIds"));
            for (i, id_val) in ids.iter().enumerate() {
                let sid = id_val.as_str().expect("id str");
                let idx = slots[sid]["index"].as_i64().expect("index");
                assert_eq!(idx, i as i64, "{sq_id}/{sid} index");
            }
        }
    }

    // ── T-345 per-faction briefing markers ───────────────────────────────────────────────────────

    /// A two-faction doc with one slot, so `flatten_to_mod_document` has something to compile (it
    /// answers `NoSlots` on an empty graph) and so the tests below can prove a marker lands on ONE
    /// side and not the other.
    ///
    /// Seeded under `INIT` like `seeded_core`, so the undo stack starts EMPTY and the first marker
    /// edit is the first tracked step — otherwise `can_undo()` is already true from the fixture and
    /// says nothing about the mutator.
    fn briefing_fixture() -> MissionDocCore {
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_faction("faction-BLUFOR", "BLUFOR", "US Army");
        doc.add_faction("faction-OPFOR", "OPFOR", "Soviet VDV");
        doc.add_squad("sq-a", "faction-BLUFOR", "1st", Some("Alpha".to_string()));
        doc.add_editor_layer("lyr", "Default Layer", None);
        doc.add_slot(
            "z1", "sq-a", "lyr", 0, "SL", None, None, 4839.2, 6620.8, 0.0, 270.0,
        );
        doc.set_origin_init(false);
        assert!(!doc.can_undo(), "the INIT seed must not be an undo step");
        doc
    }

    /// The marker rows on one faction, straight out of `small_maps_json`.
    fn markers_of(doc: &MissionDocCore, faction_id: &str) -> Vec<serde_json::Value> {
        small_maps(doc)["factionsById"][faction_id]["briefing"]["markers"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// A marker field as an f64. Not `assert_eq!(row["x"], json!(300.0))` — yrs writes
    /// `Any::Number(300.0)` as the JSON token `300`, which `serde_json` then reads back as an
    /// INTEGER `Number`, so `Value`-equality against `300.0` fails on a value that is entirely
    /// correct. The wire is unaffected (`MarkerIn::x` is an `f64` and parses either token), and the
    /// mixed encoding is documented on [`marker_any`] — but a test must compare numbers as numbers.
    fn marker_num(row: &serde_json::Value, key: &str) -> f64 {
        row[key]
            .as_f64()
            .unwrap_or_else(|| panic!("{key} is a number: {row:?}"))
    }

    /// **T-345 — the round trip that was impossible before this mutator existed.**
    ///
    /// Author a marker → `compile_payload` → `hydrate` → recompile, and assert the coordinates come
    /// back bit-exact. T-214 proved prose round-trips by using `hydrate` as the WRITER; this uses the
    /// real mutator, which is the half that did not exist and the half a frontend will call.
    ///
    /// The coordinates are deliberately non-integral: `value_to_any` encodes integral JSON numbers as
    /// `Any::BigInt` and the rest as `Any::Number`, so an integral fixture could pass while a real
    /// 3-decimal map click silently lost its fraction.
    #[test]
    fn authored_marker_round_trips_through_compile_and_hydrate() {
        let doc = briefing_fixture();
        doc.set_faction_briefing_marker(
            "faction-BLUFOR",
            "mk-1",
            4870.25,
            7760.5,
            "objective",
            "Seize the bridge",
        );

        let authored = markers_of(&doc, "faction-BLUFOR");
        assert_eq!(authored.len(), 1, "one authored marker: {authored:?}");
        assert_eq!(authored[0]["x"], serde_json::json!(4870.25));
        assert_eq!(authored[0]["z"], serde_json::json!(7760.5));
        assert_eq!(authored[0]["icon"], serde_json::json!("objective"));
        assert_eq!(authored[0]["label"], serde_json::json!("Seize the bridge"));
        assert_eq!(authored[0]["id"], serde_json::json!("mk-1"));

        // The marker is SIDE-SCOPED: the other faction authored nothing and must stay unauthored,
        // or `derive_briefings` would ship orders to a side that was never given any.
        assert!(
            small_maps(&doc)["factionsById"]["faction-OPFOR"]
                .get("briefing")
                .is_none(),
            "the unauthored side must not acquire a briefing"
        );

        // Save → reload, exactly as the editor does it.
        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&payload.to_string(), "lyr");

        let survived = markers_of(&reloaded, "faction-BLUFOR");
        assert_eq!(survived, authored, "marker rows must survive hydrate whole");

        // And the recompile is stable — author → store → compile → reload → compile agrees.
        let recompiled = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        assert_eq!(
            recompiled["editor"]["factions"][0]["briefing"]["markers"],
            payload["editor"]["factions"][0]["briefing"]["markers"]
        );
        // Proof this was a real round trip and not two empty docs agreeing with each other.
        assert_eq!(
            recompiled["editor"]["slots"]
                .as_array()
                .expect("slots")
                .len(),
            1
        );
    }

    /// **T-345 — the authored marker reaches the compiled mod document, and its doc `id` does not.**
    ///
    /// This is the assertion the whole design rests on: the doc row carries `id`, `$defs/marker` is
    /// `additionalProperties: false`, and NO emitter change strips it — `MarkerIn` ignores the unknown
    /// key and `ModMarker` re-emits only the four schema fields. If a future slice ever adds
    /// `deny_unknown_fields` to `MarkerIn`, or teaches `ModMarker` to pass rows through verbatim, this
    /// fails here naming the reason instead of shipping a document the game server rejects.
    ///
    /// `#[cfg(feature = "mission")]` — the reason the suite must run `--features doc,mission`;
    /// `--features doc` alone compiles the compiler out and silently skips this.
    #[cfg(feature = "mission")]
    #[test]
    fn authored_marker_reaches_the_mod_document_without_its_doc_id() {
        let doc = briefing_fixture();
        doc.set_faction_briefing_marker(
            "faction-BLUFOR",
            "mk-1",
            4870.25,
            7760.5,
            "objective",
            "Seize the bridge",
        );

        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let meta = crate::mission::flatten::MissionMeta {
            id: "4c7e1b08-9a35-4d62-b1f7-e30d5a86c941".into(),
            title: "Bridgehead at Levie".into(),
            author: "184472930165846017".into(),
            terrain: "everon".into(),
            custom_terrain_name: String::new(),
            max_players: 12,
            time_of_day: "06:15".into(),
            weather_preset: "overcast".into(),
        };
        let compiled = crate::mission::flatten::flatten_to_mod_document(
            &meta,
            &serde_json::to_vec(&payload).expect("payload serialises"),
        )
        .expect("the fixture has a slot, so the compile must succeed");
        let doc_json = serde_json::to_value(&compiled).expect("mod document serialises");

        // Keyed by `slug_key(faction.key)` — `BLUFOR` → `blufor`, the same slug `orbat` and
        // `slots[].faction` use, which is what `GetBriefingForFaction(slot.faction)` looks up.
        let rows = doc_json["briefings"]["blufor"]["markers"]
            .as_array()
            .expect("blufor briefing carries markers");
        assert_eq!(rows.len(), 1, "{rows:?}");

        let m = rows[0].as_object().expect("marker is an object");
        assert_eq!(m["x"], serde_json::json!(4870.25));
        assert_eq!(m["z"], serde_json::json!(7760.5));
        assert_eq!(m["icon"], serde_json::json!("objective"));
        assert_eq!(m["label"], serde_json::json!("Seize the bridge"));

        // The whole point: `$defs/marker` is `additionalProperties: false`, so the doc-internal id
        // must NOT be on the wire — and the emitter drops it for free.
        assert!(
            m.get("id").is_none(),
            "the doc-internal marker id must not reach the compiled document: {m:?}"
        );
        assert_eq!(
            m.len(),
            4,
            "exactly the four `$defs/marker` fields, no more: {m:?}"
        );

        // The unauthored side gets no entry at all, not an empty one.
        assert!(
            doc_json["briefings"].get("opfor").is_none(),
            "{:?}",
            doc_json["briefings"]
        );
    }

    /// **T-345 — the doc `id` is what makes the marker addressable**, which is the only thing it is
    /// there for. Re-setting the same id MOVES that marker in place (a drag) instead of appending a
    /// duplicate, and does not reorder the list — the mod renders in array order.
    #[test]
    fn setting_the_same_marker_id_moves_it_in_place() {
        let doc = briefing_fixture();
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 100.0, 200.0, "objective", "OBJ");
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-2", 300.0, 400.0, "hazard", "MINES");
        // The drag: same id, new coordinates and caption.
        doc.set_faction_briefing_marker(
            "faction-BLUFOR",
            "mk-1",
            111.5,
            222.5,
            "rally",
            "Rally point",
        );

        let rows = markers_of(&doc, "faction-BLUFOR");
        assert_eq!(rows.len(), 2, "an upsert must not duplicate: {rows:?}");
        // Order preserved — `mk-1` is still first, so the drag did not shuffle the list.
        assert_eq!(rows[0]["id"], serde_json::json!("mk-1"));
        assert_eq!(marker_num(&rows[0], "x"), 111.5);
        assert_eq!(marker_num(&rows[0], "z"), 222.5);
        assert_eq!(rows[0]["icon"], serde_json::json!("rally"));
        assert_eq!(rows[0]["label"], serde_json::json!("Rally point"));
        assert_eq!(rows[1]["id"], serde_json::json!("mk-2"));
        assert_eq!(marker_num(&rows[1], "x"), 300.0);
    }

    /// **T-345 — remove is by id and touches nothing else.** An index would have been enough for a
    /// test and wrong in the editor: deleting `mk-1` renumbers `mk-2`, so a queued delete would take
    /// the survivor.
    #[test]
    fn removing_a_marker_by_id_leaves_its_siblings_alone() {
        let doc = briefing_fixture();
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 100.0, 200.0, "objective", "OBJ");
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-2", 300.0, 400.0, "hazard", "MINES");
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-3", 500.0, 600.0, "rally", "RP");

        doc.remove_faction_briefing_marker("faction-BLUFOR", "mk-2");

        let rows = markers_of(&doc, "faction-BLUFOR");
        let ids: Vec<&str> = rows
            .iter()
            .map(|m| m["id"].as_str().unwrap_or(""))
            .collect();
        assert_eq!(ids, vec!["mk-1", "mk-3"], "{rows:?}");
        assert_eq!(marker_num(&rows[1], "x"), 500.0, "survivor intact");

        // Emptying the list is legal and stays legal: the emitter omits an empty `markers` exactly
        // as it omits an absent one (`skip_serializing_if = "Vec::is_empty"`).
        doc.remove_faction_briefing_marker("faction-BLUFOR", "mk-1");
        doc.remove_faction_briefing_marker("faction-BLUFOR", "mk-3");
        assert!(markers_of(&doc, "faction-BLUFOR").is_empty());
        assert!(
            small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"]["markers"].is_array(),
            "an emptied list stays an array, not a dropped key"
        );
    }

    /// **T-345 — a delete that deletes nothing must not change the compiled document.** Writing
    /// `briefing: {markers: []}` onto an unauthored faction would flip `FactionIn::briefing` from
    /// `None` to `Some` and make `derive_briefings` emit a `briefings` entry for a side that authored
    /// nothing — a compiled-output change produced by a no-op, and a needless undo step.
    #[test]
    fn removing_from_an_unauthored_faction_does_not_mint_a_briefing() {
        let doc = briefing_fixture();
        let before = small_maps(&doc);
        doc.remove_faction_briefing_marker("faction-OPFOR", "mk-1");
        doc.remove_faction_briefing_marker("faction-does-not-exist", "mk-1");

        assert!(
            small_maps(&doc)["factionsById"]["faction-OPFOR"]
                .get("briefing")
                .is_none(),
            "a no-op delete must not author a briefing"
        );
        // Nothing anywhere moved, so the compiled bytes cannot have changed either.
        assert_eq!(
            small_maps(&doc),
            before,
            "a no-op delete must write nothing"
        );
        assert!(
            !doc.can_undo(),
            "a no-op delete must not stack an undo step"
        );
    }

    /// **T-345 — markers coexist with prose on one `briefing` object**, which is the shape T-344's
    /// `set_faction_briefing` has to preserve. The prose here is planted through `hydrate` (T-214's
    /// proven writer) precisely because the prose mutator does not exist yet: this pins that a marker
    /// edit reads the existing briefing and writes it back WHOLE rather than replacing it, so the
    /// mutator that lands next cannot silently delete the other half.
    #[test]
    fn a_marker_edit_preserves_authored_prose_on_the_same_briefing() {
        let payload = serde_json::json!({
            "schemaVersion": 1,
            "map": { "terrain": "everon" },
            "editor": {
                "factions": [{
                    "id": "faction-BLUFOR", "key": "BLUFOR", "name": "US Army",
                    "squadIds": ["sq-a"],
                    "briefing": {
                        "situation": "Enemy armour holds the east bank.\n\nBridge is intact.",
                        "mission": "Seize and hold the crossing.",
                        "execution": "Alpha leads.",
                    }
                }],
                "squads": [{ "id": "sq-a", "factionId": "faction-BLUFOR", "name": "1st",
                             "slotIds": ["z1"] }],
                "slots": [{ "id": "z1", "squadId": "sq-a", "index": 0, "role": "SL",
                            "position": { "x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0 } }],
                "editorLayers": []
            }
        })
        .to_string();

        let doc = MissionDocCore::new();
        doc.hydrate(&payload, "lyr");
        doc.set_faction_briefing_marker(
            "faction-BLUFOR",
            "mk-1",
            4870.25,
            7760.5,
            "objective",
            "OBJ",
        );

        let briefing = &small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"];
        assert_eq!(
            briefing["situation"],
            serde_json::json!("Enemy armour holds the east bank.\n\nBridge is intact."),
            "the paragraph break must survive a marker edit verbatim"
        );
        assert_eq!(
            briefing["mission"],
            serde_json::json!("Seize and hold the crossing.")
        );
        assert_eq!(briefing["execution"], serde_json::json!("Alpha leads."));
        assert_eq!(briefing["markers"].as_array().expect("markers").len(), 1);

        // …and the reverse direction: removing the marker must not take the prose with it.
        doc.remove_faction_briefing_marker("faction-BLUFOR", "mk-1");
        let after = &small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"];
        assert_eq!(
            after["mission"],
            serde_json::json!("Seize and hold the crossing.")
        );
        assert!(after["markers"].as_array().expect("markers").is_empty());
    }

    // ── T-344 per-faction briefing prose ─────────────────────────────────────────────────────────

    /// A multi-paragraph situation report — the value the whole no-sanitising rule exists for.
    /// Two blank-line paragraph breaks AND a single-newline line break inside one paragraph, because
    /// the mod treats them differently (`SplitLines` drops blank parts) and a sanitiser that only ate
    /// doubled newlines would still corrupt the second case.
    const SITUATION: &str = "Enemy armour holds the east bank of the Levie crossing.\n\n\
                             Two T-72s were observed at 04:30, dug in north of the treeline.\n\
                             A third is unaccounted for.\n\n\
                             Civilians remain in the village. Weapons tight until contact.";

    /// One faction's briefing prose out of `small_maps_json`, or `Value::Null` when unauthored.
    fn prose_of(doc: &MissionDocCore, faction_id: &str, key: &str) -> serde_json::Value {
        small_maps(doc)["factionsById"][faction_id]["briefing"][key].clone()
    }

    /// **T-344 — the round trip that was impossible before this mutator existed.**
    ///
    /// Author prose with the REAL mutator → `compile_payload` → `hydrate` → recompile, and assert
    /// every paragraph break survives byte-for-byte. T-214 proved the document round-trips prose using
    /// `hydrate` as the writer; this is the other half — the writer a frontend actually calls.
    ///
    /// The multi-paragraph value is the point, not decoration. `$defs/wireSafeString` excludes briefing
    /// prose from the control-character ban because `TBD_BriefingService` ships it as parallel
    /// `array<string>` RPC parameters, so `\n` is authored content. If anything ever "tidies" newlines
    /// to spaces, this fails here.
    #[test]
    fn authored_prose_round_trips_through_compile_and_hydrate() {
        let doc = briefing_fixture();
        doc.set_faction_briefing(
            "faction-BLUFOR",
            SITUATION,
            "Seize and hold the crossing until relieved.",
            "Alpha leads, Bravo screens the north flank.",
        );

        assert_eq!(
            prose_of(&doc, "faction-BLUFOR", "situation"),
            serde_json::json!(SITUATION),
            "the authored prose must be stored verbatim"
        );
        assert!(
            prose_of(&doc, "faction-BLUFOR", "situation")
                .as_str()
                .expect("situation is a string")
                .contains("\n\n"),
            "the fixture must actually carry a paragraph break, or this test proves nothing"
        );

        // Prose is SIDE-SCOPED: the other faction authored nothing and must stay unauthored.
        assert!(
            small_maps(&doc)["factionsById"]["faction-OPFOR"]
                .get("briefing")
                .is_none(),
            "the unauthored side must not acquire a briefing"
        );

        // Save → reload, exactly as the editor does it.
        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&payload.to_string(), "lyr");

        assert_eq!(
            prose_of(&reloaded, "faction-BLUFOR", "situation"),
            serde_json::json!(SITUATION),
            "every newline must survive hydrate verbatim"
        );
        assert_eq!(
            prose_of(&reloaded, "faction-BLUFOR", "mission"),
            serde_json::json!("Seize and hold the crossing until relieved.")
        );
        assert_eq!(
            prose_of(&reloaded, "faction-BLUFOR", "execution"),
            serde_json::json!("Alpha leads, Bravo screens the north flank.")
        );

        // And the recompile is stable — author → store → compile → reload → compile agrees.
        let recompiled = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        assert_eq!(
            recompiled["editor"]["factions"][0]["briefing"],
            payload["editor"]["factions"][0]["briefing"]
        );
        // Proof this was a real round trip and not two empty docs agreeing with each other.
        assert_eq!(
            recompiled["editor"]["slots"]
                .as_array()
                .expect("slots")
                .len(),
            1
        );
    }

    /// **T-344 — prose and markers must not eat each other, in BOTH directions.**
    ///
    /// `briefing` is one opaque `Any::Map`, so either mutator writing a fresh object instead of
    /// read-modify-writing the existing one silently deletes the other half. T-345 pinned its side
    /// using `hydrate` to plant the prose, because this mutator did not exist yet; now that it does,
    /// both halves are pinned with their REAL writers and the loop is closed.
    #[test]
    fn prose_and_markers_do_not_eat_each_other() {
        // Direction 1: markers first, then a prose edit must preserve them.
        let doc = briefing_fixture();
        doc.set_faction_briefing_marker(
            "faction-BLUFOR",
            "mk-1",
            4870.25,
            7760.5,
            "objective",
            "OBJ",
        );
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-2", 300.5, 400.5, "hazard", "MINES");

        doc.set_faction_briefing("faction-BLUFOR", SITUATION, "Hold.", "Alpha leads.");

        let rows = markers_of(&doc, "faction-BLUFOR");
        let ids: Vec<&str> = rows
            .iter()
            .map(|m| m["id"].as_str().unwrap_or(""))
            .collect();
        assert_eq!(
            ids,
            vec!["mk-1", "mk-2"],
            "a prose edit must not delete markers: {rows:?}"
        );
        assert_eq!(marker_num(&rows[0], "x"), 4870.25, "marker payload intact");
        assert_eq!(
            prose_of(&doc, "faction-BLUFOR", "situation"),
            serde_json::json!(SITUATION)
        );

        // Direction 2: prose first, then a marker edit must preserve it — including the paragraph
        // breaks, which is the part a naive re-serialise would quietly flatten.
        let doc2 = briefing_fixture();
        doc2.set_faction_briefing("faction-BLUFOR", SITUATION, "Hold.", "Alpha leads.");
        doc2.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 111.5, 222.5, "rally", "RP");
        doc2.remove_faction_briefing_marker("faction-BLUFOR", "mk-1");

        assert_eq!(
            prose_of(&doc2, "faction-BLUFOR", "situation"),
            serde_json::json!(SITUATION),
            "a marker add+remove must not touch the prose"
        );
        assert_eq!(
            prose_of(&doc2, "faction-BLUFOR", "mission"),
            serde_json::json!("Hold.")
        );
        assert_eq!(
            prose_of(&doc2, "faction-BLUFOR", "execution"),
            serde_json::json!("Alpha leads.")
        );
    }

    /// **T-344 — a write that writes nothing must not change the compiled document.** T-345's
    /// `removing_from_an_unauthored_faction_does_not_mint_a_briefing` from the other side: setting
    /// all-empty prose on an unauthored faction would write `briefing: {}`, flip `FactionIn::briefing`
    /// from `None` to `Some`, and make `derive_briefings` emit a `briefings` entry for a side that
    /// authored nothing — plus stack an empty undo step.
    #[test]
    fn setting_all_empty_prose_on_an_unauthored_faction_does_not_mint_a_briefing() {
        let doc = briefing_fixture();
        let before = small_maps(&doc);

        doc.set_faction_briefing("faction-OPFOR", "", "", "");
        doc.set_faction_briefing("faction-does-not-exist", "", "", "");
        // An unknown faction with REAL prose is still a no-op — orders need a side to be given to.
        doc.set_faction_briefing("faction-does-not-exist", SITUATION, "Hold.", "Go.");

        assert!(
            small_maps(&doc)["factionsById"]["faction-OPFOR"]
                .get("briefing")
                .is_none(),
            "an all-empty set must not author a briefing"
        );
        // Nothing anywhere moved, so the compiled bytes cannot have changed either.
        assert_eq!(
            small_maps(&doc),
            before,
            "a no-op prose write must write nothing"
        );
        assert!(
            !doc.can_undo(),
            "a no-op prose write must not stack an undo step"
        );

        // Re-setting IDENTICAL prose is also a no-op — no second undo step for a non-edit.
        doc.set_faction_briefing("faction-BLUFOR", SITUATION, "Hold.", "Go.");
        assert_eq!(doc.undo_depth(), 1, "the real edit is one step");
        doc.set_faction_briefing("faction-BLUFOR", SITUATION, "Hold.", "Go.");
        assert_eq!(
            doc.undo_depth(),
            1,
            "re-setting identical prose must not stack a second step"
        );
    }

    /// **T-344 — clearing a box returns that field to UNAUTHORED, and clearing every box on a briefing
    /// that has markers is still a real edit.**
    ///
    /// `""` removes the key rather than writing `Some("")`, because a three-textbox editor cannot
    /// distinguish "never filled in" from "deliberately blanked" and the emitter's `Option<String>`
    /// carries that distinction into the compiled bytes. The second half is why the no-op guard
    /// compares the MAP and not merely "is the new prose empty": the briefing legitimately exists
    /// here, so the clear must land.
    #[test]
    fn clearing_a_prose_field_returns_it_to_unauthored() {
        let doc = briefing_fixture();
        doc.set_faction_briefing(
            "faction-BLUFOR",
            SITUATION,
            "Hold the crossing.",
            "Alpha leads.",
        );

        // Clear only `execution`; the siblings stay.
        doc.set_faction_briefing("faction-BLUFOR", SITUATION, "Hold the crossing.", "");
        let briefing = &small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"];
        assert!(
            briefing.get("execution").is_none(),
            "a cleared field is removed, not blanked to \"\": {briefing:?}"
        );
        assert_eq!(briefing["situation"], serde_json::json!(SITUATION));
        assert_eq!(briefing["mission"], serde_json::json!("Hold the crossing."));

        // Now clear everything on a briefing that also carries a marker: the prose keys go, the
        // marker stays, and the briefing survives because the marker keeps it legitimate.
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 111.5, 222.5, "rally", "RP");
        doc.set_faction_briefing("faction-BLUFOR", "", "", "");

        let briefing = &small_maps(&doc)["factionsById"]["faction-BLUFOR"]["briefing"];
        assert!(
            briefing.get("situation").is_none() && briefing.get("mission").is_none(),
            "clearing every box must land when the briefing really exists: {briefing:?}"
        );
        assert_eq!(
            markers_of(&doc, "faction-BLUFOR").len(),
            1,
            "clearing the prose must not take the marker with it"
        );
    }

    /// **T-344 — authored prose reaches the compiled mod document, keyed by faction slug, newlines
    /// intact.** This is the end-to-end claim: prose typed through this mutator arrives where
    /// `TBD_BriefingService.GetBriefingForFaction(slot.faction)` looks for it.
    ///
    /// `#[cfg(feature = "mission")]` — the reason the suite must run `--features doc,mission`;
    /// `--features doc` alone compiles the compiler out and silently skips this.
    #[cfg(feature = "mission")]
    #[test]
    fn authored_prose_reaches_the_mod_document_keyed_by_faction_slug() {
        let doc = briefing_fixture();
        doc.set_faction_briefing(
            "faction-BLUFOR",
            SITUATION,
            "Seize and hold the crossing until relieved.",
            "Alpha leads, Bravo screens the north flank.",
        );

        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let meta = crate::mission::flatten::MissionMeta {
            id: "4c7e1b08-9a35-4d62-b1f7-e30d5a86c941".into(),
            title: "Bridgehead at Levie".into(),
            author: "184472930165846017".into(),
            terrain: "everon".into(),
            custom_terrain_name: String::new(),
            max_players: 12,
            time_of_day: "06:15".into(),
            weather_preset: "overcast".into(),
        };
        let compiled = crate::mission::flatten::flatten_to_mod_document(
            &meta,
            &serde_json::to_vec(&payload).expect("payload serialises"),
        )
        .expect("the fixture has a slot, so the compile must succeed");
        let doc_json = serde_json::to_value(&compiled).expect("mod document serialises");

        // Keyed by `slug_key(faction.key)` — `BLUFOR` → `blufor`, the same slug `slots[].faction`
        // carries, which is what `GetBriefingForFaction(slot.faction)` looks up.
        let b = doc_json["briefings"]["blufor"]
            .as_object()
            .expect("blufor briefing");
        assert_eq!(
            b["situation"],
            serde_json::json!(SITUATION),
            "the paragraph breaks must reach the compiled document verbatim"
        );
        assert_eq!(
            b["mission"],
            serde_json::json!("Seize and hold the crossing until relieved.")
        );
        assert_eq!(
            b["execution"],
            serde_json::json!("Alpha leads, Bravo screens the north flank.")
        );
        // No `markers` key: an empty list is omitted exactly as an absent one is, so the three prose
        // fields are the whole entry.
        assert_eq!(b.len(), 3, "exactly the three authored fields: {b:?}");

        // The unauthored side gets no entry at all, not an empty one.
        assert!(
            doc_json["briefings"].get("opfor").is_none(),
            "{:?}",
            doc_json["briefings"]
        );
    }
}
