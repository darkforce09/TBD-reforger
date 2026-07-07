//! `MissionDocCore` — owns a `yrs` document + its tracked root maps. It applies Yjs-wire update
//! byte-streams (criterion 2), encodes/decodes the update stream (criterion 3), materializes the slot
//! SoA (criterion 1), and drives undo/redo (criterion 4). The write mutators (`add_slot` /
//! `set_slot_position` / `remove_slot`) exist to exercise the `UndoManager`; the full `state/ydoc.ts`
//! mutator surface is ported at the 3.1 cutover, not in the spike.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use yrs::sync::{Clock, Timestamp};
use yrs::types::ToJson;
use yrs::undo::{Options as UndoOptions, UndoManager};
use yrs::updates::decoder::Decode;
use yrs::{
    Any, Doc, Map, MapPrelim, MapRef, Out, ReadTxn, StateVector, Transact, TransactionMut, Update,
};

use super::soa::{Interner, NONE_IDX, STANCE_CROUCH, STANCE_PRONE, STANCE_STAND, SlotSoa};

/// Fixed, deterministic client id — so `encode_state` and the undo/redo sequence are reproducible
/// (parity for criteria 3/4). A client-id clash with an incoming peer update is harmless: `yrs`
/// keys blocks by the *originating* client, and the spike doc never co-authors a slot with a peer.
const CLIENT_ID: u64 = 1;

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

        // capture_timeout_millis = 0 → every transaction is its own undo step. yrs extends the last
        // stack item only when `now - last_change < capture_timeout_millis` (undo.rs), and `u64 < 0`
        // is never true — so no same-millisecond merge. This matches driving the JS `Y.UndoManager`
        // with `{ captureTimeout: 0 }` (Yjs uses the same `<`), the basis for criterion-4 parity.
        let opts = UndoOptions::<()> {
            capture_timeout_millis: 0,
            tracked_origins: HashSet::new(), // empty → capture no-origin (local) transactions
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

        Self {
            doc,
            slots,
            squads,
            factions,
            editor_layers,
            meta,
            undo_mgr,
        }
    }

    /// Apply a Yjs-wire (v1) update byte-stream — the exact bytes `Y.encodeStateAsUpdate(doc)` emits.
    ///
    /// # Errors
    /// Returns a message on a malformed update or an integration failure.
    pub fn apply_update(&self, bytes: &[u8]) -> Result<(), String> {
        let update = Update::decode_v1(bytes).map_err(|e| e.to_string())?;
        // A no-origin transaction — undo-tracked in the spike (never interleaved with undo in the
        // tests). The cutover will apply remote updates under an untracked `remote` origin.
        let mut txn = self.doc.transact_mut();
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
    #[must_use]
    pub fn small_maps_json(&self) -> String {
        // Grab the root handles before opening the read txn (`get_or_insert_map` takes `&self`).
        let meta = self.doc.get_or_insert_map("meta");
        let named: [(&str, MapRef); 8] = [
            ("factionsById", self.doc.get_or_insert_map("factions")),
            ("squadsById", self.doc.get_or_insert_map("squads")),
            ("loadoutsById", self.doc.get_or_insert_map("loadouts")),
            ("itemsById", self.doc.get_or_insert_map("items")),
            ("objectivesById", self.doc.get_or_insert_map("objectives")),
            ("vehiclesById", self.doc.get_or_insert_map("vehicles")),
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
            soa.squad_idx
                .push(squads.intern(read_str(&txn, &slot, "squadId").as_deref().unwrap_or("")));
            soa.layer_idx.push(match slot_layer.get(id) {
                Some(l) => layers.intern(l),
                None => NONE_IDX,
            });
        }

        soa.roles = roles.words;
        soa.tags = tags.words;
        soa.squads = squads.words;
        soa.layers = layers.words;
        soa
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
        let mut txn = self.doc.transact_mut();
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
        let mut txn = self.doc.transact_mut();
        let f = self
            .factions
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        f.insert(&mut txn, "key", key);
        f.insert(&mut txn, "name", name);
        f.insert(&mut txn, "squadIds", Any::Array(Vec::new().into()));
    }

    /// Create a squad under a faction (mirrors `ydoc.addSquad` and `ensureDefaultSquad`'s squad).
    /// Writes `{id, factionId, name, slotIds:[]}` + `callsign` only when `Some`; appends `id` to
    /// `faction.squadIds` if the faction exists.
    pub fn add_squad(&self, id: &str, faction_id: &str, name: &str, callsign: Option<String>) {
        let mut txn = self.doc.transact_mut();
        let sq = self
            .squads
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        sq.insert(&mut txn, "factionId", faction_id);
        if let Some(c) = callsign {
            sq.insert(&mut txn, "callsign", c);
        }
        sq.insert(&mut txn, "name", name);
        sq.insert(&mut txn, "slotIds", Any::Array(Vec::new().into()));
        append_id(&mut txn, &self.factions, faction_id, "squadIds", id);
    }

    /// Overwrite a slot's `position` (mirrors `slot.set('position', {...})`).
    pub fn set_slot_position(&self, id: &str, x: f64, y: f64, z: f64, rotation: f64) {
        let mut txn = self.doc.transact_mut();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            slot.insert(&mut txn, "position", position_any(x, y, z, rotation));
        }
    }

    /// Remove one slot (mirrors `slots.delete(id)`; layer detach is out of the spike mutator set).
    pub fn remove_slot(&self, id: &str) {
        let mut txn = self.doc.transact_mut();
        self.slots.remove(&mut txn, id);
    }

    /// Bulk-seed `n` random slots in ONE transaction — the browser-harness generator for the
    /// criterion-6 fps/zero-copy test. Deterministic LCG positions in `[0,w)×[0,h)`; not
    /// undo-granular (the whole seed is one step).
    pub fn seed_random(&self, n: u32, w: f64, h: f64, seed: u64) {
        let mut s = seed | 1;
        let mut txn = self.doc.transact_mut();
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
        let mut txn = self.doc.transact_mut();
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
        let mut txn = self.doc.transact_mut();
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
            slot.insert(&mut txn, "position", position_any(px, py, pz, prot));
        }
    }

    /// Move several slots by a shared world delta (drag release). z re-sampled to 0 (DEM JS-side).
    /// Mirrors `ydoc.moveEntities`.
    pub fn move_entities(&self, ids: Vec<String>, dx: f64, dy: f64) {
        let mut txn = self.doc.transact_mut();
        for id in &ids {
            if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
                let (px, py, _pz, prot) = read_position(&txn, &slot);
                slot.insert(
                    &mut txn,
                    "position",
                    position_any(px + dx, py + dy, 0.0, prot),
                );
            }
        }
    }

    /// Remove several slots and detach them from their squad's `slotIds` and every layer's
    /// `entityIds` (batched cascade). Mirrors `ydoc.removeEntities` (slots path). The cascade body
    /// lives in [`remove_slots_in_txn`] so `remove_editor_layer` can reuse it inside its own txn.
    pub fn remove_slots(&self, ids: Vec<String>) {
        let mut txn = self.doc.transact_mut();
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
    /// clamp to `[0,width]×[0,height]`; z is 0 (terrain-follow re-sampled JS-side); rotation carries
    /// from the source. `index` accumulates per squad (seeded from the squad's current `slotIds`).
    /// `""` tag/asset → key omitted. Appends are batched (each squad's `slotIds` / each layer's
    /// `entityIds` written once) — the T-059 O(k) shape.
    #[allow(clippy::too_many_arguments)]
    pub fn paste_slots(
        &self,
        ids: Vec<String>,
        squad_ids: Vec<String>,
        layer_ids: Vec<String>,
        src_x: Vec<f64>,
        src_y: Vec<f64>,
        src_rot: Vec<f64>,
        roles: Vec<String>,
        tags: Vec<String>,
        asset_ids: Vec<String>,
        stances: Vec<String>,
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

        let mut txn = self.doc.transact_mut();
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
            slot.insert(&mut txn, "position", position_any(px, py, 0.0, src_rot[i]));
            slot.insert(&mut txn, "stance", stances[i].as_str());
            slot.insert(&mut txn, "loadoutId", Any::Null);
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
        let mut txn = self.doc.transact_mut();
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
        let mut txn = self.doc.transact_mut();
        self.meta.insert(&mut txn, "title", title);
    }

    /// Merge an environment patch (a JSON object) onto the existing `meta.environment`, mirroring
    /// `ydoc.updateEnvironment` (`{...env, ...patch}`). Absent env → the patch becomes the env.
    pub fn update_environment(&self, patch_json: &str) {
        let mut txn = self.doc.transact_mut();
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
    /// if non-empty; terrain only if valid; `time`/`weather` merged onto the existing environment.
    pub fn apply_row_meta(
        &self,
        title: &str,
        terrain: &str,
        time_of_day: Option<String>,
        weather: Option<String>,
    ) {
        let mut txn = self.doc.transact_mut();
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
    }

    /// Seed default meta if empty (mirrors `ydoc.seedMeta` + `DEFAULT_META`). No-op if meta exists.
    pub fn seed_meta(&self, id: &str, title: &str) {
        let mut txn = self.doc.transact_mut();
        if self.meta.len(&txn) > 0 {
            return;
        }
        self.meta.insert(&mut txn, "id", id);
        self.meta.insert(&mut txn, "title", title);
        self.meta.insert(&mut txn, "terrain", "everon");
        let mut env: HashMap<String, Any> = HashMap::new();
        env.insert("time".to_string(), Any::String("06:00".into()));
        env.insert("weather".to_string(), Any::String("clear".into()));
        env.insert("viewDistance".to_string(), Any::BigInt(1600));
        env.insert("thermals".to_string(), Any::Bool(false));
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
    pub fn hydrate(&self, payload_json: &str, default_layer_id: &str) {
        let Any::Map(payload) = json_str_to_any(payload_json) else {
            return;
        };
        // Grab the 5 non-tracked map handles before opening the txn (`get_or_insert_map` takes &self).
        let loadouts = self.doc.get_or_insert_map("loadouts");
        let items = self.doc.get_or_insert_map("items");
        let objectives = self.doc.get_or_insert_map("objectives");
        let vehicles = self.doc.get_or_insert_map("vehicles");
        let markers = self.doc.get_or_insert_map("markers");

        let mut txn = self.doc.transact_mut();
        for m in [
            &self.slots,
            &self.squads,
            &self.factions,
            &self.editor_layers,
            &loadouts,
            &items,
            &objectives,
            &vehicles,
            &markers,
        ] {
            m.clear(&mut txn);
        }

        if let Some(env) = payload.get("environment") {
            self.meta.insert(&mut txn, "environment", env.clone());
        }
        if let Some(Any::Map(map)) = payload.get("map")
            && let Some(Any::String(terrain)) = map.get("terrain")
        {
            self.meta.insert(&mut txn, "terrain", terrain.as_ref());
        }

        load_rows(&mut txn, &objectives, payload.get("objectives"));
        load_rows(&mut txn, &vehicles, payload.get("vehicles"));
        load_rows(&mut txn, &markers, payload.get("markers"));
        if let Some(Any::Map(lo)) = payload.get("loadouts") {
            for v in lo.values() {
                load_row(&mut txn, &loadouts, v);
            }
        }

        if let Some(Any::Map(editor)) = payload.get("editor") {
            load_rows(&mut txn, &self.factions, editor.get("factions"));
            load_rows(&mut txn, &self.squads, editor.get("squads"));
            load_rows(&mut txn, &self.slots, editor.get("slots"));
            load_rows(&mut txn, &self.editor_layers, editor.get("editorLayers"));
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
        let mut txn = self.doc.transact_mut();
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
        let mut txn = self.doc.transact_mut();
        if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, id) {
            layer.insert(&mut txn, "name", name);
        }
    }

    /// Reparent an Outliner folder; rejects cycles (dropping it into its own subtree). Mirrors
    /// `ydoc.reparentEditorLayer`.
    pub fn reparent_editor_layer(&self, id: &str, new_parent_id: Option<String>) {
        let mut txn = self.doc.transact_mut();
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
        let mut txn = self.doc.transact_mut();
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
}

impl Default for MissionDocCore {
    fn default() -> Self {
        Self::new()
    }
}

/// A `{x,y,z,rotation}` plain object as a `yrs` `Any::Map` (how Yjs stores `Slot.position`).
fn position_any(x: f64, y: f64, z: f64, rotation: f64) -> Any {
    let mut m: HashMap<String, Any> = HashMap::new();
    m.insert("x".to_string(), Any::Number(x));
    m.insert("y".to_string(), Any::Number(y));
    m.insert("z".to_string(), Any::Number(z));
    m.insert("rotation".to_string(), Any::Number(rotation));
    Any::Map(Arc::new(m))
}

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

/// Load an array of entity rows (`Some(Any::Array)`) into `map`. `hydrate`'s `setEach`.
fn load_rows(txn: &mut TransactionMut, map: &MapRef, rows: Option<&Any>) {
    if let Some(Any::Array(arr)) = rows {
        for row in arr.iter() {
            load_row(txn, map, row);
        }
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

/// Read `position` (`Any::Map`) → `(x, y, z, rotation)`; missing map/keys read as 0.
fn read_position<T: ReadTxn>(txn: &T, slot: &MapRef) -> (f64, f64, f64, f64) {
    if let Some(Out::Any(Any::Map(m))) = slot.get(txn, "position") {
        let g = |k: &str| m.get(k).map_or(0.0, any_to_f64);
        (g("x"), g("y"), g("z"), g("rotation"))
    } else {
        (0.0, 0.0, 0.0, 0.0)
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
}
