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
use yrs::{Any, Doc, Map, MapPrelim, MapRef, Out, ReadTxn, StateVector, Transact, Update};

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
    editor_layers: MapRef,
    /// `M = ()`: no per-stack-item metadata needed for the spike.
    undo_mgr: UndoManager<()>,
}

impl MissionDocCore {
    /// A fresh, empty document with the two tracked root maps + an undo manager scoped to both.
    #[must_use]
    pub fn new() -> Self {
        let doc = Doc::with_client_id(CLIENT_ID);
        let slots = doc.get_or_insert_map("slots");
        let editor_layers = doc.get_or_insert_map("editorLayers");

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
        undo_mgr.expand_scope(&doc, &editor_layers);

        Self {
            doc,
            slots,
            editor_layers,
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

    /// Insert one slot as a nested map (mirrors `slots.set(id, new Y.Map())` + a plain-object
    /// `position`). Slot-only — layer/squad wiring is exercised by the real-`ydoc` criterion-2 path.
    // Transform is 4 scalars (x/y/z/rotation) alongside id/squad/role — mirrors the wasm shim's flat
    // signature; a struct would only add ceremony to a spike mutator.
    #[allow(clippy::too_many_arguments)]
    pub fn add_slot(
        &self,
        id: &str,
        squad_id: &str,
        role: &str,
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
        slot.insert(&mut txn, "role", role);
        slot.insert(&mut txn, "stance", "stand");
        slot.insert(&mut txn, "position", position_any(x, y, z, rotation));
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
        doc.add_slot("s1", "sq1", "Rifleman", 100.5, 200.25, 0.0, 0.0);
        doc.add_slot("s2", "sq1", "Squad Leader", 300.0, 400.0, 5.0, 90.0);

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
        a.add_slot("s1", "sq1", "Rifleman", 1.0, 2.0, 3.0, 4.0);
        a.add_slot("s2", "sq1", "Medic", 5.0, 6.0, 7.0, 8.0);
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
        doc.add_slot("s1", "sq1", "Rifleman", 0.0, 0.0, 0.0, 0.0);
        doc.add_slot("s2", "sq1", "Rifleman", 1.0, 1.0, 0.0, 0.0);
        doc.add_slot("s3", "sq1", "Rifleman", 2.0, 2.0, 0.0, 0.0);
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
        doc.add_slot("s1", "sq1", "Rifleman", 100.5, 200.25, 0.0, 90.0);
        let json = doc.slots_json();
        assert!(json.contains("\"s1\""), "{json}");
        assert!(json.contains("Rifleman"), "{json}");
        assert!(json.contains("100.5"), "{json}"); // exact f64 position (not the f32 SoA)
    }
}
