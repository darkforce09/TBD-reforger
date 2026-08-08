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

/// T-222 — this file used to carry `const CLIENT_ID: u64 = 1`, handed to *every* document, with a
/// comment asserting a client-id clash "is harmless". It was harmless only because no peer update
/// could ever arrive. In a CRDT the client id is what makes concurrent edits *distinguishable*:
/// `yrs` keys every block by `(client, clock)` and orders concurrent writes by that pair. Two live
/// writers sharing one id emit overlapping `(1, 0..n)` ranges, so a receiver reads the peer's
/// history as a continuation of its own, drops the blocks it believes it already has, and returns
/// `Ok`. Silent data loss on every multi-peer merge — which is why this blocked four tickets.
///
/// [`MissionDocCore::new`] now takes a randomized id (see there for why that is wasm-correct with
/// no new dependency), [`MissionDocCore::with_client_id`] is the deterministic escape hatch, and
/// [`MissionDocCore::apply_update`] refuses a collision instead of corrupting quietly.
///
/// A `yrs` client id is **53-bit**: `ClientID::new` carries `debug_assert!(value & (u64::MAX << 53)
/// == 0)`, so a debug build panics on a wider value and a release build silently truncates it.
/// `0..2^53` is also exactly the JS `Number.MAX_SAFE_INTEGER` range `Y.Doc` uses, so staying inside
/// it is what keeps the wire Yjs-compatible.
const CLIENT_ID_BITS: u32 = 53;

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
    /// Root `zones` map (`zonesById` in [`Self::small_maps_json`]) — undo-scoped (T-211).
    /// Authored play-area / objective zones; each row is `mission.schema.json#/$defs/zone`
    /// verbatim. See the T-211 mutator block ([`Self::add_circle_zone`]) for the field contract.
    zones: MapRef,
    /// T-650 — root `compositions` map (`compositionsById` in [`Self::small_maps_json`]) —
    /// undo-scoped. See the T-650 mutator block ([`Self::add_composition`]) for the row contract
    /// and the module-level ROUTING note below for why this is a doc-side collection that must
    /// export cleanly to a later user-scoped API row.
    ///
    /// ── T-650 STORAGE ROUTING (dispatcher decision — the ticket's user-scoped framing is not
    /// foreclosed) ──────────────────────────────────────────────────────────────────────────────
    /// Compositions are **DOC-SIDE in this slice**: a `compositions` root map, each row a
    /// SELF-CONTAINED JSON object `{id, title, author, category, entities:[…]}` where `entities`
    /// is the captured selection as **relative-offset** entries (slots with role/asset/loadout +
    /// vehicles with heading/crew SHAPE + objects with alias/resourceName, each `{dx, dz}` from the
    /// capture centroid). The shape reuses the clipboard capture shapes (`copy_selection` /
    /// `paste_at_cursor` in `editor_ops.rs`).
    ///
    /// **Shaped for a MECHANICAL later lift to user-scoped API rows.** Each row is self-contained:
    /// no cross-references into the rest of the doc beyond asset/kit ids (a `resourceName` /
    /// `assetId` / `loadout` blob is a value, not a doc pointer). So the row that rides
    /// `compositionsById` here is byte-for-byte the row a `POST /compositions` would store — the
    /// later lift moves WHERE the map lives (a user-scoped table instead of the mission doc) and
    /// changes nothing about the row. `author` is the current user's display string **as authored**
    /// (no server assignment in this framing).
    ///
    /// **Why the round trip needs no `mission/compile.rs` change** (that file is not this slice's
    /// owns): this mirrors the T-211 zones precedent exactly. `small_maps_json` emits the canonical
    /// `compositionsById` AND a transitional `payloadExtras.compositions` projection;
    /// `compile_payload` promotes any `payloadExtras` key it neither knows nor authored onto the
    /// wire root (T-219), so `compositions` lands at the payload root; [`Self::hydrate`] loads a
    /// top-level `compositions[]` back into this root (and `compositions` is in
    /// `is_known_editor_payload_top_level`). Self-healing: the day compile.rs authors `compositions`
    /// itself, the projection becomes dead weight to delete, not a double-emit.
    compositions: MapRef,
    /// T-079 — root `triggers` map (`triggersById` in [`Self::small_maps_json`]) — undo-scoped. The
    /// EDITOR half of triggers: an authored area (zone geometry) plus a stored-not-evaluated
    /// activation kind and an optional owner-entity link. See the T-079 mutator block
    /// ([`Self::add_circle_trigger`]) for the row contract, and the [`Self::zones`] field's T-211
    /// routing note above — this mirrors it exactly (canonical `triggersById` + a transitional
    /// `payloadExtras.triggers` projection in `small_maps_json`, top-level `triggers[]` loaded back by
    /// [`Self::hydrate`], `triggers` listed in `is_known_editor_payload_top_level`).
    ///
    /// ── T-079 STORAGE / SPLIT NOTE ──────────────────────────────────────────────────────────────
    /// A trigger row is `#/$defs/zone`-shaped for its `shape` and `rules` (so trigger GEOMETRY rides
    /// the SHIPPED zone draw tool and trigger RULES reuse `$defs/zoneRules`, per the ticket) PLUS the
    /// three trigger-only keys `name`, `ownerId`, `activation`:
    ///
    ///   id          String, required — the doc key.
    ///   name        String, optional — the author-facing label (empty allowed; the tool clears the
    ///               key rather than writing `""` when the box is emptied, mirroring `zone.label`).
    ///   shape       Object, `#/$defs/shape` — EXACTLY ONE of circle {x,z,r} / polygon [[x,z],…],
    ///               written by the SAME `circle_shape_any` / `polygon_shape_any` the zone mutators
    ///               use (the second-consumer requirement — no second geometry vocabulary).
    ///   ownerId     String, optional — the placed slot/vehicle this trigger belongs to
    ///               (CONN-TRG-OWNER-001, the data edge). `None`/absent = unowned. A DANGLING owner
    ///               (the entity was deleted) is TOLERATED: the row keeps the id, and the render/read
    ///               simply resolves nothing (no cascade delete here — the owner map is another
    ///               slice's, and a broken edge must degrade, not panic).
    ///   activation  String, optional — one of `presence` / `radio` / `timer`, a TYPED PLACEHOLDER
    ///               that is STORED, NOT EVALUATED (the activation/effects runtime is T-676). Written
    ///               opaquely; the editor offers exactly the three the ticket names.
    ///   rules       Object, optional, `#/$defs/zoneRules` — stored OPAQUE and whole, exactly like
    ///               [`Self::set_zone_rules`] (the same closed-vocabulary reason: a typed mirror would
    ///               be the second vocabulary T-241 exists to prevent).
    ///
    /// **Why the wire needs no `mission/compile.rs` change** (that file is not this slice's owns): the
    /// schema does not declare `triggers` yet — T-706 (wave 120) declares it. Until then the round trip
    /// is closed by the transitional `payloadExtras.triggers` projection promoted by `compile_payload`
    /// (T-219), exactly as `zones` / `compositions` do. Self-healing on the same terms: the day
    /// compile.rs authors `triggers` from `triggersById`, the projection retires. See the
    /// forward-constraint comment in `small_maps_json`.
    triggers: MapRef,
    /// T-651 — root `comments` map (`commentsById` in [`Self::small_maps_json`]) — undo-scoped.
    /// EDITOR-ONLY VIRTUAL ENTITIES (`PLACE-COMMENT-001`): an authored title, tooltip and world
    /// position that live in the outliner and on no game server, ever.
    ///
    /// ── T-651 — WHY A COMMENT CAN NEVER COMPILE, AND WHERE THAT IS ENFORCED ─────────────────────
    /// The constraint is STRUCTURAL, not a filter someone has to remember to run: `comments` is not
    /// a key of `mission::flatten::EditorPayload`, the struct `flatten_to_mod_document` deserialises
    /// the saved payload into. Serde therefore drops the array before any mod-document code sees it
    /// — the same mechanism the root `markers` map's §authority note describes at
    /// [`Self::set_faction_briefing_marker`] ("declares no root key whatsoever, so that lane is
    /// never compiled"), and the same one `editorHidden` rides.
    ///
    /// So the rule holds by NOT declaring, which means the way to break it is to ADD a declaration
    /// (in `mission/flatten.rs`, another slice's file) or to write a comment row into a root that
    /// IS compiled. `comments_never_reach_the_mod_document` fires on exactly that second failure.
    ///
    /// **Persistence is a different question and has a different answer.** A comment MUST survive
    /// Save→Load or the feature is a toy, so it rides the EDITOR payload exactly as `zones` /
    /// `compositions` / `triggers` do (T-211's transitional route, mirrored verbatim):
    /// `small_maps_json` emits the canonical `commentsById` AND a `payloadExtras.comments`
    /// projection; `compile_payload` promotes any `payloadExtras` key it neither knows nor authors
    /// onto the EDITOR-payload root (T-219); [`Self::hydrate`] loads a top-level `comments[]` back
    /// here, and `comments` is in `is_known_editor_payload_top_level` so it is not double-parked.
    ///
    /// Unlike those three, this projection is **NOT transitional and does not retire.** Zones,
    /// compositions and triggers are all waiting for `mission/compile.rs` to author their key and
    /// for the schema to declare them. Comments are waiting for nothing: `packages/tbd-schema/`
    /// must never gain a `comments` property, so the editor-payload root is this row's FINAL home.
    ///
    /// ROW SHAPE — the three `ATTR-FIELD-CMT-*` fields and nothing else:
    ///   id        String, required — the doc key.
    ///   title     String — ATTR-FIELD-CMT-TITLE. The outliner row label.
    ///   tooltip   String — ATTR-FIELD-CMT-TOOLTIP. The long body (FNF v3's seven-paragraph
    ///             tutorial lived in a field like this one); rendered as the row's hover text.
    ///   position  Object `{x, z}` — ATTR-FIELD-CMT-POSITION, world metres. Shaped `{x, z}` (NOT
    ///             `{x, y}`) to match `$defs/marker` and the zone/trigger circle centres, so the
    ///             one coordinate vocabulary in this document stays one.
    ///   layerId is deliberately ABSENT: a comment is filed by being listed in an
    ///   `editorLayers[].entityIds` array, the identical mechanism a slot uses
    ///   ([`Self::move_comment_to_layer`]), so "supports layers" needs no second representation.
    ///
    /// ── CORPUS EVIDENCE, WEIGHTED HONESTLY ──────────────────────────────────────────────────────
    /// This feature is evidenced by **ONE community across TWO eras**, not by a four-way
    /// convergence, and the distinction is load-bearing for anyone deciding how much to build here.
    /// FNF v3 ships 28 in-map Comment objects including a seven-paragraph tutorial
    /// (`mission.sqm:4093`) plus per-object instructions. FNF v4 then deleted the 219-line
    /// `configGuide.txt` and the entire 421-file template, and what survived as onboarding is
    /// literally TWO Comment entities. **WOG and OFCRA have no comment equivalent at all.** The
    /// signal is that the mechanism survived a total rewrite that threw away everything else —
    /// which is why [`Self::seed_template_comments`] seeds exactly two, and not twenty-eight.
    comments: MapRef,
    /// T-672 — root `connections` map (`connectionsById` in [`Self::small_maps_json`]) — undo-scoped.
    /// The EDITOR-ONLY connection graph (`CONN-START-001` / `CONN-SYNC-001` / `CONN-DEL-001`): the
    /// author-declared relations between two placed things, as Eden's `Connect ▸` submenu makes them.
    ///
    /// ── T-672 — WHY A CONNECTION CAN NEVER COMPILE, AND WHERE THAT IS ENFORCED ──────────────────
    /// This adopts the T-651 `comments` arrangement, and it does so on MEASURED evidence rather than
    /// by analogy. `packages/tbd-schema/schema/mission.schema.json` sets `additionalProperties:
    /// false` at its top level and declares nineteen properties — `schemaVersion`, `meta`,
    /// `environment`, `factions`, `orbat`, `slots`, `radioPlan`, `zones`, `entities`, `layers`,
    /// `flow`, `winConditions`, `briefings`, `settings`, `objectives`, `vehicles`, `editorTriggers`,
    /// `variants`, `missionParams`. **None of them is a relation collection**, and neither
    /// `$defs/entity` nor `$defs/slot` nor `$defs/vehicle` carries a `syncedTo` / `connections` /
    /// `owner` field. There is nowhere in the compiled mission for an edge to land: a connection
    /// that "compiled" could only compile into a key the schema rejects.
    ///
    /// So the rule holds by NOT declaring — `connections` is not a key of
    /// `mission::flatten::EditorPayload`, so serde drops the array before any mod-document code sees
    /// it. The way to break it is to ADD a declaration (in `mission/flatten.rs`, another slice's
    /// file) or to write an edge row into a root that IS compiled;
    /// `connections_never_reach_the_mod_document` fires on exactly that second failure, the way
    /// T-651's twin does.
    ///
    /// **Persistence is the different question with the different answer**, identically to comments:
    /// `small_maps_json` emits the canonical `connectionsById` AND a `payloadExtras.connections`
    /// projection; `compile_payload` promotes it onto the EDITOR-payload root (T-219);
    /// [`Self::hydrate`] loads a top-level `connections[]` back here, and `connections` is in
    /// `is_known_editor_payload_top_level` so it is not double-parked. Like comments — and unlike
    /// zones / compositions / triggers — this projection is **PERMANENT**: no later ticket teaches
    /// `mission/compile.rs` to author the key, because the schema must never declare one.
    ///
    /// ROW SHAPE — four strings and nothing else:
    ///   id    String, required — the doc key.
    ///   kind  String — `"sync"` | `"group"` | `"triggerOwner"`, the three verbs Eden's `Connect ▸`
    ///         submenu offers ([`ConnectionKind`]). An unknown kind is refused at
    ///         [`Self::add_connection`], so the vocabulary cannot fork the way T-241 exists to stop.
    ///   from  String — the source id (the entity the operator armed the connect ON).
    ///   to    String — the target id.
    ///
    /// **`sync` is UNDIRECTED and is normalised at write** (endpoints sorted — see
    /// [`ConnectionKind::is_directed`]) so `sync(A,B)` and `sync(B,A)` are ONE edge and the duplicate
    /// rule can actually see them. `group` / `triggerOwner` are directed and stored verbatim.
    ///
    /// ── THE FNF v4 WARNING, AND WHAT IT BOUGHT ──────────────────────────────────────────────────
    /// The framework corpus puts FNF v4's entire defect cluster on exactly this mechanism. That is
    /// why this collection ships its READ and CHECK surfaces ([`Self::connection_rows_json`],
    /// [`Self::connection_findings_json`]) in the SAME edit as its write surface, and why every
    /// refusal at [`Self::add_connection`] has a matching *finding* code: the mutator can keep this
    /// editor's own authoring clean, but a hydrate can still bring in a self-link, a duplicate or a
    /// dangling endpoint from a payload this editor did not author, and an edge nobody can enumerate
    /// or validate is what that cluster is made of.
    connections: MapRef,
    /// When true, mutators stamp `INIT` (untracked) instead of `LOCAL` — set around boot / hydrate /
    /// default-seeding so a load is not an undo step. Interior mutability: mutators take `&self`.
    init_mode: Cell<bool>,
    /// `M = ()`: no per-stack-item metadata needed.
    undo_mgr: UndoManager<()>,
}

impl MissionDocCore {
    /// A fresh, empty document with the tracked root maps + an undo manager scoped to them, on a
    /// **randomized client id** — the constructor every peer uses.
    ///
    /// # Where the id comes from, and why it works in wasm
    ///
    /// `Doc::new()` is `yrs`'s own randomized-identity constructor: it draws a 53-bit
    /// `ClientID::random()` from `fastrand`. That is the same policy `Y.Doc` follows in JS, so the
    /// wire stays Yjs-compatible.
    ///
    /// The reason this is the right source — and the thing worth checking before reaching for
    /// anything else — is that it costs **no new dependency**. This crate is pure Rust and also
    /// compiles to `wasm32-unknown-unknown`, a target with no ambient entropy: `getrandom` 0.3
    /// hard-`compile_error!`s there unless its `wasm_js` backend is on, and this workspace sets no
    /// `getrandom_backend` cfg. `yrs` has already solved that for us — it depends on
    /// `fastrand` with the `js` feature, which turns on `getrandom/wasm_js` (i.e.
    /// `crypto.getRandomValues`) for exactly the wasm targets. Verified with
    /// `cargo tree -p website-frontend --target wasm32-unknown-unknown -e features`:
    /// `fastrand "js"` → `fastrand "getrandom"` → `getrandom "wasm_js"` → `getrandom v0.3.4`.
    /// Adding our own `rand`/`uuid`/`getrandom` would have put a *fourth* getrandom in a
    /// workspace-wide lockfile that already carries three majors, for entropy we already have.
    ///
    /// **The hazard to know about:** that correctness rides on `fastrand`'s `js` feature staying
    /// enabled. Turn it off (e.g. a `default-features = false` on `yrs`) and `fastrand` falls back
    /// to a *hardcoded* `DEFAULT_RNG_SEED` on wasm with no compile error, so `ClientID::random()`
    /// would return the same value in every browser — T-222 reborn wearing a "random" label. That
    /// is a second reason [`Self::apply_update`]'s collision guard exists: it fails loudly if the
    /// entropy ever silently degrades.
    #[must_use]
    pub fn new() -> Self {
        Self::from_doc(Doc::new())
    }

    /// A fresh, empty document that identifies as `client_id` — the **deterministic** constructor.
    ///
    /// Use it when the identity must be reproducible (unit tests pinning a merge scenario) or when
    /// a host wants to mint the id itself. Everyday code wants [`Self::new`].
    ///
    /// # The contract on `client_id`
    ///
    /// * **Unique across every writer that can concurrently edit the same document.** `yrs` orders
    ///   concurrent blocks by `(client, clock)`; two live writers on one id produce overlapping
    ///   ranges that no merge can separate. [`Self::apply_update`] rejects that rather than
    ///   corrupting, but rejection is a backstop, not a plan.
    /// * **At most [`CLIENT_ID_BITS`] (53) bits.** A wider value panics a debug build inside
    ///   `yrs::ClientID::new` and is silently truncated in release.
    /// * **Stable for the life of one document session** — from construction until this
    ///   `MissionDocCore` is dropped. It must not change under the doc, because every clock already
    ///   handed out is relative to it.
    /// * **Not stable across sessions, deliberately.** A reload takes a fresh id; see
    ///   [`Self::apply_update`] for why rehydration does not resurrect the persisted one.
    #[must_use]
    pub fn with_client_id(client_id: u64) -> Self {
        debug_assert!(
            client_id >> CLIENT_ID_BITS == 0,
            "yrs client ids are {CLIENT_ID_BITS}-bit; {client_id} would be truncated"
        );
        Self::from_doc(Doc::with_client_id(client_id))
    }

    /// Shared body of the two constructors: bind the root maps and scope the undo manager. Split out
    /// so `new` and `with_client_id` cannot drift — the identity is the only difference between them.
    fn from_doc(doc: Doc) -> Self {
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
        undo_mgr.expand_scope(&doc, &zones);
        // T-650 — a save/rename/delete of a composition row is an undoable user edit, exactly like a
        // zone or a crew board.
        undo_mgr.expand_scope(&doc, &compositions);
        // T-079 — authoring a trigger (draw / rename / owner / activation / rules / delete) is an
        // undoable user edit, exactly like a zone.
        undo_mgr.expand_scope(&doc, &triggers);
        // T-651 — placing, retitling, dragging, copying or deleting a comment is an undoable user
        // gesture like any other authoring act. Editor-only ≠ untracked: an accidental Delete on an
        // annotation that carried a seven-paragraph brief must be Ctrl+Z-able.
        undo_mgr.expand_scope(&doc, &comments);
        // T-672 — drawing or deleting a connection is an undoable user gesture. This matters more
        // here than anywhere else in this list: `CONN-DEL-001` is a DESTRUCTIVE verb on a relation
        // that has no glyph of its own, so the only way an operator recovers from deleting the wrong
        // edge is Ctrl+Z. Scoping the root is what makes that true.
        undo_mgr.expand_scope(&doc, &connections);

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
            init_mode: Cell::new(false),
            undo_mgr,
        }
    }

    /// This document's client id — the identity every block it authors is keyed by. T-222; callers
    /// that mint the id (and tests that assert two peers really are two peers) read it back here.
    #[must_use]
    pub fn client_id(&self) -> u64 {
        // `yrs::ClientID` is a 53-bit newtype over NonZeroU64; `.get()` unwraps the plain value.
        self.doc.client_id().get()
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
    /// This is both the peer-sync seam and the **persistence restore** seam, and T-222 turns on the
    /// difference between them:
    ///
    /// * **Restore into a fresh document** (the boot path — `mission_editor.rs` builds a new core and
    ///   replays the IndexedDB blob into it) carries blocks authored by the *previous* session's
    ///   client. We have authored nothing yet, so there is no overlap: the blob's history is adopted
    ///   wholesale and the new session then authors under its own, new id. **A rehydrated document
    ///   deliberately does not resurrect the persisted client id.** Two tabs of the same user restore
    ///   the *same* blob from the same origin's IndexedDB; if the id rode along in it, both tabs
    ///   would wake up as the same peer and then diverge under one identity — the T-222 bug reborn
    ///   one layer down. Nothing is lost by taking a fresh id: replay reproduces the document exactly
    ///   (`encode_decode_roundtrip_is_stable`), and this matches `Y.Doc`, which also mints a new
    ///   client id on every construction and never restores one from y-indexeddb.
    /// * **A concurrent writer that shares our client id** is the unrecoverable case. `yrs` would
    ///   splice its blocks onto our own `(client, clock)` sequence and merge two authors into one
    ///   history, silently and successfully. We refuse what we can detect of it instead: a *loud*
    ///   failure the caller can surface beats a document that merges wrong and reports OK.
    ///
    /// # The collision guard, and exactly how far it reaches
    ///
    /// The predicate is **"the update claims my own client id has progressed past the last clock I
    /// issued"** — `update.state_vector().get(&me) > my_clock`. Only another writer authoring as me
    /// can produce that.
    ///
    /// It deliberately does *not* fire on the two legitimate cases that also carry our id:
    /// * a **restore into a fresh doc**, where `my_clock == 0` and the blob simply gets adopted; and
    /// * a **re-exchange**, where a peer echoes our own already-integrated blocks back at us. That
    ///   is normal, idempotent traffic in any sync transport, and an earlier draft of this guard
    ///   rejected it — caught by `two_peers_with_distinct_ids_merge_concurrent_edits`, which
    ///   re-exchanges on purpose for exactly that reason.
    ///
    /// **What it cannot catch:** a colliding writer that is *behind* us on the shared id. Its blocks
    /// sit inside a clock range we have already issued, so `yrs` discards them as "already seen" and
    /// the wire format offers nothing to tell that apart from an echo — it is not a weak
    /// implementation, it is undecidable from a v1 update. `yrs` says the same in `ClientID`'s own
    /// docs: *"No two active peers are allowed to share the same ClientID. If that happens,
    /// following updates may cause document store to be corrupted."* So this is a **backstop that
    /// makes the common degradation loud, not a licence to reuse ids** — the real guarantee is that
    /// [`Self::new`] draws a fresh 53-bit random id per document.
    ///
    /// # Errors
    /// Returns a message on a malformed update, an integration failure, or a detected client-id
    /// collision with a concurrent writer.
    pub fn apply_update(&self, bytes: &[u8]) -> Result<(), String> {
        let update = Update::decode_v1(bytes).map_err(|e| e.to_string())?;
        let me = self.doc.client_id();
        let my_clock = self.doc.transact().state_vector().get(&me);
        let claimed = update.state_vector().get(&me);
        // `my_clock == 0` is the fresh-doc restore: we have issued nothing, so the blob's history
        // under our id is simply adopted as ours. Only a doc that has already authored can be
        // *overtaken* by a second writer on the same id.
        if my_clock != 0 && claimed > my_clock {
            return Err(format!(
                "client id collision: incoming update carries blocks authored by client {me} up to \
                 clock {claimed}, but that is this document's own id and it has only issued {my_clock}. \
                 Another writer is authoring as us; applying this would interleave two authors into \
                 one history. Give every peer its own id — MissionDocCore::new() mints one."
            ));
        }
        // Always INIT (untracked) regardless of mode — a persistence restore / peer sync is never an
        // undo step. (`begin()` would honor init-mode, but forcing INIT keeps this correct off-boot.)
        let mut txn = self.doc.transact_mut_with(INIT_ORIGIN);
        txn.apply_update(update).map_err(|e| e.to_string())
    }

    /// Encode the whole document as a Yjs-wire (v1) update stream — the persistence blob (criterion 3)
    /// and the seed a fresh peer replays. Deterministic for a GIVEN document: re-encoding the same
    /// doc twice is byte-identical, whatever its client id. It is NOT byte-comparable across docs —
    /// a fresh peer that replayed this stream re-encodes different bytes (its own id keys the
    /// blocks); only the *materialization* is equal. See `yrs_persist::slots_digest`.
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
    ///
    /// **T-211 — `zonesById` + the transitional `payloadExtras.zones` projection.** Authored zones
    /// are emitted twice, deliberately, and the two emits retire at different times:
    ///
    /// * `zonesById` is the CANONICAL by-id emit, shaped exactly like `entitiesById`. It is what
    ///   `compile_payload` will read once it grows the sibling one-liner
    ///   `"zones": values_of_ordered(&small, "zonesById", "zones")` (see this method's T-211 note in
    ///   the report — `mission/compile.rs` is a different slice's file).
    /// * `payloadExtras.zones` is the TRANSITIONAL wire route that closes the round trip TODAY.
    ///   `compile_payload` promotes any `payloadExtras` key it neither knows nor already authored
    ///   onto the wire root (T-219), and `zones` is currently both — so the array lands at the
    ///   payload root, which is exactly where `flatten.rs`'s `EditorPayload.zones` (T-201) reads it.
    ///
    /// **This is self-healing, not a landmine.** The moment `compile_payload` authors `zones`
    /// itself, its extras loop skips this key on BOTH of its guards (`is_known_editor_payload_top_
    /// level(k)` once `zones` joins `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS`, and `obj.contains_key(k)`
    /// regardless) — so the projection silently becomes dead weight to delete rather than a
    /// double-emit or an override. The ordering below matches `values_of_ordered` so the two routes
    /// cannot disagree about sequence during the overlap.
    #[must_use]
    pub fn small_maps_json(&self) -> String {
        // Grab the root handles before opening the read txn (`get_or_insert_map` takes `&self`).
        let meta = self.doc.get_or_insert_map("meta");
        let payload_extras = self.doc.get_or_insert_map("payloadExtras");
        let entity_order = self.doc.get_or_insert_map("entityOrder");
        let named: [(&str, MapRef); 14] = [
            ("factionsById", self.doc.get_or_insert_map("factions")),
            ("squadsById", self.doc.get_or_insert_map("squads")),
            ("loadoutsById", self.doc.get_or_insert_map("loadouts")),
            ("itemsById", self.doc.get_or_insert_map("items")),
            ("objectivesById", self.doc.get_or_insert_map("objectives")),
            ("vehiclesById", self.doc.get_or_insert_map("vehicles")),
            ("entitiesById", self.doc.get_or_insert_map("entities")),
            ("zonesById", self.doc.get_or_insert_map("zones")),
            ("markersById", self.doc.get_or_insert_map("markers")),
            (
                "editorLayersById",
                self.doc.get_or_insert_map("editorLayers"),
            ),
            // T-650 — the canonical by-id emit, shaped exactly like `zonesById`. The panel's
            // narrow reader ([`Self::compositions_json`]) reads the same map; a Save carries it.
            (
                "compositionsById",
                self.doc.get_or_insert_map("compositions"),
            ),
            // T-079 — the canonical by-id emit for triggers, shaped exactly like `zonesById`. The
            // panel's narrow reader ([`Self::triggers_json`]) reads the same map; a Save carries it.
            ("triggersById", self.doc.get_or_insert_map("triggers")),
            // T-651 — the canonical by-id emit for editor-only comments, shaped exactly like
            // `zonesById`. This key reaches the EDITOR payload and stops there; see the `comments`
            // field's never-compiles note for why `flatten_to_mod_document` cannot see it.
            ("commentsById", self.doc.get_or_insert_map("comments")),
            // T-672 — the canonical by-id emit for the editor-only connection graph, shaped exactly
            // like `commentsById`. This key reaches the EDITOR payload and stops there; see the
            // `connections` field's never-compiles note for why `flatten_to_mod_document` cannot see
            // it (the schema declares no relation collection at all).
            ("connectionsById", self.doc.get_or_insert_map("connections")),
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
        // T-211 — project authored zones into the compile side-channel as the ordered `zones[]`
        // array (see this method's doc comment for why, and for when this block retires). Built
        // before the `payloadExtras` emit so the live doc always wins over a stale parked copy.
        //
        // Reads `self.zones` (the struct field), NOT `self.doc.get_or_insert_map("zones")`:
        // `get_or_insert_map` opens its own transaction internally, and calling it while `txn` is
        // alive DEADLOCKS. That is why every other handle in this method is hoisted above the
        // `transact()` line. Measured here — four tests hung rather than failed, which is the
        // worse symptom because a hang reads as a slow gate, not a defect.
        let zone_rows = ordered_rows(&txn, &self.zones, &entity_order, "zones");
        // T-650 — same transitional projection for compositions: `compile_payload` does not yet
        // read `compositionsById`, so the round trip is closed by promoting `payloadExtras
        // .compositions` onto the wire root (T-219). Built off `self.compositions` (the struct
        // field, not `get_or_insert_map` — that deadlocks against the live `txn`, see the zones note
        // above), ordered like every sibling.
        let comp_rows = ordered_rows(&txn, &self.compositions, &entity_order, "compositions");
        // T-079 — the same transitional projection for triggers. The schema does not declare
        // `triggers` yet — T-706 (wave 120) declares it — so `compile_payload` cannot AUTHOR the key,
        // and the round trip is closed exactly as `zones`/`compositions` do: promote
        // `payloadExtras.triggers` onto the wire root (T-219). When T-706 lands and compile.rs grows
        // its own `"triggers": values_of_ordered(&small, "triggersById", "triggers")`, this
        // projection retires on both of the extras loop's guards (see the zones note above). Built
        // off `self.triggers` (the struct field, not `get_or_insert_map` — that deadlocks against the
        // live `txn`, the zones note's measured hang), ordered like every sibling.
        let trigger_rows = ordered_rows(&txn, &self.triggers, &entity_order, "triggers");
        // T-651 — the same projection for editor-only comments, and it is the ONLY thing that makes
        // a placed annotation survive a Save→Load. `compile_payload` promotes it onto the EDITOR
        // payload root; `flatten_to_mod_document` cannot see that root key, so the compiled mission
        // is unaffected (see the `comments` field's never-compiles note). Unlike its three
        // neighbours this projection is PERMANENT — there is no later ticket that teaches compile.rs
        // to author `comments`, because the schema must never declare one. Built off `self.comments`
        // (the struct field, not `get_or_insert_map` — that deadlocks against the live `txn`, the
        // zones note's measured hang), ordered like every sibling.
        let comment_rows = ordered_rows(&txn, &self.comments, &entity_order, "comments");
        // T-672 — the same projection for the editor-only connection graph, and it is the ONLY thing
        // that makes a drawn edge survive a Save→Load. `compile_payload` promotes it onto the EDITOR
        // payload root; `flatten_to_mod_document` cannot see that root key, so the compiled mission
        // is unaffected (see the `connections` field's never-compiles note). PERMANENT for the same
        // reason comments' is: there is no later ticket that teaches compile.rs to author
        // `connections`, because the schema must never declare one. Built off `self.connections` (the
        // struct field, not `get_or_insert_map` — that deadlocks against the live `txn`, the zones
        // note's measured hang), ordered like every sibling.
        let connection_rows = ordered_rows(&txn, &self.connections, &entity_order, "connections");
        // Omit when empty so a clean doc's snapshot shape stays unchanged.
        if payload_extras.len(&txn) > 0
            || !zone_rows.is_empty()
            || !comp_rows.is_empty()
            || !trigger_rows.is_empty()
            || !comment_rows.is_empty()
            || !connection_rows.is_empty()
        {
            let mut extras: HashMap<String, Any> = match payload_extras.to_json(&txn) {
                Any::Map(m) => (*m).clone(),
                _ => HashMap::new(),
            };
            if zone_rows.is_empty() {
                // A doc that authored zones and then deleted them all must not re-emit a stale
                // parked array — absence has to be expressible, not just non-empty presence.
                extras.remove("zones");
            } else {
                extras.insert("zones".to_string(), Any::Array(zone_rows.into()));
            }
            // T-650 — mirror the zones absence rule: deleting every composition must clear the wire,
            // not re-emit the last-saved array a reload parked.
            if comp_rows.is_empty() {
                extras.remove("compositions");
            } else {
                extras.insert("compositions".to_string(), Any::Array(comp_rows.into()));
            }
            // T-079 — same absence rule for triggers: deleting every trigger must clear the wire.
            if trigger_rows.is_empty() {
                extras.remove("triggers");
            } else {
                extras.insert("triggers".to_string(), Any::Array(trigger_rows.into()));
            }
            // T-651 — same absence rule for comments: deleting every annotation must clear the wire
            // rather than re-emit the array a reload parked.
            if comment_rows.is_empty() {
                extras.remove("comments");
            } else {
                extras.insert("comments".to_string(), Any::Array(comment_rows.into()));
            }
            // T-672 — same absence rule for connections: `CONN-DEL-001` deleting the last edge must
            // CLEAR the wire, not re-emit the array a reload parked. Absence has to be expressible
            // or "delete connection" silently un-deletes itself on the next load — the exact
            // half-applied-mutation shape the FNF v4 cluster is made of.
            if connection_rows.is_empty() {
                extras.remove("connections");
            } else {
                extras.insert(
                    "connections".to_string(),
                    Any::Array(connection_rows.into()),
                );
            }
            if !extras.is_empty() {
                root.insert("payloadExtras".to_string(), Any::Map(Arc::new(extras)));
            }
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
    ///
    /// T-665 — a slot filed under a **hidden** layer (or under a layer whose ancestor is hidden) is
    /// OMITTED from the SoA. This is the whole of "per-layer visibility": the render engine only ever
    /// sees the materialized SoA, so a filtered slot never uploads — cheaper than a render-side
    /// visibility mask and it keeps the flag out of the 6k-line engine. It is a VIEW filter only —
    /// the slot is untouched in the doc, still present in `slots_json` / `small_maps_json`, so a Save
    /// carries the full mission and un-hiding brings the slot straight back (no data loss).
    ///
    /// T-701 — a slot carrying its OWN editor-local `editorHidden` flag (per-ENTITY visibility, 3den
    /// E9 "Enable Visibility") is dropped from the SoA too. Effective-hidden here is the UNION —
    /// `layer-hidden OR entity-hidden` — so the per-entity check joins the per-layer one at this same
    /// filter site (an entity is hidden if its layer chain hides it OR it hides itself). Same VIEW
    /// contract as the layer flag: the row is untouched in the doc (`slots_json` still carries it),
    /// so it round-trips a Save and clearing the flag brings it straight back. Distinct from the
    /// layer flag: `editorHidden` rides the SLOT row, not a folder — a maker can declutter a single
    /// entity in a dense area without hiding its whole layer.
    #[must_use]
    pub fn materialize(&self) -> SlotSoa {
        let txn = self.doc.transact();

        // slotId -> layerId: the first Outliner folder whose `entityIds` lists the slot.
        // `hidden_layers` caches, per layer that files slots, whether it (or an ancestor) is hidden,
        // so the per-slot skip is a single map lookup rather than a fresh ancestor walk each row.
        let mut slot_layer: HashMap<String, String> = HashMap::new();
        let mut hidden_layers: HashMap<String, bool> = HashMap::new();
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
                hidden_layers
                    .entry(layer_id.to_string())
                    .or_insert_with(|| {
                        layer_flag_effective(&txn, &self.editor_layers, layer_id, "hidden")
                    });
            }
        }

        let mut soa = SlotSoa::default();
        let mut roles = Interner::new();
        let mut tags = Interner::new();
        let mut squads = Interner::new();
        let mut layers = Interner::new();

        for (id, out) in self.slots.iter(&txn) {
            let Out::YMap(slot) = out else { continue };
            // T-665 — drop slots on a hidden (or hidden-ancestor) layer before any column is pushed.
            // T-701 — effective-hidden is the UNION: OR the per-entity `editorHidden` flag on the
            // slot row itself, so an entity a maker hid individually is dropped even when its layer is
            // visible (and, symmetrically, a hidden layer still drops a slot whose own flag is unset).
            let layer_hidden = slot_layer
                .get(id)
                .is_some_and(|l| hidden_layers.get(l).copied().unwrap_or(false));
            if layer_hidden || read_bool(&txn, &slot, "editorHidden") {
                continue;
            }
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

    /// T-076 — board a placed slot into one seat of a vehicle: `vehicle.crew[seat_id] = slot_id`.
    ///
    /// **The crew map is doc state on the vehicle row**, exactly like [`Self::set_vehicle_cargo`]'s
    /// `cargo`: it rides `vehiclesById` (already undo-scoped and carried by [`Self::small_maps_json`]),
    /// so a board is PERSISTED with the mission, round-trips through Save→reload, and — because it
    /// goes through [`Self::begin`] — is ONE undo step. This is the shape the T-665 layer flags used
    /// (a per-row property that rides the row it belongs to) rather than a tenth root map: the seat
    /// assignment is meaningless without the vehicle, so it lives on the vehicle.
    ///
    /// `crew` is a MAP `seat_id → slot_id` (a *generic* seat model — driver/gunner/commander/cargoN —
    /// because vehicle data has no per-class seat schema yet; that is T-205). The seat ids are the
    /// authoring surface's, opaque to the doc.
    ///
    /// **The write rides the row exactly like cargo, on the HYDRATE axis too.** After a server-adopt
    /// boot the vehicle row is re-`hydrate`d, and [`load_row`] stores nested objects as opaque
    /// `Any::Map`s, not tracked `YMap`s — there is no `crew` sub-key to `insert` into. So this mutator
    /// uses the same whole-`Any` read-modify-write idiom cargo/briefing use ([`read_crew_map`] reads
    /// the whole map tolerant of BOTH shapes, mutate the plain map, [`write_crew_map`] writes it back
    /// whole): a board on a hydrated mission preserves the loaded crew instead of wiping it, and the
    /// eviction scan below sees hydrated crews instead of skipping them. Matching `Out::YMap` (the
    /// pre-fix shape) was dead or destructive on any mission opened on a second machine (wave-103).
    ///
    /// **One slot occupies at most ONE seat across ALL vehicles** — enforced HERE, not in the UI: the
    /// same soldier cannot be two places at once, and a rule the caller can forget to apply is not a
    /// rule. Before writing, `slot_id` is cleared from every other seat of every vehicle (its own
    /// included), so a re-board is a move, never a duplicate. Assigning a slot already in this exact
    /// seat is idempotent.
    ///
    /// No-ops (leaving the doc untouched) when the vehicle id is missing or `seat_id`/`slot_id` is
    /// empty — an empty key/value would author a crew entry no reader could act on. Unboard is
    /// [`Self::clear_crew_seat`]; there is deliberately no "clear by slot" here because the panel
    /// always knows the seat it is clearing.
    pub fn assign_crew_seat(&self, vehicle_id: &str, seat_id: &str, slot_id: &str) {
        if seat_id.is_empty() || slot_id.is_empty() {
            return;
        }
        let mut txn = self.begin();
        if self.vehicles.get(&txn, vehicle_id).is_none() {
            return;
        }
        // One-seat-per-slot: strip this slot from any seat of any vehicle before assigning it here.
        // Collect first (cannot mutate while iterating the vehicles map that owns the crew maps).
        // The crew map is read via [`read_crew_map`], the whole-`Any` reader — [`load_row`] stores
        // `crew` as an opaque `Any::Map` on a HYDRATED mission, not a tracked `YMap`, so an
        // `Out::YMap` match here would skip every hydrated crew and let the same slot sit in two
        // vehicles at once. This mirrors cargo's/briefing's read-modify-write-whole idiom (T-345).
        let mut evict: Vec<(String, String)> = Vec::new();
        for (vid, out_v) in self.vehicles.iter(&txn) {
            let Out::YMap(v) = out_v else { continue };
            for (sid, occ) in read_crew_map(&txn, &v) {
                if matches!(occ, Any::String(ref s) if s.as_ref() == slot_id)
                    && !(vid == vehicle_id && sid == seat_id)
                {
                    evict.push((vid.to_string(), sid));
                }
            }
        }
        for (vid, sid) in evict {
            if let Some(Out::YMap(v)) = self.vehicles.get(&txn, &vid) {
                let mut crew = read_crew_map(&txn, &v);
                crew.remove(&sid);
                // An empty crew map removes the key — a never-crewed vehicle's row stays byte-
                // identical to before this ticket (the `cargo`/`tag` omit idiom).
                write_crew_map(&mut txn, &v, crew);
            }
        }
        // Read the target crew whole, set the seat, write it back whole (hydrate-proof).
        let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) else {
            return;
        };
        let mut crew = read_crew_map(&txn, &v);
        crew.insert(seat_id.to_string(), Any::String(slot_id.into()));
        write_crew_map(&mut txn, &v, crew);
    }

    /// T-076 — record the RIGHT-CREW-001 manned/unmanned intent on a placed vehicle
    /// (`vehicle.crewed = false` for unmanned). This is authored placement STATE, additive to the
    /// T-180.2 row exactly like [`Self::set_vehicle_faction`]'s `factionId`: it says how the operator
    /// wants the vehicle to spawn, for the split-out vehicle-roster compile drop to honor.
    ///
    /// Stored only when `false` (unmanned): a with-crew vehicle is the Eden default, and the omit
    /// idiom keeps a manned vehicle's row byte-identical to before this ticket (absent ⇒ crewed).
    pub fn set_vehicle_crewed(&self, vehicle_id: &str, crewed: bool) {
        let mut txn = self.begin();
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            if crewed {
                v.remove(&mut txn, "crewed");
            } else {
                v.insert(&mut txn, "crewed", false);
            }
        }
    }

    /// T-076 — unboard: clear one seat of a vehicle's crew map. No-op when the vehicle or seat is
    /// absent. Removes the `crew` key entirely once its last seat is cleared, so an emptied crew
    /// leaves the row shape it had before any board (the omit idiom [`Self::set_vehicle_cargo`] uses
    /// for an empty cargo list). One LOCAL undo step, like the board it reverses.
    pub fn clear_crew_seat(&self, vehicle_id: &str, seat_id: &str) {
        let mut txn = self.begin();
        // Whole-map read-modify-write like the board it reverses: on a HYDRATED mission `crew` is an
        // opaque `Any::Map` ([`load_row`]), so matching `Out::YMap` would make unboard a silent
        // no-op. [`read_crew_map`] tolerates both shapes; [`write_crew_map`] drops the key once the
        // last seat is gone (the omit idiom). No-op when nothing was actually removed.
        if let Some(Out::YMap(v)) = self.vehicles.get(&txn, vehicle_id) {
            let mut crew = read_crew_map(&txn, &v);
            if crew.remove(seat_id).is_some() {
                write_crew_map(&mut txn, &v, crew);
            }
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

    // ── T-211 — authored play-area / objective zones (`zonesById`) ──────────────────────────────
    //
    // ROW SHAPE IS `mission.schema.json#/$defs/zone` VERBATIM — six keys, no editor-only extras:
    //
    //   id       String, required, `$defs/wireSafeString` + minLength 1
    //   type     String, required, one of spawn | objective_capture | objective_destroy |
    //            objective_hold_until | boundary | base_protection
    //   shape    Object, required, `$defs/shape` — EXACTLY ONE of `circle` {x,z,r} or
    //            `polygon` [[x,z],…]. The `oneOf` is why the two shapes are separate setters:
    //            a row carrying both keys is schema-INVALID, so no mutator can ever leave both.
    //   label    String, optional, wireSafeString, empty allowed (means "use the mod's
    //            PrettyZoneTitle fallback" — an empty label is a committed golden)
    //   faction  String, optional, `$defs/factionKey` (`^[a-z][a-z0-9_]*$`)
    //   rules    Object, optional, `$defs/zoneRules`
    //
    // WHY `rules` IS OPAQUE JSON AND NOT A TYPED RUST STRUCT. T-241 closed `zoneRules` to a
    // 16-key vocabulary with `additionalProperties: false` specifically so its four consumer
    // tickets would not each invent their own. A typed mirror here would BE a second vocabulary:
    // it would have to be edited in lockstep with the schema, it would silently drop a key the
    // schema declares but this struct forgot, and — per T-216 — emitting a key the schema does
    // NOT declare 500s `/compiled` for every mission. So `set_zone_rules` takes the object whole
    // and stores it opaquely, exactly as `update_slot_loadout` does. The schema stays the single
    // declaration site and the single validator.
    //
    // WHY THESE MUTATORS DO NOT RANGE-CHECK. `$defs/circle.r` is `exclusiveMinimum: 0`,
    // `$defs/polygon` is `minItems: 3`, and T-241/T-275 pinned the `zoneRules` minima and maxima.
    // None of that is re-encoded here, on purpose: a second copy of a bound is a second thing that
    // can drift from the schema, and a doc layer that silently clamps turns an authoring error
    // into a wrong-but-valid mission. These write faithfully; the schema rejects. The guard that
    // an in-progress polygon needs (do not COMMIT a zone until the ring closes with ≥3 points) is
    // the draw tool's, and is specified in the T-211 follow-up rather than smeared across both.

    /// T-211 — create a circle zone. `kind` is schema `zone.type`; `x`/`z` are world metres and
    /// `r` the radius. Mirrors [`Self::add_entity`]'s row-construction idiom.
    pub fn add_circle_zone(&self, id: &str, kind: &str, x: f64, z: f64, r: f64) {
        let mut txn = self.begin();
        let zone = self
            .zones
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        zone.insert(&mut txn, "type", kind);
        zone.insert(&mut txn, "shape", circle_shape_any(x, z, r));
    }

    /// T-211 — create a polygon zone from a FLAT `[x0,z0,x1,z1,…]` ring (the wasm-boundary shape:
    /// one `Vec<f64>` crosses cheaply where a `Vec<Vec<f64>>` does not). A trailing unpaired
    /// coordinate is dropped rather than written as a malformed vertex.
    pub fn add_polygon_zone(&self, id: &str, kind: &str, points_flat: &[f64]) {
        let mut txn = self.begin();
        let zone = self
            .zones
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        zone.insert(&mut txn, "type", kind);
        zone.insert(&mut txn, "shape", polygon_shape_any(points_flat));
    }

    /// T-211 — reshape an existing zone to a circle (drag / resize). Replaces the whole `shape`
    /// object, so a polygon becomes a circle without leaving both `oneOf` branches present.
    pub fn set_zone_circle(&self, zone_id: &str, x: f64, z: f64, r: f64) {
        let mut txn = self.begin();
        if let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) {
            zone.insert(&mut txn, "shape", circle_shape_any(x, z, r));
        }
    }

    /// T-211 — reshape an existing zone to a polygon (vertex edit). Same whole-`shape` replacement
    /// as [`Self::set_zone_circle`], for the same `oneOf` reason.
    pub fn set_zone_polygon(&self, zone_id: &str, points_flat: &[f64]) {
        let mut txn = self.begin();
        if let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) {
            zone.insert(&mut txn, "shape", polygon_shape_any(points_flat));
        }
    }

    /// T-211 — schema `zone.type`. Retyping is a rules-preserving edit: the vocabulary is FLAT and
    /// not narrowed by type (T-241), so a `captureSeconds` left on a retyped boundary zone parses
    /// and is ignored exactly as it does today rather than failing the document.
    pub fn set_zone_type(&self, zone_id: &str, kind: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) {
            zone.insert(&mut txn, "type", kind);
        }
    }

    /// T-211 — schema `zone.label` (optional). `None` REMOVES the key; `Some("")` writes an empty
    /// label, which is a distinct authored state the schema allows on purpose (no `minLength`) and
    /// which the mod reads as "fall back to type + id". Both are reachable, deliberately.
    pub fn set_zone_label(&self, zone_id: &str, label: Option<&str>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) {
            if let Some(l) = label {
                zone.insert(&mut txn, "label", l);
            } else {
                zone.remove(&mut txn, "label");
            }
        }
    }

    /// T-211 — schema `zone.faction` (optional `factionKey` slug, e.g. `blufor`). `None` removes
    /// the key, which is how a zone becomes faction-neutral again.
    pub fn set_zone_faction(&self, zone_id: &str, faction: Option<&str>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) {
            if let Some(f) = faction {
                zone.insert(&mut txn, "faction", f);
            } else {
                zone.remove(&mut txn, "faction");
            }
        }
    }

    /// T-211 — schema `zone.rules` (optional `$defs/zoneRules`). Takes the object as JSON and
    /// stores it opaquely; see the block comment above for why this is not a typed struct.
    ///
    /// `None`, malformed JSON, a non-object, and `{}` all REMOVE the key. That last one matters:
    /// `rules` is optional as a whole and every key defaults, so "the author cleared every rule"
    /// and "the author never opened the panel" are the same document — writing `"rules": {}` would
    /// invent a third state the schema and the mod cannot tell apart.
    pub fn set_zone_rules(&self, zone_id: &str, rules_json: Option<&str>) {
        let mut txn = self.begin();
        let Some(Out::YMap(zone)) = self.zones.get(&txn, zone_id) else {
            return;
        };
        let parsed = rules_json.map(json_str_to_any);
        match parsed {
            Some(Any::Map(m)) if !m.is_empty() => {
                zone.insert(&mut txn, "rules", Any::Map(m));
            }
            _ => {
                zone.remove(&mut txn, "rules");
            }
        }
    }

    /// T-211 — delete an authored zone row.
    pub fn remove_zone(&self, zone_id: &str) {
        let mut txn = self.begin();
        self.zones.remove(&mut txn, zone_id);
    }

    /// T-211 — the `zones` root as a JSON object (`zonesById`), for the draw tool's render read.
    /// `small_maps_json` carries the same map; this is the narrow getter that does not pay for the
    /// other nine.
    #[must_use]
    pub fn zones_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.zones.to_json(&txn).to_json(&mut buf);
        buf
    }

    /// T-211 — authored zone count (cheap; backs "does this mission define a play area?").
    #[must_use]
    pub fn zone_count(&self) -> usize {
        self.zones.len(&self.doc.transact()) as usize
    }

    // ── T-650 — saved compositions (`compositionsById`) ─────────────────────────────────────────
    //
    // ROW SHAPE (SELF-CONTAINED — see the `compositions` field's ROUTING note):
    //
    //   id         String, required — the doc key.
    //   title      String — the author-facing name (ATTR-FIELD-COMP-TITLE).
    //   author     String — the current user's display string, as-authored (ATTR-FIELD-COMP-AUTHOR;
    //              no server assignment in this framing).
    //   category   String — grouping label (ATTR-FIELD-COMP-CATEGORY).
    //   entities   Array  — the captured selection as RELATIVE-OFFSET entries. Each entry is one of:
    //                • {kind:"slot",   dx, dz, rotation, role, tag, assetId, stance, loadout?}
    //                • {kind:"vehicle",dx, dz, rotation, resourceName, crewed?, crew?}
    //                • {kind:"object", dx, dz, rotation, alias, resourceName, faction}
    //              `dx`/`dz` are metres from the capture centroid; place re-anchors the centroid at
    //              the drop point (the `paste_at_cursor` centroid→cursor rule). `crew` is the SHAPE
    //              (`{seat_id: slot_id}`) captured verbatim — the slot ids are stale on placement
    //              (the boarded bodies are not re-created here), so the placer treats `crew` as an
    //              opaque record and does not re-board; it survives for a later crew-aware placer.
    //
    // WHY THE WHOLE ROW IS OPAQUE JSON (not typed sub-maps). Like `set_zone_rules` /
    // `update_slot_loadout`, the row is stored via [`json_str_to_any`] and written whole. A typed
    // mirror would be a second declaration of the entry vocabulary that could drift from the
    // authoring surface; keeping it opaque is exactly what makes the mechanical lift to a
    // user-scoped API row a *move*, not a re-encode. The mutators below therefore do whole-`Any`
    // read-modify-write for field edits (the crew idiom), so a rename after a hydrate — where the
    // row is an opaque `Any::Map`, not a tracked `YMap` — is sound.

    /// T-650 (COMP-SAVE-001) — store one composition row from its self-contained JSON. Malformed
    /// JSON or a non-object is refused (no row written), like a malformed zone-rules edit. The `id`
    /// is the doc key and is forced onto the stored object so the row is addressable regardless of
    /// what the caller put in the JSON's own `id`.
    pub fn add_composition(&self, id: &str, row_json: &str) {
        let Any::Map(fields) = json_str_to_any(row_json) else {
            return;
        };
        let mut map: HashMap<String, Any> = (*fields).clone();
        map.insert("id".to_string(), Any::String(id.into()));
        let mut txn = self.begin();
        self.compositions
            .insert(&mut txn, id, Any::Map(Arc::new(map)));
    }

    /// T-650 (COMP-EDIT-001 / ATTR-FIELD-COMP-TITLE) — rename a composition. Whole-`Any`
    /// read-modify-write (the crew idiom) so it is hydrate-proof. No-op when the id is absent.
    pub fn set_composition_title(&self, id: &str, title: &str) {
        self.set_composition_field(id, "title", title);
    }

    /// T-650 (COMP-EDIT-001 / ATTR-FIELD-COMP-CATEGORY) — recategorize a composition. Same idiom.
    pub fn set_composition_category(&self, id: &str, category: &str) {
        self.set_composition_field(id, "category", category);
    }

    /// T-650 (ATTR-FIELD-COMP-AUTHOR) — set the author display string. Author is normally stamped
    /// once at save, but exposed as an edit for completeness (the three metadata fields are the
    /// row's own title/author/category, all editable via the inline edit).
    pub fn set_composition_author(&self, id: &str, author: &str) {
        self.set_composition_field(id, "author", author);
    }

    /// Shared whole-`Any` field write for the three metadata edits. Reads the row entire (tolerant
    /// of BOTH the freshly-authored `YMap` and the post-hydrate opaque `Any::Map`, via
    /// [`read_composition_map`]), sets the one string key, writes it back whole — so a metadata edit
    /// on a reloaded mission behaves exactly like one on a freshly-saved doc and does not wipe
    /// `entities`.
    fn set_composition_field(&self, id: &str, key: &str, value: &str) {
        let mut txn = self.begin();
        let Some(mut row) = read_composition_map(&txn, &self.compositions, id) else {
            return;
        };
        row.insert(key.to_string(), Any::String(value.into()));
        self.compositions
            .insert(&mut txn, id, Any::Map(Arc::new(row)));
    }

    /// T-650 (COMP-EDIT-001) — delete a saved composition row.
    pub fn remove_composition(&self, id: &str) {
        let mut txn = self.begin();
        self.compositions.remove(&mut txn, id);
    }

    /// T-650 — the `compositions` root as a JSON object (`compositionsById`), for the palette's
    /// list read. `small_maps_json` carries the same map; this is the narrow getter.
    #[must_use]
    pub fn compositions_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.compositions.to_json(&txn).to_json(&mut buf);
        buf
    }

    /// T-650 — saved-composition count (cheap; backs the palette header count).
    #[must_use]
    pub fn composition_count(&self) -> usize {
        self.compositions.len(&self.doc.transact()) as usize
    }

    // ── T-079 — triggers, the editor half (`triggersById`) ──────────────────────────────────────
    //
    // ROW SHAPE + ROUTING: see the `triggers` field's SPLIT NOTE. The row is `#/$defs/zone`-shaped
    // for `shape` + `rules`, plus the trigger-only `name` / `ownerId` / `activation`. The geometry
    // setters call the SAME `circle_shape_any` / `polygon_shape_any` the zone mutators do — that
    // shared geometry is the "trigger area is a SECOND CONSUMER of the zone draw tool" contract at
    // the doc layer: there is no second circle/polygon encoding for triggers to drift from.
    //
    // WHY THESE MUTATORS DO NOT RANGE-CHECK OR TYPE-NARROW — the same reasoning as the T-211 zone
    // block above: `rules` is stored opaque and whole (the closed `$defs/zoneRules` vocabulary is the
    // single validator, T-241), and `$defs/circle.r` / `$defs/polygon` bounds are the schema's to
    // enforce, not re-encoded here. The draw tool's `radius_survives_compile` / `polygon_is_committable`
    // guards (shared with zones) keep a degenerate shape from being COMMITTED in the first place.

    /// T-079 — create a circle trigger. `x`/`z` are world metres, `r` the radius; `activation` is the
    /// stored-not-evaluated activation kind (`presence`/`radio`/`timer`), written verbatim. Mirrors
    /// [`Self::add_circle_zone`], with the trigger-only `activation` key added.
    pub fn add_circle_trigger(&self, id: &str, activation: &str, x: f64, z: f64, r: f64) {
        let mut txn = self.begin();
        let t = self
            .triggers
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        t.insert(&mut txn, "activation", activation);
        t.insert(&mut txn, "shape", circle_shape_any(x, z, r));
    }

    /// T-079 — create a polygon trigger from a FLAT `[x0,z0,x1,z1,…]` ring (the wasm-boundary shape,
    /// exactly [`Self::add_polygon_zone`]). A trailing unpaired coordinate is dropped.
    pub fn add_polygon_trigger(&self, id: &str, activation: &str, points_flat: &[f64]) {
        let mut txn = self.begin();
        let t = self
            .triggers
            .insert(&mut txn, id, MapPrelim::from([("id", id)]));
        t.insert(&mut txn, "activation", activation);
        t.insert(&mut txn, "shape", polygon_shape_any(points_flat));
    }

    /// T-079 — reshape a trigger to a circle. Whole-`shape` replacement (the `oneOf` reason from
    /// [`Self::set_zone_circle`]), so `name` / `ownerId` / `activation` / `rules` survive.
    pub fn set_trigger_circle(&self, trigger_id: &str, x: f64, z: f64, r: f64) {
        let mut txn = self.begin();
        if let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) {
            t.insert(&mut txn, "shape", circle_shape_any(x, z, r));
        }
    }

    /// T-079 — reshape a trigger to a polygon. Whole-`shape` replacement, as [`Self::set_zone_polygon`].
    pub fn set_trigger_polygon(&self, trigger_id: &str, points_flat: &[f64]) {
        let mut txn = self.begin();
        if let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) {
            t.insert(&mut txn, "shape", polygon_shape_any(points_flat));
        }
    }

    /// T-079 — the trigger's author-facing `name`. `None` REMOVES the key; `Some("")` is refused-empty
    /// upstream by the panel (which sends `None` on an emptied box), mirroring [`Self::set_zone_label`]'s
    /// two reachable states without inventing a third for the mod.
    pub fn set_trigger_name(&self, trigger_id: &str, name: Option<&str>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) {
            if let Some(n) = name {
                t.insert(&mut txn, "name", n);
            } else {
                t.remove(&mut txn, "name");
            }
        }
    }

    /// T-079 (CONN-TRG-OWNER-001) — the owner link, the DATA EDGE. `Some(entity_id)` records the
    /// placed slot/vehicle this trigger belongs to; `None` clears it (unowned). No referential check:
    /// the owner map is another slice's and an owner can be deleted after assignment — the edge is
    /// allowed to DANGLE and every reader tolerates a `ownerId` that resolves to nothing (see the
    /// field SPLIT NOTE). This is deliberately NOT the drag-connect gesture (that is T-672's
    /// CONN-START-001) — it is a plain id write the picker drives.
    pub fn set_trigger_owner(&self, trigger_id: &str, owner_id: Option<&str>) {
        let mut txn = self.begin();
        if let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) {
            if let Some(o) = owner_id {
                t.insert(&mut txn, "ownerId", o);
            } else {
                t.remove(&mut txn, "ownerId");
            }
        }
    }

    /// T-079 — the stored-not-evaluated `activation` kind (`presence`/`radio`/`timer`). Written
    /// verbatim; the runtime that acts on it is T-676. The panel offers only the three the ticket
    /// names, so a value outside them can only come from a hand-authored payload — kept as-is (opaque)
    /// rather than clamped, matching the doc layer's write-faithfully / schema-rejects discipline.
    pub fn set_trigger_activation(&self, trigger_id: &str, activation: &str) {
        let mut txn = self.begin();
        if let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) {
            t.insert(&mut txn, "activation", activation);
        }
    }

    /// T-079 — the trigger's `rules` object (`$defs/zoneRules`). Stored OPAQUE and whole, identical to
    /// [`Self::set_zone_rules`]: `None` / malformed / non-object / `{}` all REMOVE the key (the
    /// "cleared every rule" == "never authored" identity), so triggers reuse the exact zone-rules
    /// storage discipline and the schema stays the single vocabulary.
    pub fn set_trigger_rules(&self, trigger_id: &str, rules_json: Option<&str>) {
        let mut txn = self.begin();
        let Some(Out::YMap(t)) = self.triggers.get(&txn, trigger_id) else {
            return;
        };
        match rules_json.map(json_str_to_any) {
            Some(Any::Map(m)) if !m.is_empty() => {
                t.insert(&mut txn, "rules", Any::Map(m));
            }
            _ => {
                t.remove(&mut txn, "rules");
            }
        }
    }

    /// T-079 — delete a trigger row.
    pub fn remove_trigger(&self, trigger_id: &str) {
        let mut txn = self.begin();
        self.triggers.remove(&mut txn, trigger_id);
    }

    /// T-079 — the `triggers` root as a JSON object (`triggersById`), for the palette's render read.
    /// `small_maps_json` carries the same map; this is the narrow getter (the [`Self::zones_json`] twin).
    #[must_use]
    pub fn triggers_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.triggers.to_json(&txn).to_json(&mut buf);
        buf
    }

    /// T-079 — authored trigger count (cheap; backs the palette header count).
    #[must_use]
    pub fn trigger_count(&self) -> usize {
        self.triggers.len(&self.doc.transact()) as usize
    }

    // ── T-651 — editor comments / annotations (`commentsById`) ───────────────────────────────────
    //
    // ROW SHAPE + ROUTING + the corpus weighting: see the `comments` field's note. The one thing
    // worth repeating here, because it is the constraint that keeps this collection safe, is that a
    // comment NEVER COMPILES: `mission::flatten::EditorPayload` declares no `comments` key, so serde
    // drops the array before the mod document exists. Nothing in this block filters anything — the
    // exclusion is the ABSENCE of a declaration in another file, which is why the test that guards
    // it (`comments_never_reach_the_mod_document`) works by writing a comment into a root that IS
    // compiled and watching the assertion fire, not by re-reading this code.
    //
    // WHY THESE MUTATORS DO NOT VALIDATE. `title` / `tooltip` are stored VERBATIM, uncapped and
    // untrimmed, for the `set_faction_briefing_marker` reason: this text reaches no consumer that
    // could reject it, so there is nothing to normalise FOR, and destroying an authored value in the
    // one place the author can still see it is the worse failure. Positions are not clamped either —
    // an off-terrain annotation is legal (Eden lets you park a note in the sea) and, unlike a slot,
    // it cannot desync a mod.
    //
    // The row is written as a whole opaque `Any::Map` (the composition idiom) and the field edits do
    // a whole-row read-modify-write via [`read_comment_map`], so an edit AFTER a hydrate — where the
    // row materialises as an opaque `Any::Map` rather than a tracked `YMap` — behaves identically to
    // one on a freshly-placed comment.

    /// T-651 (`PLACE-COMMENT-001`) — place a comment at `(x, z)` world metres with `title`
    /// (ATTR-FIELD-CMT-TITLE) and `tooltip` (ATTR-FIELD-CMT-TOOLTIP). The id is the doc key.
    /// Overwrites an existing row with the same id (upsert), like every other by-id add here.
    pub fn add_comment(&self, id: &str, title: &str, tooltip: &str, x: f64, z: f64) {
        let mut txn = self.begin();
        self.comments.insert(
            &mut txn,
            id,
            Any::Map(Arc::new(comment_row(id, title, tooltip, x, z))),
        );
    }

    /// T-651 (ATTR-FIELD-CMT-TITLE) — retitle a comment. No-op on an unknown id.
    pub fn set_comment_title(&self, id: &str, title: &str) {
        self.set_comment_field(id, "title", Any::String(title.into()));
    }

    /// T-651 (ATTR-FIELD-CMT-TOOLTIP) — rewrite a comment's tooltip body. No-op on an unknown id.
    pub fn set_comment_tooltip(&self, id: &str, tooltip: &str) {
        self.set_comment_field(id, "tooltip", Any::String(tooltip.into()));
    }

    /// T-651 (ATTR-FIELD-CMT-POSITION) — move a comment to `(x, z)` world metres. This is the DRAG
    /// commit: one transaction ⇒ one undo step, so a drag is one Ctrl+Z exactly like a slot move.
    ///
    /// Deliberately NOT gated by [`Self::slot_layer_is_locked`]. A locked layer is a TRANSFORM lock
    /// on mission geometry — it exists so a finished ORBAT cannot be nudged. A comment is not
    /// mission geometry and cannot desync anything by moving, so silently refusing to drag the note
    /// that explains a locked layer would be an obstruction with no safety behind it. Stated here
    /// because the omission is otherwise indistinguishable from an oversight.
    pub fn set_comment_position(&self, id: &str, x: f64, z: f64) {
        let mut pos: HashMap<String, Any> = HashMap::new();
        pos.insert("x".to_string(), Any::Number(x));
        pos.insert("z".to_string(), Any::Number(z));
        self.set_comment_field(id, "position", Any::Map(Arc::new(pos)));
    }

    /// Shared whole-row read-modify-write for the three field edits (the composition idiom, so a
    /// post-hydrate opaque row edits exactly like a freshly-placed one). No-op on an unknown id.
    fn set_comment_field(&self, id: &str, key: &str, value: Any) {
        let mut txn = self.begin();
        let Some(mut row) = read_comment_map(&txn, &self.comments, id) else {
            return;
        };
        row.insert(key.to_string(), value);
        self.comments.insert(&mut txn, id, Any::Map(Arc::new(row)));
    }

    /// T-651 — COPY a comment: duplicate `src_id` as `new_id`, offset by `(dx, dz)` metres, keeping
    /// title and tooltip verbatim. Returns `false` (writing nothing) when `src_id` is unknown.
    ///
    /// A comment's copy is its own mutator rather than a branch of the clipboard because the
    /// clipboard lane (`editor_ops::copy_selection` → `paste_at_cursor`) is slot-shaped end to end:
    /// it snapshots `slots_json` rows and replays them through `paste_slots`' parallel role / tag /
    /// asset / stance / loadout arrays. A comment has none of those fields, so joining that lane
    /// would mean widening the paste ABI for a row that shares no column with it. The duplicate is
    /// one transaction ⇒ one undo step, which is the property that actually matters.
    pub fn duplicate_comment(&self, src_id: &str, new_id: &str, dx: f64, dz: f64) -> bool {
        let mut txn = self.begin();
        let Some(row) = read_comment_map(&txn, &self.comments, src_id) else {
            return false;
        };
        let (x, z) = comment_xz(&row);
        let title = comment_str(&row, "title");
        let tooltip = comment_str(&row, "tooltip");
        self.comments.insert(
            &mut txn,
            new_id,
            Any::Map(Arc::new(comment_row(
                new_id,
                &title,
                &tooltip,
                x + dx,
                z + dz,
            ))),
        );
        true
    }

    /// T-651 — delete a comment row.
    pub fn remove_comment(&self, id: &str) {
        let mut txn = self.begin();
        self.comments.remove(&mut txn, id);
        remove_id_from_all_layers(&mut txn, &self.editor_layers, id);
    }

    /// T-651 — file a comment into an Outliner folder. This is "comments support LAYERS", and it is
    /// literally [`Self::move_slot_to_layer`]: that mutator only ever moves an ID between
    /// `editorLayers[].entityIds` arrays and never reads the slots map, so a comment id files exactly
    /// like a slot id and `build_outliner` resolves it the same way. Delegating rather than copying
    /// is the point — one filing mechanism means a comment cannot end up half-filed by a future edit
    /// to only one of two implementations.
    pub fn move_comment_to_layer(&self, comment_id: &str, layer_id: &str) {
        self.move_slot_to_layer(comment_id, layer_id);
    }

    /// T-651 — the `comments` root as a JSON object (`commentsById`), for the outliner's row read.
    /// `small_maps_json` carries the same map; this is the narrow getter.
    #[must_use]
    pub fn comments_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.comments.to_json(&txn).to_json(&mut buf);
        buf
    }

    /// T-651 — placed-comment count (cheap).
    #[must_use]
    pub fn comment_count(&self) -> usize {
        self.comments.len(&self.doc.transact()) as usize
    }

    /// T-651 — **the new-mission template's comments.** Seeds exactly TWO annotations into an empty
    /// doc, then no-ops forever (any existing comment ⇒ this is not a new mission). Returns the ids
    /// it wrote, empty when it declined.
    ///
    /// Two, and the number is the evidence rather than a guess. FNF v4 deleted the 219-line
    /// `configGuide.txt` and the whole 421-file template, and the onboarding that survived that
    /// rewrite is literally two Comment entities seeded into `mission.sqm`. FNF v3 had 28 in-map
    /// comments including a seven-paragraph tutorial; almost none of it survived. So the seed copies
    /// what SURVIVED, not what once existed. **This is one community across two eras — WOG and OFCRA
    /// have no comment equivalent at all** — which is the other reason to seed two and not twenty.
    ///
    /// Callers must bracket this with `set_origin_init(true)` so a template is not an undo step
    /// (the boot/seed contract of [`Self::set_origin_init`]); the editor's boot does.
    ///
    /// Positions are terrain-centre-ish (Everon is 12.8 km square) and stacked 200 m apart so the
    /// two rows are distinguishable the moment the outliner paints.
    pub fn seed_template_comments(&self) -> Vec<String> {
        if self.comment_count() > 0 {
            return Vec::new();
        }
        let seeds: [(&str, &str, &str, f64, f64); 2] = [
            (
                "comment-template-1",
                "Start here",
                "Place your ORBAT first: right-click the map to add units, then drag them into \
                 folders in the Outliner. Delete this note when you no longer need it — comments \
                 are editor-only and never reach the compiled mission.",
                6_400.0,
                6_500.0,
            ),
            (
                "comment-template-2",
                "Mission notes",
                "Use comments for anything the mission file cannot carry: intent, timings, \
                 reminders for the next editor. Right-click empty ground and choose Place Comment \
                 to add another.",
                6_400.0,
                6_300.0,
            ),
        ];
        let mut ids = Vec::with_capacity(seeds.len());
        for (id, title, tooltip, x, z) in seeds {
            self.add_comment(id, title, tooltip, x, z);
            ids.push(id.to_string());
        }
        ids
    }

    // ── T-672 — the editor-only CONNECTION GRAPH (`connectionsById`) ─────────────────────────────
    //
    // ROW SHAPE + ROUTING + the never-compiles proof: see the `connections` field's note.
    //
    // **READ THIS FIRST — the order these three surfaces ship in is the ticket.** The framework
    // corpus records FNF v4's entire defect cluster on this mechanism, and the instruction that came
    // with it is "the inspector and the validation rules must precede the edges — do not ship edges
    // you cannot see or check". So this block is written SEE → CHECK → WRITE:
    //
    //   SEE    [`Self::connection_rows_json`] — every edge, stable-ordered, addressable by id.
    //   CHECK  [`Self::connection_findings_json`] — four rules over the WHOLE graph
    //          (self-link, dangling endpoint, duplicate edge, cycle in the directed subgraph).
    //   WRITE  [`Self::add_connection`] / [`Self::remove_connection`] /
    //          [`Self::remove_connections_touching`].
    //
    // WHY BOTH A REFUSAL AND A FINDING FOR THE SAME CONDITION. `add_connection` refuses a self-link,
    // an unknown kind and a duplicate, writing nothing. That keeps THIS editor's authoring clean and
    // nothing more: [`Self::hydrate`] uses the generic ordered row loader every sibling collection
    // uses, so a payload authored by another tool (or by an older build of this one) can land any of
    // those shapes plus a dangling endpoint the mutator cannot even see at write time — an endpoint
    // is only dangling relative to a document state that changes after the edge is drawn. Refusing
    // rows at LOAD would silently destroy an operator's data. Showing them as findings is the honest
    // answer, and it is why the checker is not merely a re-statement of the mutator's guards.
    //
    // WHY THE MUTATORS DO NOT REPAIR. Nothing here deletes a dangling edge, dedupes, or breaks a
    // cycle. A relation the author drew is data; the graph's job is to tell the truth about it, and
    // the operator's job is to decide. The one exception is
    // [`Self::remove_connections_touching`], which is a CASCADE, not a repair: deleting an entity
    // is an explicit destructive act, and leaving its edges behind would manufacture the dangling
    // rows the checker exists to report.

    /// T-672 (SEE) — every connection as a stable-ordered JSON array of
    /// `{id, kind, from, to}`.
    ///
    /// **This is the inspector feed, and it is the half of this ticket that must never regress.**
    /// A connection has no map glyph in this slice (see the `LaneRole::SquadLinks` trace in the
    /// slice notes), so this listing is the ONLY way an operator can observe the graph they are
    /// authoring. `connections_json` returns the raw by-id map, whose iteration order is
    /// `serde_json`'s; a list that reshuffled between reads would make the rows dance under the
    /// cursor for a document that never changed, and would make "delete the third one" a lie.
    ///
    /// Sort key is the full tuple `(kind, from, to, id)` — total, so the order is a function of the
    /// CONTENT and two documents with the same edges list them identically regardless of the order
    /// they were drawn in. Rows whose `id` field disagrees with their doc key, or which are missing
    /// `from`/`to`, are still listed (keyed by the doc key, missing fields as `""`) rather than
    /// skipped: a malformed row is exactly what the operator needs to SEE, and
    /// [`Self::connection_findings_json`] will flag it dangling.
    #[must_use]
    pub fn connection_rows_json(&self) -> String {
        let rows = self.connection_rows();
        let out: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "kind": r.kind,
                    "from": r.from,
                    "to": r.to,
                })
            })
            .collect();
        serde_json::Value::Array(out).to_string()
    }

    /// T-672 (CHECK) — validate the whole graph; a stable-ordered JSON array of
    /// `{code, connectionId, detail}`.
    ///
    /// The four rules, and why each one is a rule:
    ///
    /// * **`CONN-SELF`** — `from == to`. An entity connected to itself is meaningless under every
    ///   one of the three kinds and is the classic off-by-one of a two-click connect gesture that
    ///   forgot to advance its target.
    /// * **`CONN-DANGLING`** — an endpoint id that is in no slot / entity / vehicle / zone / trigger
    ///   map. This is the rule that cannot be enforced at write time (an endpoint becomes dangling
    ///   when the OTHER end is deleted, long after the edge was drawn), which is why the delete
    ///   cascade in [`Self::remove_connections_touching`] exists and why this check still exists
    ///   beside it — the cascade cannot reach an edge that arrived by hydrate.
    /// * **`CONN-DUPLICATE`** — two rows with the same `(kind, from, to)` after `sync` normalisation.
    ///   Reported on the SECOND and later rows in listing order, so the finding names the row to
    ///   delete and the survivor is deterministic.
    /// * **`CONN-CYCLE`** — an edge that closes a cycle in the DIRECTED subgraph (`group` /
    ///   `triggerOwner`). Ownership must be a DAG: `A groups to B groups to A` has no leader, and a
    ///   trigger-owner cycle has no owner. `sync` is deliberately EXCLUDED — it is an undirected
    ///   peer relation, so a "cycle" in it is just a connected component and is perfectly legal.
    ///
    /// Findings are ordered by `(code, connectionId)` for the same reason the rows are: a panel that
    /// reorders its own warnings between reads is unreadable.
    #[must_use]
    pub fn connection_findings_json(&self) -> String {
        let rows = self.connection_rows();
        let known = self.known_endpoint_ids();
        let out: Vec<serde_json::Value> = validate_connection_rows(&rows, &known)
            .into_iter()
            .map(|f| {
                serde_json::json!({
                    "code": f.code,
                    "connectionId": f.connection_id,
                    "detail": f.detail,
                })
            })
            .collect();
        serde_json::Value::Array(out).to_string()
    }

    /// T-672 — the `connections` root as a JSON object (`connectionsById`), the narrow raw getter.
    /// Prefer [`Self::connection_rows_json`] for anything an operator reads: this one's key order is
    /// unspecified.
    #[must_use]
    pub fn connections_json(&self) -> String {
        let txn = self.doc.transact();
        let mut buf = String::new();
        self.connections.to_json(&txn).to_json(&mut buf);
        buf
    }

    /// T-672 — drawn-edge count (cheap).
    #[must_use]
    pub fn connection_count(&self) -> usize {
        self.connections.len(&self.doc.transact()) as usize
    }

    /// T-672 (`CONN-START-001` / `CONN-SYNC-001`) — **draw an edge.** Returns `false` writing
    /// NOTHING when the row would be junk:
    ///
    /// * `id`, `from` or `to` empty — an unaddressable or endpoint-less edge.
    /// * `kind` not one of `sync` / `group` / `triggerOwner` ([`ConnectionKind::parse`]).
    /// * `from == to` — a self-link (`CONN-SELF`).
    /// * an existing row already has the same `(kind, from, to)` after normalisation
    ///   (`CONN-DUPLICATE`). Drawing the same relation twice is not an edit; it is a second row the
    ///   operator now has to find and delete.
    ///
    /// Cycles are NOT refused here. A cycle is a property of the graph, not of the edge, and the
    /// edge that closes it is rarely the wrong one — refusing the last click would blame the wrong
    /// gesture. `CONN-CYCLE` is a finding so the operator can see it and pick which edge to remove.
    ///
    /// `sync` endpoints are sorted before the write (see the field note): `sync(B,A)` stores as
    /// `sync(A,B)`, so the duplicate guard above catches the reversed re-draw that a naive verbatim
    /// store would let through as a second edge.
    ///
    /// Upserts by `id` like every other by-id add here — but note the duplicate guard runs FIRST, so
    /// re-adding the same relation under a NEW id is refused rather than silently doubled.
    pub fn add_connection(&self, id: &str, kind: &str, from: &str, to: &str) -> bool {
        if id.is_empty() || from.is_empty() || to.is_empty() {
            return false;
        }
        let Some(kind) = ConnectionKind::parse(kind) else {
            return false;
        };
        if from == to {
            return false;
        }
        let (from, to) = kind.normalise(from, to);
        if self
            .connection_rows()
            .iter()
            .any(|r| r.id != id && r.kind == kind.as_str() && r.from == from && r.to == to)
        {
            return false;
        }
        let mut txn = self.begin();
        self.connections.insert(
            &mut txn,
            id,
            Any::Map(Arc::new(connection_row(id, kind.as_str(), &from, &to))),
        );
        true
    }

    /// T-672 (`CONN-DEL-001`) — delete one edge by id. One transaction ⇒ one Ctrl+Z, which is the
    /// only recovery path a relation with no map glyph has.
    pub fn remove_connection(&self, id: &str) {
        let mut txn = self.begin();
        self.connections.remove(&mut txn, id);
    }

    /// T-672 — **the delete CASCADE**: drop every edge with `entity_id` at either end, in ONE
    /// transaction, and return the ids removed.
    ///
    /// Called when the entity itself is deleted. This is not the checker being enforced — it is the
    /// difference between a delete that finishes and one that manufactures `CONN-DANGLING` rows for
    /// the operator to clean up by hand. One transaction so the entity delete and its edge removals
    /// undo together; several mutators would be several undo steps
    /// (`capture_timeout_millis = 0` makes every txn its own step), and a Ctrl+Z that restored the
    /// unit but not its connections would be a half-applied undo.
    pub fn remove_connections_touching(&self, entity_id: &str) -> Vec<String> {
        if entity_id.is_empty() {
            return Vec::new();
        }
        let doomed: Vec<String> = self
            .connection_rows()
            .into_iter()
            .filter(|r| r.from == entity_id || r.to == entity_id)
            .map(|r| r.id)
            .collect();
        if doomed.is_empty() {
            return Vec::new();
        }
        let mut txn = self.begin();
        for id in &doomed {
            self.connections.remove(&mut txn, id.as_str());
        }
        doomed
    }

    /// T-672 — the connection rows as owned structs, sorted by `(kind, from, to, id)`. The single
    /// read every surface above goes through, so the listing, the checker and the duplicate guard
    /// cannot disagree about what the graph contains or what order it is in.
    #[must_use]
    fn connection_rows(&self) -> Vec<ConnectionRow> {
        let txn = self.doc.transact();
        let mut rows: Vec<ConnectionRow> = self
            .connections
            .iter(&txn)
            .filter_map(|(key, _)| {
                let row = read_connection_map(&txn, &self.connections, key)?;
                Some(ConnectionRow {
                    id: key.to_string(),
                    kind: comment_str(&row, "kind"),
                    from: comment_str(&row, "from"),
                    to: comment_str(&row, "to"),
                })
            })
            .collect();
        rows.sort_by(|a, b| {
            (&a.kind, &a.from, &a.to, &a.id).cmp(&(&b.kind, &b.from, &b.to, &b.id))
        });
        rows
    }

    /// T-672 — every id an edge is allowed to point AT: slots, entities (objects), vehicles, zones
    /// and triggers. The `CONN-DANGLING` universe.
    ///
    /// Comments are deliberately absent. A comment is an editor-only annotation with no presence in
    /// the compiled mission; syncing a unit to a sticky note is not a relation the mission can
    /// express, so an edge pointing at one is dangling by construction and should read as such.
    #[must_use]
    fn known_endpoint_ids(&self) -> HashSet<String> {
        let txn = self.doc.transact();
        let mut out = HashSet::new();
        for map in [
            &self.slots,
            &self.entities,
            &self.vehicles,
            &self.zones,
            &self.triggers,
        ] {
            for (k, _) in map.iter(&txn) {
                out.insert(k.to_string());
            }
        }
        out
    }

    // ── T-672 — `ACTION-FORM-001` / `CTX-FORMATION-001`: force a squad to formation ──────────────

    /// T-672 (`ACTION-FORM-001` `ForceToFormation`) — snap every member of the squad that owns
    /// `leader_slot_id` onto its formation position, in ONE transaction. Returns the number of slots
    /// moved (0 when the leader is unknown, unfiled, or alone).
    ///
    /// **The leader does not move.** Eden's `ForceToFormation` re-forms the group AROUND its leader;
    /// moving the leader too would translate the whole squad and make the action a
    /// nobody-asked-for reposition. So the leader is the anchor and the members take the offsets.
    ///
    /// Offsets come from [`formation_offsets`], which is a pure function and is where the geometry
    /// is tested. Heading is the LEADER's `position.rotation` in degrees, so a squad re-formed after
    /// the leader turns faces the way the leader faces — a formation that always pointed north would
    /// be wrong the moment the operator rotated anything.
    ///
    /// `y` (elevation) is carried from each member's CURRENT value rather than resampled: this crate
    /// has no DEM (the caller samples it — see [`Self::move_entities`]'s `zs`), and inventing a 0.0
    /// would drop every re-formed unit to sea level. A formation snap is a horizontal action; the
    /// vertical stays the operator's/DEM's business.
    ///
    /// One transaction ⇒ one Ctrl+Z, which matters here more than usual: this action moves several
    /// units at once and an operator who dislikes the result must get all of them back in one press.
    pub fn force_to_formation(&self, leader_slot_id: &str, formation: &str) -> usize {
        let members = self.squad_members_of_leader(leader_slot_id);
        if members.is_empty() {
            return 0;
        }
        let mut txn = self.begin();
        let Some(Out::YMap(leader)) = self.slots.get(&txn, leader_slot_id) else {
            return 0;
        };
        let anchor = read_position_map(&txn, &leader);
        let px = |k: &str| match anchor.get(k) {
            Some(Any::Number(n)) => *n,
            #[allow(clippy::cast_precision_loss)] // world metres are far inside f64's exact range
            Some(Any::BigInt(i)) => *i as f64,
            _ => 0.0,
        };
        let (lx, ly, heading) = (px("x"), px("y"), px("rotation"));
        let offsets = formation_offsets(formation, members.len());
        let (sin_h, cos_h) = heading.to_radians().sin_cos();
        let mut moved = 0usize;
        for (member, (ox, oy)) in members.iter().zip(offsets) {
            let Some(Out::YMap(slot)) = self.slots.get(&txn, member.as_str()) else {
                continue;
            };
            let existing = read_position_map(&txn, &slot);
            // Rotate the body-frame offset into world space by the leader's heading. `+y` is the
            // formation's FORWARD axis, matching the heading convention `position.rotation` uses.
            let wx = lx + ox.mul_add(cos_h, oy * sin_h);
            let wy = ly + oy.mul_add(cos_h, -(ox * sin_h));
            let z = match existing.get("z") {
                Some(Any::Number(n)) => *n,
                #[allow(clippy::cast_precision_loss)]
                Some(Any::BigInt(i)) => *i as f64,
                _ => 0.0,
            };
            slot.insert(
                &mut txn,
                "position",
                position_any_merged(existing, wx, wy, z, heading),
            );
            moved += 1;
        }
        moved
    }

    /// T-672 — the non-leader members of the squad `leader_slot_id` leads, in `squad.slotIds` order.
    /// Empty when the slot leads no squad (only the DECLARED `leaderSlotId` counts — a formation
    /// action fired from a rifleman must do nothing rather than silently re-form the squad around
    /// him, which would be a leadership change nobody asked for).
    #[must_use]
    fn squad_members_of_leader(&self, leader_slot_id: &str) -> Vec<String> {
        if leader_slot_id.is_empty() {
            return Vec::new();
        }
        let txn = self.doc.transact();
        for (squad_id, out_v) in self.squads.iter(&txn) {
            let Out::YMap(sq) = out_v else { continue };
            if read_str(&txn, &sq, "leaderSlotId").as_deref() != Some(leader_slot_id) {
                continue;
            }
            return read_id_array(&txn, &self.squads, squad_id, "slotIds")
                .iter()
                .filter_map(|a| match a {
                    Any::String(s) if s.as_ref() != leader_slot_id => Some(s.to_string()),
                    _ => None,
                })
                .collect();
        }
        Vec::new()
    }

    /// T-650 (COMP-PLACE-001) — place a saved composition: stamp every captured entity onto the map
    /// at `(drop_x, drop_y)` as ONE undoable transaction. This is the multi-paste the ticket calls
    /// for — a `paste_at_cursor`-shaped drop, but writing slots AND vehicles AND objects, all in a
    /// single `begin()` so the whole placement is one Ctrl+Z (mixing several existing mutators would
    /// be several undo steps, because `capture_timeout_millis = 0` makes every txn its own step).
    ///
    /// Each entry's world position is `drop + (dx, dz)` — the RELATIVE-OFFSET entries are re-anchored
    /// so the composition's captured centroid lands under the cursor, clamped to the terrain
    /// `width`/`height` (the `paste_slots` clamp). `ids[i]` is the pre-minted id for `entities[i]`
    /// (minted by `editor_ops`, which proves uniqueness against the live doc); the caller passes as
    /// many ids as there are entities.
    ///
    /// SIDE OWNERSHIP mirrors the single-entity place: the side's `faction-{SIDE}` row is ensured
    /// in-txn (so faction creation is part of the same undo step), vehicles/objects record it, and
    /// slots file into `layer_id` UNFILED (`squadId` absent) — a composition is a set of loose
    /// entities, not a squad, which is the whole reason placing it does not stamp `squad.template`
    /// (see the TEMPLATE-COVERAGE note on this method). Returns the ids actually written (an unknown
    /// entity `kind`, or an id/entity count mismatch, skips that entry).
    ///
    /// **TEMPLATE-COVERAGE (T-657 forward constraint).** T-657's ORBAT-TEMPLATE-COVERAGE rule reads
    /// `squad.template.requiredRoles`. This placer writes NO squad — its slots are unfiled loose
    /// bodies — so there is no `squad.template` to stamp and the rule stays forward-compatible
    /// exactly as before. Were compositions ever to capture whole squads (with squad identity), the
    /// natural place to stamp the carried roles would be here; they do not, so nothing is forced.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn place_composition(
        &self,
        entities_json: &str,
        ids: &[String],
        side: &str,
        layer_id: &str,
        drop_x: f64,
        drop_y: f64,
        width: f64,
        height: f64,
    ) -> Vec<String> {
        let Any::Array(entities) = json_str_to_any(entities_json) else {
            return Vec::new();
        };
        let mut written = Vec::new();
        let mut txn = self.begin();

        // Ensure the side faction in THIS txn so faction creation joins the one undo step.
        let faction_id = format!("faction-{side}");
        if self.factions.get(&txn, &faction_id).is_none() {
            let f = self.factions.insert(
                &mut txn,
                faction_id.as_str(),
                MapPrelim::from([("id", faction_id.as_str())]),
            );
            f.insert(&mut txn, "key", side);
            f.insert(&mut txn, "name", side);
            f.insert(&mut txn, "squadIds", Any::Array(Vec::new().into()));
        }
        // Slots filed into `layer_id`'s `entityIds`, accumulated once (the T-059 O(k) shape).
        let mut layer_entities: Vec<Any> =
            read_id_array(&txn, &self.editor_layers, layer_id, "entityIds");

        let g_str = |m: &HashMap<String, Any>, k: &str| match m.get(k) {
            Some(Any::String(s)) => s.to_string(),
            _ => String::new(),
        };
        let g_num = |m: &HashMap<String, Any>, k: &str| m.get(k).map_or(0.0, any_to_f64);

        for (i, ent) in entities.iter().enumerate() {
            let Some(id) = ids.get(i) else { break };
            let Any::Map(fields) = ent else { continue };
            let kind = g_str(fields, "kind");
            let wx = (drop_x + g_num(fields, "dx")).clamp(0.0, width);
            let wy = (drop_y + g_num(fields, "dz")).clamp(0.0, height);
            let rot = g_num(fields, "rotation");
            match kind.as_str() {
                "slot" => {
                    let slot = self.slots.insert(
                        &mut txn,
                        id.as_str(),
                        MapPrelim::from([("id", id.as_str())]),
                    );
                    // Unfiled: no squadId (a loose body — see the method's TEMPLATE-COVERAGE note).
                    slot.insert(&mut txn, "index", Any::BigInt(0));
                    let role = g_str(fields, "role");
                    slot.insert(
                        &mut txn,
                        "role",
                        if role.is_empty() {
                            "Rifleman"
                        } else {
                            role.as_str()
                        },
                    );
                    let tag = g_str(fields, "tag");
                    if !tag.is_empty() {
                        slot.insert(&mut txn, "tag", tag.as_str());
                    }
                    let asset = g_str(fields, "assetId");
                    if !asset.is_empty() {
                        slot.insert(&mut txn, "assetId", asset.as_str());
                    }
                    let stance = g_str(fields, "stance");
                    slot.insert(
                        &mut txn,
                        "stance",
                        if stance.is_empty() {
                            "stand"
                        } else {
                            stance.as_str()
                        },
                    );
                    slot.insert(&mut txn, "loadoutId", Any::Null);
                    if let Some(l) = fields.get("loadout").filter(|l| !matches!(l, Any::Null)) {
                        slot.insert(&mut txn, "loadout", l.clone());
                    }
                    slot.insert(&mut txn, "position", position_any(wx, wy, 0.0, rot));
                    layer_entities.push(Any::String(id.as_str().into()));
                    written.push(id.clone());
                }
                "vehicle" => {
                    let resource = g_str(fields, "resourceName");
                    if resource.is_empty() {
                        continue;
                    }
                    let v = self.vehicles.insert(
                        &mut txn,
                        id.as_str(),
                        MapPrelim::from([("id", id.as_str())]),
                    );
                    v.insert(&mut txn, "resourceName", resource.as_str());
                    v.insert(&mut txn, "position", position_any(wx, wy, 0.0, rot));
                    v.insert(&mut txn, "factionId", faction_id.as_str());
                    // `crewed` omit idiom: only write `false` (the with-crew default is absence).
                    if fields.get("crewed") == Some(&Any::Bool(false)) {
                        v.insert(&mut txn, "crewed", false);
                    }
                    // T-650 — carry the crew SHAPE verbatim (opaque; the boarded slot ids are stale
                    // on a place and are NOT re-created here — a later crew-aware placer can act on
                    // it). Absent/empty leaves the row crew-free.
                    if let Some(Any::Map(crew)) = fields.get("crew")
                        && !crew.is_empty()
                    {
                        v.insert(&mut txn, "crew", Any::Map(crew.clone()));
                    }
                    written.push(id.clone());
                }
                "object" => {
                    let resource = g_str(fields, "resourceName");
                    let alias = g_str(fields, "alias");
                    if resource.is_empty() || alias.is_empty() {
                        continue;
                    }
                    let e = self.entities.insert(
                        &mut txn,
                        id.as_str(),
                        MapPrelim::from([("id", id.as_str())]),
                    );
                    e.insert(&mut txn, "alias", alias.as_str());
                    e.insert(&mut txn, "resourceName", resource.as_str());
                    e.insert(&mut txn, "position", position_any(wx, wy, 0.0, rot));
                    let faction = g_str(fields, "faction");
                    if !faction.is_empty() {
                        e.insert(&mut txn, "faction", faction.as_str());
                    }
                    written.push(id.clone());
                }
                _ => {} // unknown kind — skip, do not guess a row type
            }
        }
        // One write of the layer's `entityIds` (the T-059 shape), only if slots were filed.
        if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, layer_id) {
            layer.insert(&mut txn, "entityIds", Any::Array(layer_entities.into()));
        }
        written
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
        move_vehicles_in_txn(&mut txn, &self.vehicles, ids, dx, dy);
    }

    /// T-491 — move slots **and** vehicles in **one** LOCAL yrs transaction (one undo step).
    ///
    /// The T-425 host path called [`Self::move_entities`] then [`Self::move_vehicles`] — two txns —
    /// so a mixed slot+vehicle drag needed two Ctrl+Z. Prefer this for any drag that may include
    /// both kinds. Slot `zs[i]` matches [`Self::move_entities`]; vehicle z/rotation are preserved.
    ///
    /// T-574 — the example below is a **pin**, not decoration. `move_entities_and_vehicles_*` in
    /// this file's `tests` module is the primary behavioural proof, but it compiles under
    /// `--cfg test`, so a `#[cfg(not(test))]` twin of this function could gut production while the
    /// unit test exercised a `#[cfg(test)]` honest copy. A doctest links against the crate built
    /// **without** `--cfg test`, so it is the one check here that shape cannot hide from. `cargo
    /// test -p map-engine-core --features doc,mission` runs it (`Doc-tests map_engine_core`).
    ///
    /// ```
    /// use map_engine_core::doc::MissionDocCore;
    ///
    /// let doc = MissionDocCore::new();
    /// doc.set_origin_init(true);
    /// doc.add_slot("s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0);
    /// doc.add_vehicle("v0", "Prefab/Vehicle.et", Some(300.0), Some(400.0), Some(0.0), Some(45.0));
    /// doc.set_origin_init(false);
    ///
    /// doc.move_entities_and_vehicles(vec!["s0".into()], &["v0".into()], 10.0, 20.0, vec![1.5]);
    ///
    /// let slots = doc.materialize(); // one slot, so row 0 is s0
    /// assert_eq!((slots.xs[0], slots.ys[0], slots.zs[0]), (110.0, 220.0, 1.5), "the slot moved");
    /// assert_eq!(doc.vehicle_xy_flat(), vec![310.0, 420.0], "the vehicle moved too");
    /// assert_eq!(doc.undo_depth(), 1, "…and both inside ONE transaction");
    /// ```
    pub fn move_entities_and_vehicles(
        &self,
        slot_ids: Vec<String>,
        vehicle_ids: &[String],
        dx: f64,
        dy: f64,
        zs: Vec<f64>,
    ) {
        let mut txn = self.begin();
        move_entities_in_txn(
            &mut txn,
            &self.slots,
            &self.editor_layers,
            &slot_ids,
            dx,
            dy,
            &zs,
        );
        move_vehicles_in_txn(&mut txn, &self.vehicles, vehicle_ids, dx, dy);
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

    /// T-082 (ATTR-FIELD-OBJ-TYPE / ATTR-FIELD-OBJ-ROLE-DESC) — the two Attributes-modal OBJECT
    /// fields [`Self::update_slot`] does not carry: `assetId` (the entity **type**) and
    /// `description` (Eden's free-text "Role Description").
    ///
    /// **Why not [`Self::update_slot_role_character`]**, which already writes `assetId`: that one is
    /// the ORBAT *Apply* mutator. It takes `role` by value and rewrites it unconditionally, and it
    /// CLEARS `tag` whenever `tag` is `None` — correct there (the library row is authoritative on
    /// Apply), fatal here. The Attributes modal commits one field at a time and multi-edit passes
    /// `None` for every field the operator did not opt into, so routing a type edit through it would
    /// stamp the modal's snapshot of `role` back onto the row and wipe `tag`.
    ///
    /// `None` therefore means **leave this key exactly as it is** — the same discipline
    /// [`Self::update_slot`] uses, and the thing multi-edit's per-field opt-in depends on.
    /// `Some("")` CLEARS the key (absent ⇒ unset, the `tag`/`assetId` omit idiom of
    /// [`Self::add_slot`]); `Some(non-empty)` sets it. Both keys move in ONE transaction, so a
    /// commit is one undo step. No-op when the slot id is absent, or when both args are `None`.
    ///
    /// **`description` is editor-block state, never the MOD wire** — the same contract as
    /// `slot.tag` and `editorHidden`. It rides `editor.slots`, which
    /// `mission-editor-payload.schema.json` leaves deliberately unconstrained and
    /// [`Self::hydrate`]'s `load_rows` reloads verbatim, so it survives save/reload and copy/paste
    /// (`editor_ops::paste_at` carries unknown slot keys through `paste_slots`'s `extras`). It is
    /// structurally absent from the compiled document: `mission::flatten` deserializes into `SlotIn`,
    /// whose fixed field list omits it. `assetId` is the same editor-block field it has always been
    /// — `flatten` resolves it into the compiled `kit:` alias.
    pub fn update_slot_object(
        &self,
        id: &str,
        asset_id: Option<String>,
        description: Option<String>,
    ) {
        if asset_id.is_none() && description.is_none() {
            return; // nothing opted in — do not even open a transaction
        }
        let mut txn = self.begin();
        if let Some(Out::YMap(slot)) = self.slots.get(&txn, id) {
            for (key, val) in [("assetId", asset_id), ("description", description)] {
                let Some(v) = val else { continue };
                if v.is_empty() {
                    slot.remove(&mut txn, key);
                } else {
                    slot.insert(&mut txn, key, v);
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
        // T-665 — transform lock: a slot on a locked layer (or under a locked ancestor) refuses the
        // Attributes-tab position edit, the same silent refusal the drag path takes in
        // `move_entities_in_txn`. Guarded before the write so no `position` is rewritten.
        if slot_is_transform_locked(&txn, &self.editor_layers, id) {
            return;
        }
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
        move_entities_in_txn(
            &mut txn,
            &self.slots,
            &self.editor_layers,
            &ids,
            dx,
            dy,
            &zs,
        );
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
    /// content (same trim rule as title / featured briefing). Blank / whitespace is **not** a clear
    /// — use [`Self::clear_meta_briefing`] when the author deliberately emptied the field (T-766).
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

    /// T-766 — remove `meta.briefing` so a deliberate clear reaches `compile_export`.
    ///
    /// [`Self::apply_row_meta`] treats blank / whitespace briefing as "not supplied" so boot hydrate
    /// cannot wipe a good value with an empty row. That guard is load-bearing and stays. This mutator
    /// is the explicit "set to empty" arm the editor uses after a successful PATCH of
    /// `missions.briefing` to `""` — without it, same-session Export ships the deleted text.
    pub fn clear_meta_briefing(&self) {
        let mut txn = self.begin();
        self.meta.remove(&mut txn, "briefing");
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
            &self.zones,
            &self.compositions,
            &self.triggers,
            &self.comments,
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
        // T-211 — top-level `zones[]` into the `zones` root, ordered like every sibling. This is
        // also what takes `zones` OFF the `payloadExtras` parking path (see
        // `is_known_editor_payload_top_level`): the root map becomes the single source of truth,
        // and `small_maps_json` projects it back onto the wire.
        load_rows_ordered(
            &mut txn,
            &self.zones,
            payload.get("zones"),
            &entity_order,
            "zones",
        );
        // T-650 — top-level `compositions[]` into the `compositions` root, ordered like every
        // sibling. This is also what takes `compositions` OFF the `payloadExtras` parking path (it
        // is listed in [`is_known_editor_payload_top_level`]): the root map becomes the single
        // source of truth, and `small_maps_json` re-projects it onto the wire.
        load_rows_ordered(
            &mut txn,
            &self.compositions,
            payload.get("compositions"),
            &entity_order,
            "compositions",
        );
        // T-079 — top-level `triggers[]` into the `triggers` root, ordered like every sibling. As
        // with `zones` / `compositions`, this takes `triggers` OFF the `payloadExtras` parking path
        // (it is in [`is_known_editor_payload_top_level`]): the root map becomes the single source of
        // truth and `small_maps_json` re-projects it onto the wire until T-706 authors the key.
        load_rows_ordered(
            &mut txn,
            &self.triggers,
            payload.get("triggers"),
            &entity_order,
            "triggers",
        );
        // T-651 — top-level `comments[]` (promoted from `payloadExtras` by `compile_payload`) back
        // into the `comments` root, ordered like every sibling. Listing `comments` in
        // [`is_known_editor_payload_top_level`] is what keeps this off the parking path: the root map
        // is the single source of truth and `small_maps_json` re-projects it. This load is the whole
        // reason a placed annotation survives a reload — and it is EDITOR-side only, so nothing here
        // moves the compiled mission (see the `comments` field's never-compiles note).
        load_rows_ordered(
            &mut txn,
            &self.comments,
            payload.get("comments"),
            &entity_order,
            "comments",
        );
        // T-672 — top-level `connections[]` (promoted from `payloadExtras` by `compile_payload`) back
        // into the `connections` root, ordered like every sibling. Listing `connections` in
        // [`is_known_editor_payload_top_level`] is what keeps this off the parking path. This load is
        // the whole reason a drawn edge survives a reload — and it is EDITOR-side only, so nothing
        // here moves the compiled mission (see the `connections` field's never-compiles note).
        //
        // It is also why [`Self::connection_findings_json`] exists: this loader is DELIBERATELY
        // unvalidating (it is the generic row loader every sibling uses), so a payload authored
        // elsewhere can land a self-link, a duplicate or a dangling endpoint that
        // [`Self::add_connection`] would have refused. Rejecting rows at load would silently destroy
        // an operator's data; SHOWING them as findings is the honest answer.
        load_rows_ordered(
            &mut txn,
            &self.connections,
            payload.get("connections"),
            &entity_order,
            "connections",
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

    /// T-665 — set an Outliner layer's `hidden` VIEW flag (per-layer visibility).
    ///
    /// This is the ATTR-FIELD-LYR-ENABLE-VIS writer, and it is deliberately a sibling of
    /// [`Self::rename_editor_layer`]: like `name`, `hidden` is a per-layer property that rides the
    /// layer row in the doc, so it PERSISTS with the mission and goes through [`Self::begin`] — a
    /// LOCAL flip is one undo step, exactly like a rename ([`Self::hidden_flag_is_one_undo_step`]).
    ///
    /// **Hide is a view, not a delete.** Nothing about the slots changes: [`Self::materialize`]
    /// FILTERS a hidden layer's slots out of the render SoA (so the engine never uploads them —
    /// see that method's T-665 block for why the filter lives there and not render-side), while
    /// `slots_json` / `small_maps_json` still carry every slot verbatim, so a Save round-trips the
    /// full document. Un-hiding brings the slots straight back with no data loss.
    ///
    /// The flag is only written when `true`: `hidden == false` REMOVES the key rather than storing
    /// `false`, so a never-hidden layer's row shape is byte-identical to before this ticket (the
    /// `tag`/`assetId` omit idiom from [`Self::add_slot`]). Absent ⇒ visible.
    pub fn set_editor_layer_hidden(&self, id: &str, hidden: bool) {
        let mut txn = self.begin();
        if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, id) {
            if hidden {
                layer.insert(&mut txn, "hidden", true);
            } else {
                layer.remove(&mut txn, "hidden");
            }
        }
    }

    /// T-665 — set an Outliner layer's `locked` transform-lock flag (ATTR-FIELD-LYR-ENABLE-XFORM).
    ///
    /// Twin of [`Self::set_editor_layer_hidden`]: persisted per layer like `name`, undoable through
    /// [`Self::begin`], and stored only when `true` (absent ⇒ unlocked). A locked layer's slots
    /// refuse position edits — [`Self::move_entities`], [`Self::move_entities_and_vehicles`] and
    /// [`Self::update_slot_position`] silently skip a slot whose layer (or any ancestor layer)
    /// is locked; see [`Self::slot_layer_is_locked`] for the resolution and the refusal contract.
    pub fn set_editor_layer_locked(&self, id: &str, locked: bool) {
        let mut txn = self.begin();
        if let Some(Out::YMap(layer)) = self.editor_layers.get(&txn, id) {
            if locked {
                layer.insert(&mut txn, "locked", true);
            } else {
                layer.remove(&mut txn, "locked");
            }
        }
    }

    /// T-665 / T-082 — is `slot_id` transform-locked, i.e. does its resolved Outliner layer (or any
    /// ancestor of it) carry `locked`? The read half of the refusal contract
    /// [`Self::set_editor_layer_locked`] documents, and the exact predicate
    /// [`Self::update_slot_position`] / [`Self::move_entities`] branch on — the SAME
    /// `slot_is_transform_locked` function, not a restatement of it, so a UI that asks this cannot
    /// disagree with the core that enforces it.
    ///
    /// T-082 added it because the refusal was WRITE-ONLY: the mutators skipped a locked slot
    /// silently and no caller could ask in advance, so the Attributes modal had no way to tell an
    /// operator that the Transform field they are typing into is inert. An unfiled slot (in no
    /// layer) is never locked. Cheap: one map walk up `parentId`, cycle-guarded.
    #[must_use]
    pub fn slot_layer_is_locked(&self, slot_id: &str) -> bool {
        let txn = self.doc.transact();
        slot_is_transform_locked(&txn, &self.editor_layers, slot_id)
    }

    /// T-701 — set a SLOT's editor-local `editorHidden` VIEW flag (per-ENTITY visibility, 3den E9).
    ///
    /// The per-entity twin of [`Self::set_editor_layer_hidden`] and mirror of `slot.tag`'s home: like
    /// the layer flag it PERSISTS on the row and rides [`Self::begin`], so a single flip is ONE undo
    /// step; unlike it, the bit lives on the slot itself, letting a maker declutter a single dense-area
    /// entity without hiding its whole layer. Enforcement is at [`Self::materialize`], where effective-
    /// hidden = `layer-hidden OR entity-hidden`; a hidden slot leaves the render SoA while
    /// `slots_json` / the `editor.slots` payload still carry it verbatim (hide is a VIEW, not a delete
    /// — no data loss, un-hiding restores it).
    ///
    /// **Editor-block state, never the MOD wire.** The flag rides `editor.slots` (the editor-only
    /// block `MissionDocCore::hydrate` reloads verbatim), exactly like `slot.tag`; the MOD document
    /// (`mission::flatten::flatten_to_mod_document`) deserializes slots into `SlotIn`, whose fixed
    /// field list omits it, and emits `ModSlot`, which has no such field — so `editorHidden` is
    /// STRUCTURALLY absent from the compiled mission (proven by `editor_hidden_never_reaches_mod_wire`).
    ///
    /// Stored only when `true`: `false` REMOVES the key (the `tag`/`assetId` omit idiom of
    /// [`Self::add_slot`]), so a never-hidden slot's row is byte-identical to before this ticket
    /// (absent ⇒ visible). No-op when the slot id is absent.
    pub fn set_slot_editor_hidden(&self, id: &str, hidden: bool) {
        let mut txn = self.begin();
        set_slot_editor_hidden_in_txn(&mut txn, &self.slots, id, hidden);
    }

    /// T-701 — set `editorHidden` on MANY slots in ONE transaction (one undo step for the whole
    /// selection). Backs `editor_ops::{hide_selection, show_selection}` — the H-key affordance flips
    /// the whole censused selection, and Eden's Hide/Show is one undoable action, not one-per-slot.
    /// This is the per-entity-flag one-txn batch mutator the T-732 UNDO-HONESTY note says the position
    /// lane lacks; here store.rs is authored in-slice so the batch exists and the op is honestly one
    /// step. Unknown ids are skipped; `false` removes the key per the omit idiom.
    pub fn set_slots_editor_hidden(&self, ids: &[String], hidden: bool) {
        let mut txn = self.begin();
        for id in ids {
            set_slot_editor_hidden_in_txn(&mut txn, &self.slots, id, hidden);
        }
    }

    /// T-701 — clear EVERY slot's `editorHidden` in ONE transaction (the "Show All" / reveal-all
    /// command, one undo step). Only slots that actually carry the key are touched, so on a doc with
    /// no hidden entities this commits an empty transaction (no spurious undo entry beyond the
    /// begin/commit). Returns the number of slots un-hidden.
    pub fn clear_all_editor_hidden(&self) -> usize {
        let mut txn = self.begin();
        let hidden_ids: Vec<String> = self
            .slots
            .iter(&txn)
            .filter_map(|(id, out)| match out {
                Out::YMap(slot) if read_bool(&txn, &slot, "editorHidden") => Some(id.to_string()),
                _ => None,
            })
            .collect();
        for id in &hidden_ids {
            set_slot_editor_hidden_in_txn(&mut txn, &self.slots, id, false);
        }
        hidden_ids.len()
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
        // T-651 — the detach half is now [`remove_id_from_all_layers`], shared with
        // [`Self::remove_comment`] so "unfile this id" has exactly one implementation.
        remove_id_from_all_layers(&mut txn, &self.editor_layers, slot_id);
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

    /// T-069 — every authored briefing marker in the document, across every faction, as a JSON
    /// array of `{factionId, id, x, z, icon, label}`.
    ///
    /// **This is the half T-345 did not ship, and its absence is why the two marker mutators above
    /// had zero product callers.** A dock cannot list, re-caption, move or delete a marker it cannot
    /// READ, and until now the only way to see one was `small_maps_json()` — the whole document
    /// serialized to a string, per render, to reach four fields on one nested array. The two writers
    /// address a marker by `(faction_id, marker_id)`; this returns exactly that pair per row, so a
    /// caller can round-trip a listed row straight back into
    /// [`Self::set_faction_briefing_marker`] / [`Self::remove_faction_briefing_marker`] with no
    /// second lookup.
    ///
    /// ## Why the return type is a JSON string and not a typed row vector
    ///
    /// `doc/mod.rs` re-exports `store::MissionDocCore` and nothing else from this file, so a
    /// `pub struct BriefingMarkerRow` declared here would be unreachable from the frontend without a
    /// re-export. The string idiom is the one every other cross-crate reader on this type already
    /// uses ([`Self::slots_json`], [`Self::small_maps_json`], `compositions_json`), and the frontend
    /// parses it into ITS row type — which is where the display vocabulary belongs anyway.
    ///
    /// ## Ordering is deterministic, and it has to be
    ///
    /// `MapRef::iter` is unordered (a hash map walk), so faction order is sorted by `factionId` and
    /// the sort is STABLE, which preserves each faction's ARRAY order inside its own group. Array
    /// order is not cosmetic: `derive_briefings` pushes markers into the parallel arrays
    /// `TBD_MarkerService.Build` sends in exactly that order, and
    /// [`Self::set_faction_briefing_marker`] replaces in place specifically so a drag cannot
    /// reorder them. A list that shuffled between renders would make the dock rows dance under the
    /// cursor for a document that never changed.
    ///
    /// Rows with no doc-internal `id` are SKIPPED. Such a row is still compiled (see
    /// [`marker_row_id`]) — it is simply not addressable, so offering it in a list whose every verb
    /// takes an id would produce controls that silently do nothing.
    #[must_use]
    pub fn briefing_marker_rows_json(&self) -> String {
        let txn = self.doc.transact();
        let mut rows: Vec<serde_json::Value> = Vec::new();
        for (faction_id, out) in self.factions.iter(&txn) {
            let Out::YMap(f) = out else { continue };
            let briefing = read_any_map(&txn, &f, "briefing");
            for row in briefing_markers(&briefing) {
                let Some(id) = marker_row_id(&row) else {
                    continue;
                };
                let Any::Map(fields) = &row else { continue };
                let text = |k: &str| match fields.get(k) {
                    Some(Any::String(s)) => s.to_string(),
                    _ => String::new(),
                };
                rows.push(serde_json::json!({
                    "factionId": faction_id,
                    "id": id,
                    "x": fields.get("x").map_or(0.0, any_to_f64),
                    "z": fields.get("z").map_or(0.0, any_to_f64),
                    "icon": text("icon"),
                    "label": text("label"),
                }));
            }
        }
        // STABLE sort: faction groups become deterministic, array order inside a group survives.
        rows.sort_by(|a, b| a["factionId"].as_str().cmp(&b["factionId"].as_str()));
        serde_json::Value::Array(rows).to_string()
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
    /// / vehicle / entity / zone / marker. Backs `useMissionEditor.hasLocalContent` (the warm-session
    /// / conflict gate).
    ///
    /// **T-211 — zones count.** A mission whose only local edit is a drawn play area is a mission
    /// with unsaved work; omitting `zones` here would let the conflict gate discard it silently.
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
            || self.zones.len(&txn) > 0
            // T-650 — a saved composition is authored work that persists with the mission (the
            // zones-alone precedent): the conflict gate must not discard a doc that has one.
            || self.compositions.len(&txn) > 0
            // T-651 — a placed comment is authored work too (the zones-alone precedent). A maker who
            // opened a mission, wrote a seven-paragraph brief into an annotation and nothing else has
            // unsaved work, and the conflict gate must not discard it as an empty document.
            || self.comments.len(&txn) > 0
            || markers.len(&txn) > 0
    }

    // ── T-491 — pure pick / marquee math (host-shared; Class-R in this module) ───────────────────
    // Associated fns (no `&self`) so `MissionDocCore::…` is reachable without widening `doc/mod.rs`.
    // The Leptos `select_tool` wrappers call these — one implementation, native-tested.

    /// Click pick radius (CSS px) — React `slotSpatialIndex.pickNearest` default / select_tool pin.
    pub const PICK_RADIUS_PX: f64 = 4.0;
    /// `PointIndex` grid cell (world m) — React / select_tool `GRID_CELL_M`.
    pub const GRID_CELL_M: f64 = 256.0;

    /// Nearest slot id under a screen pixel, or `None` (box-nearest over the SoA).
    #[must_use]
    pub fn pick_slot(
        cam: &crate::camera::OrthoCamera,
        soa: &SlotSoa,
        px: f64,
        py: f64,
    ) -> Option<String> {
        mix_pick_slot(cam, soa, px, py)
    }

    /// T-425 — nearest placed vehicle id under a screen pixel, or `None`.
    #[must_use]
    pub fn pick_vehicle(
        cam: &crate::camera::OrthoCamera,
        points: &[(String, f64, f64)],
        px: f64,
        py: f64,
    ) -> Option<String> {
        mix_pick_vehicle(cam, points, px, py)
    }

    /// T-425 / T-491 — pick slot or vehicle; when both are in range, the closer world-distance wins.
    #[must_use]
    pub fn pick_slot_or_vehicle(
        cam: &crate::camera::OrthoCamera,
        soa: &SlotSoa,
        vehicle_points: &[(String, f64, f64)],
        px: f64,
        py: f64,
    ) -> Option<String> {
        mix_pick_slot_or_vehicle(cam, soa, vehicle_points, px, py)
    }

    /// Slot ids inside the marquee world AABB from press `(start_wx, start_wy)` + release px.
    #[must_use]
    pub fn marquee_slot_ids(
        cam: &crate::camera::OrthoCamera,
        soa: &SlotSoa,
        start_wx: f64,
        start_wy: f64,
        end_px: f64,
        end_py: f64,
    ) -> Vec<String> {
        mix_marquee_slot_ids(cam, soa, start_wx, start_wy, end_px, end_py)
    }

    /// T-425 — vehicle ids inside the marquee world AABB (same corners as [`Self::marquee_slot_ids`]).
    #[must_use]
    pub fn marquee_vehicle_ids(
        cam: &crate::camera::OrthoCamera,
        points: &[(String, f64, f64)],
        start_wx: f64,
        start_wy: f64,
        end_px: f64,
        end_py: f64,
    ) -> Vec<String> {
        mix_marquee_vehicle_ids(cam, points, start_wx, start_wy, end_px, end_py)
    }

    /// T-425 / T-491 — marquee over slots **and** placed vehicles (vehicles appended after slots).
    #[must_use]
    pub fn marquee_ids_with_vehicles(
        cam: &crate::camera::OrthoCamera,
        soa: &SlotSoa,
        vehicle_points: &[(String, f64, f64)],
        start_wx: f64,
        start_wy: f64,
        end_px: f64,
        end_py: f64,
    ) -> Vec<String> {
        mix_marquee_ids_with_vehicles(cam, soa, vehicle_points, start_wx, start_wy, end_px, end_py)
    }

    /// T-693 (NEW-F4 / MENU-SCEN-011) — merge ANOTHER mission's EDITOR payload into the CURRENT doc.
    ///
    /// `payload` is the same JSON shape [`Self::hydrate`] consumes and [`compile_payload`] emits:
    /// top-level `vehicles[]` / `entities[]` / `zones[]` / `compositions[]` / `triggers[]` /
    /// `markers[]`, plus `editor.{factions,squads,slots,editorLayers}[]`. This does NOT replace the
    /// doc (unlike `hydrate`, which clears everything first) — it **adds** the incoming content on top
    /// of what is already here, so it is the "compose an ORBAT from a template mission" primitive.
    ///
    /// ## Three hard problems, and how each is handled
    ///
    /// **1. Id collision (the hard part).** Two independently-authored missions mint ids from the same
    /// small alphabet (`s0`, `sq`, `faction-BLUFOR`, …), so a naive merge would collide the incoming
    /// `s0` onto the resident one and corrupt both. EVERY incoming id — of every kind — is re-minted
    /// through one [`RemintMap`] (the `paste_slots` re-mint idiom, generalized: paste re-mints slot ids
    /// caller-side and this does it here for the whole graph), and every INTRA-payload reference is
    /// rewritten through the same map so the merged sub-graph is internally consistent against its new
    /// ids: `squad.slotIds`/`leaderSlotId`/`factionId`/`vehicleIds`, `faction.squadIds`,
    /// `slot.squadId`, `editorLayer.entityIds`/`parentId`, `vehicle.squadId`/`factionId` and each
    /// crew seat (`crew[seat] = slotId`), and `trigger.ownerId`. A reference that resolves to nothing
    /// after re-mint (a dangling incoming edge) is dropped, never guessed.
    ///
    /// **2. ORBAT dedup by name+side.** A template's BLUFOR is the resident BLUFOR — merging should
    /// grow the existing side, not spawn a parallel one. A faction is deduped by (`name`, `key`) and a
    /// squad by (`name`, its faction's `key`): an incoming squad whose (name, side) matches a resident
    /// squad MERGES its slots into that squad (the incoming squad id maps to the resident one, its
    /// slots are appended to the resident `slotIds`, and no new squad row is written); a squad with no
    /// match is CREATED with a re-minted id. Same for factions. This is why factions/squads are
    /// resolved BEFORE slots: a slot's `squadId` must point at the post-dedup squad.
    ///
    /// **3. Totality (T-657 discipline).** A malformed incoming row — missing `id`, a slot pointing at
    /// a squad neither incoming nor resident, a non-object where a row is expected — is SKIPPED and
    /// recorded in [`MergeReport::skipped`], never panicked on. The whole merge is applied inside ONE
    /// transaction (like `paste_slots`), so it is exactly one undo step and an undo restores the
    /// pre-merge document precisely.
    ///
    /// ## Placement
    ///
    /// Merged entities keep their AUTHORED positions by default (a mission is a coherent spatial
    /// document — the template's spawns are already where they belong relative to each other and the
    /// terrain). `opts.offset` = `Some((dx, dy))` shifts every placed entity by that world delta (the
    /// template-into-a-corner case); positions are NOT clamped to bounds here — the source mission's
    /// coordinates are trusted, and a merge is not a placement gesture.
    #[must_use]
    pub fn merge_mission_payload(
        &self,
        payload: &serde_json::Value,
        opts: MergeOpts,
    ) -> MergeReport {
        let mut report = MergeReport::default();
        let Some(obj) = payload.as_object() else {
            report
                .skipped
                .push(("payload".into(), String::new(), "not a JSON object".into()));
            return report;
        };
        let editor = obj.get("editor").and_then(serde_json::Value::as_object);

        // ── Pass 0: build the id re-mint map + the faction/squad dedup decisions ────────────────────
        // Read the resident squads/factions ONCE (tolerant of both YMap + hydrated-opaque shapes via
        // `ordered_rows`) to build the (name, side) → id dedup index the incoming rows resolve against.
        let (dx, dy) = opts.offset.unwrap_or((0.0, 0.0));

        // Grab the non-tracked root handles BEFORE any txn opens: `get_or_insert_map` opens its own
        // internal transaction, so calling it while a `transact()` / `begin()` is alive DEADLOCKS
        // (the measured hang the `small_maps_json` note warns of). `markers` is the only entity map
        // this method writes that is not a struct field.
        let entity_order = self.doc.get_or_insert_map("entityOrder");
        let markers = self.doc.get_or_insert_map("markers");

        // Resident dedup indices AND the resident id universe, read before the write txn opens. The
        // universe (every resident row's id — its doc KEY — across every entity map this method may
        // insert into) seeds the re-mint's collision guard so no minted id can equal an id already in
        // the doc, including a `mrg-…` id a PRIOR merge left resident (BLOCKER-1). Same pre-txn hoist
        // as the handles above: iterating these maps under the open write txn would be an alias, and
        // `markers` is read here (its `iter` on the just-grabbed handle) rather than re-grabbed later.
        let (resident_factions, resident_squads_by_side, resident_ids) = {
            let txn = self.doc.transact();
            let mut resident_ids: HashSet<String> = HashSet::new();
            for map in [
                &self.slots,
                &self.squads,
                &self.factions,
                &self.editor_layers,
                &self.vehicles,
                &self.entities,
                &self.zones,
                &self.triggers,
                &self.compositions,
                &markers,
            ] {
                for (id, _out) in map.iter(&txn) {
                    resident_ids.insert(id.to_string());
                }
            }
            let fac_rows = ordered_rows(&txn, &self.factions, &entity_order, "factions");
            // faction (name, key) → resident faction id, and faction id → key (to resolve squad side).
            let mut fac_index: HashMap<(String, String), String> = HashMap::new();
            let mut fac_key_by_id: HashMap<String, String> = HashMap::new();
            for row in &fac_rows {
                if let Any::Map(m) = row {
                    let id = any_map_str(m, "id");
                    let key = any_map_str(m, "key");
                    let name = any_map_str(m, "name");
                    if let Some(id) = &id {
                        if let Some(k) = &key {
                            fac_key_by_id.insert(id.clone(), k.clone());
                        }
                        if let (Some(k), Some(n)) = (&key, &name) {
                            fac_index
                                .entry((n.clone(), k.clone()))
                                .or_insert(id.clone());
                        }
                    }
                }
            }
            let sq_rows = ordered_rows(&txn, &self.squads, &entity_order, "squads");
            // squad (name, side-key) → resident squad id.
            let mut sq_index: HashMap<(String, String), String> = HashMap::new();
            for row in &sq_rows {
                if let Any::Map(m) = row {
                    let id = any_map_str(m, "id");
                    let name = any_map_str(m, "name");
                    let side = any_map_str(m, "factionId")
                        .and_then(|fid| fac_key_by_id.get(&fid).cloned());
                    if let (Some(id), Some(n), Some(s)) = (id, name, side) {
                        sq_index.entry((n, s)).or_insert(id);
                    }
                }
            }
            (fac_index, sq_index, resident_ids)
        };

        // The re-mint table seeded with the resident id universe: every id it now mints is guaranteed
        // absent from the doc (and from its own prior mints), so a second merge of the same template
        // lands alongside the first instead of overwriting it.
        let mut remint = RemintMap::with_reserved(resident_ids);

        // Incoming faction rows: decide MERGE (map to resident) vs CREATE (fresh id), and remember
        // each incoming faction's key so squad side can be resolved from the incoming graph too.
        let incoming_factions: Vec<&serde_json::Map<String, serde_json::Value>> = editor
            .and_then(|e| e.get("factions"))
            .and_then(serde_json::Value::as_array)
            .map(|a| a.iter().filter_map(serde_json::Value::as_object).collect())
            .unwrap_or_default();
        let mut incoming_fac_key: HashMap<String, String> = HashMap::new();
        for f in &incoming_factions {
            let Some(id) = json_str(f, "id") else {
                continue;
            };
            let key = json_str(f, "key");
            if let Some(k) = &key {
                incoming_fac_key.insert(id.clone(), k.clone());
            }
            let name = json_str(f, "name");
            if let (Some(k), Some(n)) = (&key, &name)
                && let Some(resident) = resident_factions.get(&(n.clone(), k.clone()))
            {
                remint.map_to_existing(&id, resident); // dedup: incoming faction IS the resident one
            }
        }

        // Incoming squad rows: dedup by (name, side) where side = incoming/resident faction key of the
        // squad's factionId. A matched squad maps to the resident squad (slots merge in); else fresh.
        let incoming_squads: Vec<&serde_json::Map<String, serde_json::Value>> = editor
            .and_then(|e| e.get("squads"))
            .and_then(serde_json::Value::as_array)
            .map(|a| a.iter().filter_map(serde_json::Value::as_object).collect())
            .unwrap_or_default();
        // squad id → resident squad id it merged into (so slots append rather than create a squad).
        let mut squad_merged_into: HashMap<String, String> = HashMap::new();
        for s in &incoming_squads {
            let Some(id) = json_str(s, "id") else {
                continue;
            };
            let name = json_str(s, "name");
            // Resolve the squad's side key: prefer the incoming faction's key, fall back to a resident
            // one (the squad's factionId may already have been deduped onto a resident faction).
            let side = json_str(s, "factionId").and_then(|fid| {
                incoming_fac_key.get(&fid).cloned().or_else(|| {
                    remint.get(&fid).and_then(|rid| {
                        // fid deduped onto a resident faction — read that faction's key.
                        resident_factions
                            .iter()
                            .find_map(|((_n, k), v)| (v == &rid).then(|| k.clone()))
                    })
                })
            });
            if let (Some(n), Some(s_key)) = (name, side)
                && let Some(resident_sq) = resident_squads_by_side.get(&(n, s_key))
            {
                remint.map_to_existing(&id, resident_sq);
                squad_merged_into.insert(id, resident_sq.clone());
            }
        }

        // Every other incoming id (factions/squads not already mapped, plus slots, vehicles, entities,
        // layers, zones, compositions, triggers, markers) gets a FRESH re-minted id. Reserve all of
        // them up front so intra-payload references resolve regardless of row order.
        for f in &incoming_factions {
            if let Some(id) = json_str(f, "id") {
                remint.ensure_fresh(&id);
            }
        }
        for s in &incoming_squads {
            if let Some(id) = json_str(s, "id") {
                remint.ensure_fresh(&id);
            }
        }
        for arr in [
            editor.and_then(|e| e.get("slots")),
            editor.and_then(|e| e.get("editorLayers")),
            obj.get("vehicles"),
            obj.get("entities"),
            obj.get("zones"),
            obj.get("compositions"),
            obj.get("triggers"),
            obj.get("markers"),
        ] {
            if let Some(rows) = arr.and_then(serde_json::Value::as_array) {
                for row in rows {
                    if let Some(id) = row.as_object().and_then(|m| json_str(m, "id")) {
                        remint.ensure_fresh(&id);
                    }
                }
            }
        }

        // ── Pass 1: write everything under ONE txn (one undo step) ─────────────────────────────────
        let mut txn = self.begin();

        // Factions: only CREATE the ones that did not dedup onto a resident faction.
        for f in &incoming_factions {
            let Some(old_id) = json_str(f, "id") else {
                report
                    .skipped
                    .push(("faction".into(), String::new(), "missing id".into()));
                continue;
            };
            if squad_or_faction_is_merged(&remint, &old_id) {
                report.factions_merged += 1;
                continue; // merged into a resident faction — squadIds handled by squad writes below
            }
            let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
            let fac = self.factions.insert(
                &mut txn,
                new_id.as_str(),
                MapPrelim::from([("id", new_id.as_str())]),
            );
            copy_row_fields_except(&mut txn, &fac, f, &["id", "squadIds"]);
            // squadIds is rebuilt from the squads that actually land under this faction (below), so
            // seed it empty here and append as squads are written.
            fac.insert(&mut txn, "squadIds", Any::Array(Vec::new().into()));
            report.factions_created += 1;
        }

        // Squads: MERGE (append slots to resident) or CREATE (fresh row + attach to its faction).
        for s in &incoming_squads {
            let Some(old_id) = json_str(s, "id") else {
                report
                    .skipped
                    .push(("squad".into(), String::new(), "missing id".into()));
                continue;
            };
            if squad_merged_into.contains_key(&old_id) {
                report.squads_merged += 1;
                continue; // resident squad row untouched; its slotIds grow via the slot writes below
            }
            let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
            let sq = self.squads.insert(
                &mut txn,
                new_id.as_str(),
                MapPrelim::from([("id", new_id.as_str())]),
            );
            // Copy every field except id + the ref arrays we rebuild from membership + factionId +
            // leaderSlotId (both are ids that must be re-minted, written explicitly below).
            copy_row_fields_except(
                &mut txn,
                &sq,
                s,
                &["id", "slotIds", "vehicleIds", "factionId", "leaderSlotId"],
            );
            sq.insert(&mut txn, "slotIds", Any::Array(Vec::new().into()));
            sq.insert(&mut txn, "vehicleIds", Any::Array(Vec::new().into()));
            // factionId → the re-minted or deduped faction (dropped if it resolves nowhere).
            if let Some(fid) = json_str(s, "factionId").and_then(|f| remint.get(&f)) {
                sq.insert(&mut txn, "factionId", fid.as_str());
                append_id(&mut txn, &self.factions, &fid, "squadIds", &new_id);
            }
            // leaderSlotId → the re-minted member slot. The slot writes below also re-file the
            // slot into this squad; `set_leader_in_txn`'s membership guard is satisfied because the
            // slot's `slotIds` append happens in the same txn before any read of the leader.
            if let Some(lid) = json_str(s, "leaderSlotId").and_then(|l| remint.get(&l)) {
                sq.insert(&mut txn, "leaderSlotId", lid.as_str());
            }
            report.squads_created += 1;
        }

        // Slots: re-mint id, rewrite squadId to the post-dedup squad, keep authored position (+offset),
        // and append to that squad's slotIds + (optionally) a layer's entityIds handled in the layer
        // pass. A slot whose squad resolves nowhere is filed unfiled (squadId dropped) but still lands.
        let incoming_slots = editor
            .and_then(|e| e.get("slots"))
            .and_then(serde_json::Value::as_array);
        if let Some(rows) = incoming_slots {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report
                        .skipped
                        .push(("slot".into(), String::new(), "not an object".into()));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("slot".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let slot = self.slots.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                // squadId → post-dedup squad (resident-merged or freshly created). Dropped if unknown.
                let squad_id = json_str(m, "squadId").and_then(|sid| remint.get(&sid));
                copy_row_fields_except(&mut txn, &slot, m, &["id", "squadId", "position"]);
                if let Some(sid) = &squad_id {
                    slot.insert(&mut txn, "squadId", sid.as_str());
                }
                // Authored position, offset by opts.offset; unknown position sub-keys preserved.
                let (px, py, pz, prot) = json_position(m);
                let mut pos = json_position_map(m);
                pos.insert("x".to_string(), Any::Number(px + dx));
                pos.insert("y".to_string(), Any::Number(py + dy));
                pos.insert("z".to_string(), Any::Number(pz));
                pos.insert("rotation".to_string(), Any::Number(prot));
                slot.insert(&mut txn, "position", Any::Map(Arc::new(pos)));
                if let Some(sid) = &squad_id {
                    append_id(&mut txn, &self.squads, sid, "slotIds", &new_id);
                }
                report.slots_added += 1;
            }
        }

        // Editor layers: re-mint id + parentId + entityIds (slot ids). A layer's parentId that
        // resolves nowhere becomes a root layer rather than dangling.
        if let Some(rows) = editor
            .and_then(|e| e.get("editorLayers"))
            .and_then(serde_json::Value::as_array)
        {
            for row in rows {
                let Some(m) = row.as_object() else { continue };
                let Some(old_id) = json_str(m, "id") else {
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let layer = self.editor_layers.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                copy_row_fields_except(&mut txn, &layer, m, &["id", "parentId", "entityIds"]);
                match json_str(m, "parentId").and_then(|p| remint.get(&p)) {
                    Some(pid) => layer.insert(&mut txn, "parentId", pid.as_str()),
                    None => layer.insert(&mut txn, "parentId", Any::Null),
                };
                let entity_ids: Vec<Any> = m
                    .get("entityIds")
                    .and_then(serde_json::Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(serde_json::Value::as_str)
                            .filter_map(|s| remint.get(s))
                            .map(|s| Any::String(s.into()))
                            .collect()
                    })
                    .unwrap_or_default();
                layer.insert(&mut txn, "entityIds", Any::Array(entity_ids.into()));
            }
        }

        // Vehicles: re-mint id + squadId + factionId + every crew seat's slot id; keep authored
        // position (+offset). A crew seat pointing at a dropped slot is removed from the seat map.
        if let Some(rows) = obj.get("vehicles").and_then(serde_json::Value::as_array) {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report
                        .skipped
                        .push(("vehicle".into(), String::new(), "not an object".into()));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("vehicle".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let v = self.vehicles.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                copy_row_fields_except(
                    &mut txn,
                    &v,
                    m,
                    &["id", "squadId", "factionId", "crew", "position"],
                );
                if let Some(sid) = json_str(m, "squadId").and_then(|s| remint.get(&s)) {
                    v.insert(&mut txn, "squadId", sid.as_str());
                    append_id(&mut txn, &self.squads, &sid, "vehicleIds", &new_id);
                }
                if let Some(fid) = json_str(m, "factionId").and_then(|f| remint.get(&f)) {
                    v.insert(&mut txn, "factionId", fid.as_str());
                }
                if m.contains_key("position") {
                    let (px, py, pz, prot) = json_position(m);
                    let mut pos = json_position_map(m);
                    pos.insert("x".to_string(), Any::Number(px + dx));
                    pos.insert("y".to_string(), Any::Number(py + dy));
                    pos.insert("z".to_string(), Any::Number(pz));
                    pos.insert("rotation".to_string(), Any::Number(prot));
                    v.insert(&mut txn, "position", Any::Map(Arc::new(pos)));
                }
                // Crew: seat → slot id, each slot id re-minted; drop a seat whose slot vanished.
                if let Some(crew) = m.get("crew").and_then(serde_json::Value::as_object) {
                    let mut seats: HashMap<String, Any> = HashMap::new();
                    for (seat, occ) in crew {
                        if let Some(sid) = occ.as_str().and_then(|s| remint.get(s)) {
                            seats.insert(seat.clone(), Any::String(sid.into()));
                        }
                    }
                    if !seats.is_empty() {
                        v.insert(&mut txn, "crew", Any::Map(Arc::new(seats)));
                    }
                }
                report.vehicles_added += 1;
            }
        }

        // Entities (mission-placed world objects): re-mint id, keep authored position (+offset). No
        // squad/faction re-mint (their `faction` is a slug string, not an id).
        if let Some(rows) = obj.get("entities").and_then(serde_json::Value::as_array) {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report
                        .skipped
                        .push(("entity".into(), String::new(), "not an object".into()));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("entity".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let e = self.entities.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                copy_row_fields_except(&mut txn, &e, m, &["id", "position"]);
                if m.contains_key("position") {
                    let (px, py, pz, prot) = json_position(m);
                    let mut pos = json_position_map(m);
                    pos.insert("x".to_string(), Any::Number(px + dx));
                    pos.insert("y".to_string(), Any::Number(py + dy));
                    pos.insert("z".to_string(), Any::Number(pz));
                    pos.insert("rotation".to_string(), Any::Number(prot));
                    e.insert(&mut txn, "position", Any::Map(Arc::new(pos)));
                }
                report.entities_added += 1;
            }
        }

        // Zones: re-mint id + offset the shape geometry. Geometry stays opaque otherwise.
        report.zones_added += merge_shape_rows(
            &mut txn,
            &self.zones,
            obj.get("zones"),
            &remint,
            dx,
            dy,
            "zone",
            &mut report.skipped,
        );

        // Triggers: re-mint id + ownerId (a placed slot/vehicle) + offset the shape geometry. A
        // dangling ownerId (owner not in this payload nor resolvable) is dropped (the T-079 contract).
        if let Some(rows) = obj.get("triggers").and_then(serde_json::Value::as_array) {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report
                        .skipped
                        .push(("trigger".into(), String::new(), "not an object".into()));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("trigger".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let t = self.triggers.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                copy_row_fields_except(&mut txn, &t, m, &["id", "ownerId", "shape"]);
                if let Some(oid) = json_str(m, "ownerId").and_then(|o| remint.get(&o)) {
                    t.insert(&mut txn, "ownerId", oid.as_str());
                }
                if let Some(shape) = m.get("shape") {
                    t.insert(&mut txn, "shape", offset_shape_any(shape, dx, dy));
                }
                report.triggers_added += 1;
            }
        }

        // Compositions: self-contained TEMPLATE rows (their inner `entities` are relative-offset, not
        // live ids), so only the composition ROW id is re-minted — the offset does NOT apply (a
        // composition places relative to a future drop point, not the mission frame). Copy verbatim.
        if let Some(rows) = obj
            .get("compositions")
            .and_then(serde_json::Value::as_array)
        {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report.skipped.push((
                        "composition".into(),
                        String::new(),
                        "not an object".into(),
                    ));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("composition".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let c = self.compositions.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                // No remint of inner refs (there are none) — copy every non-id field verbatim.
                for (k, v) in m {
                    if k != "id" {
                        c.insert(&mut txn, k.as_str(), value_to_any(v));
                    }
                }
                report.compositions_added += 1;
            }
        }

        // Markers: re-mint id + offset the {x,z} coordinate. No cross-refs. (`markers` handle was
        // hoisted above `self.begin()` — grabbing it here would deadlock against the open txn.)
        if let Some(rows) = obj.get("markers").and_then(serde_json::Value::as_array) {
            for row in rows {
                let Some(m) = row.as_object() else {
                    report
                        .skipped
                        .push(("marker".into(), String::new(), "not an object".into()));
                    continue;
                };
                let Some(old_id) = json_str(m, "id") else {
                    report
                        .skipped
                        .push(("marker".into(), String::new(), "missing id".into()));
                    continue;
                };
                let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
                let mk = markers.insert(
                    &mut txn,
                    new_id.as_str(),
                    MapPrelim::from([("id", new_id.as_str())]),
                );
                for (k, v) in m {
                    match k.as_str() {
                        "id" => {}
                        "x" => {
                            mk.insert(&mut txn, "x", Any::Number(json_num(m, "x") + dx));
                        }
                        "z" => {
                            mk.insert(&mut txn, "z", Any::Number(json_num(m, "z") + dy));
                        }
                        _ => {
                            mk.insert(&mut txn, k.as_str(), value_to_any(v));
                        }
                    }
                }
                report.markers_added += 1;
            }
        }

        report
    }

    /// T-693 — JSON wrapper over [`Self::merge_mission_payload`] for the wasm boundary: parses the
    /// incoming payload text, runs the merge, and returns [`MergeReport`] as a JSON string. The typed
    /// method + its `MergeReport`/`MergeOpts` types are not re-exported from `doc/mod.rs` (out of this
    /// slice's owns), so the Leptos command reaches the merge through this string seam — the same
    /// `small_maps_json` / `slots_json` idiom every other command uses. A payload that does not parse
    /// yields a report whose `skipped` names the parse failure rather than erroring.
    #[must_use]
    pub fn merge_mission_payload_json(
        &self,
        payload_json: &str,
        offset: Option<(f64, f64)>,
    ) -> String {
        let report = match serde_json::from_str::<serde_json::Value>(payload_json) {
            Ok(payload) => self.merge_mission_payload(&payload, MergeOpts { offset }),
            Err(e) => {
                let mut report = MergeReport::default();
                report.skipped.push((
                    "payload".into(),
                    String::new(),
                    format!("invalid JSON: {e}"),
                ));
                report
            }
        };
        report.to_json_string()
    }
}

impl Default for MissionDocCore {
    fn default() -> Self {
        Self::new()
    }
}

// ── T-491 mix pick/marquee (pure; used by [`MissionDocCore`] associated fns + Class-R) ───────────

fn mix_world_pick_radius(
    cam: &crate::camera::OrthoCamera,
    px: f64,
    py: f64,
    radius_px: f64,
) -> f64 {
    let c = cam.unproject_xy(px, py);
    let e = cam.unproject_xy(px + radius_px, py);
    (e[0] - c[0]).abs()
}

fn mix_box_nearest(
    idx: &crate::spatial::point_index::PointIndex,
    soa: &SlotSoa,
    qx: f64,
    qy: f64,
    r: f64,
) -> Option<u32> {
    let mut best: Option<(f64, u32)> = None;
    for h in idx.pick_rect(qx - r, qy - r, qx + r, qy + r) {
        let dx = f64::from(soa.xs[h as usize]) - qx;
        let dy = f64::from(soa.ys[h as usize]) - qy;
        let d2 = dx * dx + dy * dy;
        if best.is_none_or(|(bd, _)| d2 < bd) {
            best = Some((d2, h));
        }
    }
    best.map(|(_, h)| h)
}

fn mix_pick_slot(
    cam: &crate::camera::OrthoCamera,
    soa: &SlotSoa,
    px: f64,
    py: f64,
) -> Option<String> {
    if soa.ids.is_empty() {
        return None;
    }
    let c = cam.unproject_xy(px, py);
    let (qx, qy) = (c[0], c[1]);
    if !qx.is_finite() || !qy.is_finite() {
        return None;
    }
    let r = mix_world_pick_radius(cam, px, py, MissionDocCore::PICK_RADIUS_PX);
    let idx = crate::spatial::point_index::PointIndex::build(
        soa.xs.clone(),
        soa.ys.clone(),
        MissionDocCore::GRID_CELL_M,
    );
    mix_box_nearest(&idx, soa, qx, qy, r).map(|h| soa.ids[h as usize].clone())
}

fn mix_pick_vehicle(
    cam: &crate::camera::OrthoCamera,
    points: &[(String, f64, f64)],
    px: f64,
    py: f64,
) -> Option<String> {
    if points.is_empty() {
        return None;
    }
    let c = cam.unproject_xy(px, py);
    let (qx, qy) = (c[0], c[1]);
    if !qx.is_finite() || !qy.is_finite() {
        return None;
    }
    let r = mix_world_pick_radius(cam, px, py, MissionDocCore::PICK_RADIUS_PX);
    let r2 = r * r;
    let mut best: Option<(f64, &str)> = None;
    for (id, x, y) in points {
        let dx = x - qx;
        let dy = y - qy;
        let d2 = dx * dx + dy * dy;
        if d2 > r2 {
            continue;
        }
        if best.is_none_or(|(bd, _)| d2 < bd) {
            best = Some((d2, id.as_str()));
        }
    }
    best.map(|(_, id)| id.to_string())
}

fn mix_pick_slot_or_vehicle(
    cam: &crate::camera::OrthoCamera,
    soa: &SlotSoa,
    vehicle_points: &[(String, f64, f64)],
    px: f64,
    py: f64,
) -> Option<String> {
    let slot = mix_pick_slot(cam, soa, px, py);
    let veh = mix_pick_vehicle(cam, vehicle_points, px, py);
    match (slot, veh) {
        (None, v) => v,
        (s, None) => s,
        (Some(s), Some(v)) => {
            let c = cam.unproject_xy(px, py);
            let (qx, qy) = (c[0], c[1]);
            let slot_d2 = soa
                .ids
                .iter()
                .position(|id| *id == s)
                .map(|i| {
                    let dx = f64::from(soa.xs[i]) - qx;
                    let dy = f64::from(soa.ys[i]) - qy;
                    dx * dx + dy * dy
                })
                .unwrap_or(f64::INFINITY);
            let veh_d2 = vehicle_points
                .iter()
                .find(|(id, _, _)| *id == v)
                .map(|(_, x, y)| {
                    let dx = x - qx;
                    let dy = y - qy;
                    dx * dx + dy * dy
                })
                .unwrap_or(f64::INFINITY);
            if veh_d2 < slot_d2 { Some(v) } else { Some(s) }
        }
    }
}

fn mix_marquee_slot_ids(
    cam: &crate::camera::OrthoCamera,
    soa: &SlotSoa,
    start_wx: f64,
    start_wy: f64,
    end_px: f64,
    end_py: f64,
) -> Vec<String> {
    if soa.ids.is_empty() {
        return Vec::new();
    }
    let e = cam.unproject_xy(end_px, end_py);
    let (ewx, ewy) = (e[0], e[1]);
    if !ewx.is_finite() || !ewy.is_finite() || !start_wx.is_finite() || !start_wy.is_finite() {
        return Vec::new();
    }
    let (min_x, max_x) = (start_wx.min(ewx), start_wx.max(ewx));
    let (min_y, max_y) = (start_wy.min(ewy), start_wy.max(ewy));
    let idx = crate::spatial::point_index::PointIndex::build(
        soa.xs.clone(),
        soa.ys.clone(),
        MissionDocCore::GRID_CELL_M,
    );
    idx.pick_rect(min_x, min_y, max_x, max_y)
        .into_iter()
        .map(|h| soa.ids[h as usize].clone())
        .collect()
}

fn mix_marquee_vehicle_ids(
    cam: &crate::camera::OrthoCamera,
    points: &[(String, f64, f64)],
    start_wx: f64,
    start_wy: f64,
    end_px: f64,
    end_py: f64,
) -> Vec<String> {
    if points.is_empty() {
        return Vec::new();
    }
    let e = cam.unproject_xy(end_px, end_py);
    let (ewx, ewy) = (e[0], e[1]);
    if !ewx.is_finite() || !ewy.is_finite() || !start_wx.is_finite() || !start_wy.is_finite() {
        return Vec::new();
    }
    let (min_x, max_x) = (start_wx.min(ewx), start_wx.max(ewx));
    let (min_y, max_y) = (start_wy.min(ewy), start_wy.max(ewy));
    points
        .iter()
        .filter(|(_, x, y)| *x >= min_x && *x <= max_x && *y >= min_y && *y <= max_y)
        .map(|(id, _, _)| id.clone())
        .collect()
}

fn mix_marquee_ids_with_vehicles(
    cam: &crate::camera::OrthoCamera,
    soa: &SlotSoa,
    vehicle_points: &[(String, f64, f64)],
    start_wx: f64,
    start_wy: f64,
    end_px: f64,
    end_py: f64,
) -> Vec<String> {
    let mut ids = mix_marquee_slot_ids(cam, soa, start_wx, start_wy, end_px, end_py);
    ids.extend(mix_marquee_vehicle_ids(
        cam,
        vehicle_points,
        start_wx,
        start_wy,
        end_px,
        end_py,
    ));
    ids
}

/// A `{x,y,z,rotation}` plain object as a `yrs` `Any::Map` (how Yjs stores `Slot.position`).
fn position_any(x: f64, y: f64, z: f64, rotation: f64) -> Any {
    position_any_merged(HashMap::new(), x, y, z, rotation)
}

/// T-211 — `$defs/shape` circle branch: `{ "circle": { x, z, r } }`.
///
/// `Any::Number` (not `Any::BigInt`) for all three even when the value is integral. Yjs encodes
/// integer-valued numbers as `BigInt` and `value_to_any` reproduces that on the hydrate side, so a
/// zone authored at x=4200.0 comes BACK as `BigInt(4200)`. Both serialise to the same JSON token
/// `4200`, which is what the wire and the schema see, so the round trip is stable at the payload
/// level — the variant asymmetry is internal and is exactly why the round-trip test asserts on the
/// compiled JSON rather than on `Any` equality.
fn circle_shape_any(x: f64, z: f64, r: f64) -> Any {
    let circle: HashMap<String, Any> = HashMap::from([
        ("x".to_string(), Any::Number(x)),
        ("z".to_string(), Any::Number(z)),
        ("r".to_string(), Any::Number(r)),
    ]);
    Any::Map(Arc::new(HashMap::from([(
        "circle".to_string(),
        Any::Map(Arc::new(circle)),
    )])))
}

/// T-211 — `$defs/shape` polygon branch: `{ "polygon": [[x,z],…] }` from a flat `[x0,z0,x1,z1,…]`.
/// `chunks_exact(2)` drops a trailing unpaired coordinate rather than emitting a 1-element vertex,
/// which `$defs/polygon`'s `minItems: 2 / maxItems: 2` per point would reject.
fn polygon_shape_any(points_flat: &[f64]) -> Any {
    let ring: Vec<Any> = points_flat
        .chunks_exact(2)
        .map(|p| Any::Array(vec![Any::Number(p[0]), Any::Number(p[1])].into()))
        .collect();
    Any::Map(Arc::new(HashMap::from([(
        "polygon".to_string(),
        Any::Array(ring.into()),
    )])))
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
        // Dedup the append: an id already in the array is not appended again. Without this a
        // duplicate incoming id (two rows sharing one id — MINOR-4) or a re-merge whose mint collided
        // pre-fix would double-append the same id into a `slotIds`/`squadIds`/`entityIds` array,
        // inflating membership over the real row count. The membership arrays hold each id at most
        // once by contract (a slot belongs to a squad once), so this is the invariant, not a patch.
        if next
            .iter()
            .any(|a| matches!(a, Any::String(s) if s.as_ref() == id))
        {
            return;
        }
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

// ── T-693 merge_mission_payload support (types + pure helpers) ───────────────────────────────────

/// T-693 — options for [`MissionDocCore::merge_mission_payload`].
///
/// `offset` shifts every merged entity's authored position by `(dx, dy)` world meters. `None` (and
/// the `(0.0, 0.0)` it collapses to) keeps the source mission's coordinates verbatim — the default,
/// because a mission is a coherent spatial document. The template-into-a-corner case supplies a delta.
#[derive(Debug, Clone, Copy, Default)]
pub struct MergeOpts {
    /// World-space `(dx, dy)` applied to every placed entity; `None` = keep authored positions.
    pub offset: Option<(f64, f64)>,
}

/// T-693 — the outcome of a merge, per the NEW-F4 design.
///
/// Counts are of rows that LANDED. `squads_merged` / `factions_merged` count incoming rows that
/// deduped onto a resident side (their content was folded in, no new row created); `*_created` count
/// fresh rows. `skipped` is the tolerance ledger: `(kind, id, reason)` for every malformed row the
/// merge refused rather than panicking on (the T-657 totality discipline).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeReport {
    /// Slots added to the document.
    pub slots_added: u32,
    /// Incoming squads folded into a resident squad (same name+side).
    pub squads_merged: u32,
    /// Incoming squads created fresh (no resident match).
    pub squads_created: u32,
    /// Incoming factions folded into a resident faction (same name+side key).
    pub factions_merged: u32,
    /// Incoming factions created fresh.
    pub factions_created: u32,
    /// Vehicles added.
    pub vehicles_added: u32,
    /// Mission-placed entities (world objects) added.
    pub entities_added: u32,
    /// Zones added.
    pub zones_added: u32,
    /// Triggers added.
    pub triggers_added: u32,
    /// Compositions (self-contained templates) added.
    pub compositions_added: u32,
    /// Markers added.
    pub markers_added: u32,
    /// Malformed rows the merge tolerated: `(kind, id, reason)`. Never a panic.
    pub skipped: Vec<(String, String, String)>,
}

impl MergeReport {
    /// Serialize the report to a compact JSON object for the wasm command seam. `skipped` becomes an
    /// array of `{kind,id,reason}` objects. Uses `serde_json::Value` (no derive) so this stays inside
    /// the `doc` feature without a `serde::Serialize` dependency on these types.
    #[must_use]
    pub fn to_json_string(&self) -> String {
        let skipped: Vec<serde_json::Value> = self
            .skipped
            .iter()
            .map(|(kind, id, reason)| {
                serde_json::json!({ "kind": kind, "id": id, "reason": reason })
            })
            .collect();
        serde_json::json!({
            "slots_added": self.slots_added,
            "squads_merged": self.squads_merged,
            "squads_created": self.squads_created,
            "factions_merged": self.factions_merged,
            "factions_created": self.factions_created,
            "vehicles_added": self.vehicles_added,
            "entities_added": self.entities_added,
            "zones_added": self.zones_added,
            "triggers_added": self.triggers_added,
            "compositions_added": self.compositions_added,
            "markers_added": self.markers_added,
            "skipped": skipped,
        })
        .to_string()
    }
}

/// T-693 — the id re-mint + dedup table for one merge. Maps each incoming id to the id it becomes in
/// the current doc: either a FRESH minted id (a created row) or a RESIDENT id (a deduped faction /
/// squad — the "merged" case). `merged` remembers which mappings were dedup so the writer can skip
/// creating a row for them.
///
/// # Collision-proofing (BLOCKER-1 fix)
///
/// A minted id must exist NOWHERE the merge could later insert against: not the resident doc's ids,
/// not the ids of THIS payload, and not a PRIOR merge's minted ids. The naive `mrg-<seq>-<old>` from a
/// per-call `seq` was not: merging the same template twice makes the same dedup decisions, so the same
/// `<old>` reaches `ensure_fresh` at the same `<seq>` and mints the SAME `mrg-<seq>-<old>` the first
/// merge already resident — a `MapRef::insert` on that key OVERWRITES the first merge's row (its edits
/// lost) while `append_id` double-appends the id. `taken` closes that: it is seeded (via
/// [`RemintMap::with_reserved`]) with the doc's whole id universe — every resident row id across
/// slots/squads/factions/layers/vehicles/entities/zones/triggers/compositions/markers, collected in
/// Pass 0 with the same pre-txn hoist the deadlock fix uses — and every id this call mints is added to
/// it, so `ensure_fresh` bumps `seq` past any candidate already present (a resident `mrg-1-s0` from a
/// first merge, or an intra-payload duplicate id) until the id is free everywhere. The second merge
/// then mints `mrg-2-s0` (or higher), lands ALONGSIDE the first, and the report counts are true.
struct RemintMap {
    map: HashMap<String, String>,
    merged: HashSet<String>,
    /// Every id that already exists somewhere the merge must not collide with: the resident doc's id
    /// universe (seeded once) plus every id minted so far this call. A mint is rejected until it is
    /// absent here — this is what makes minting collision-proof against a prior merge's residents.
    taken: HashSet<String>,
    seq: u64,
}

impl RemintMap {
    /// A re-mint table that only knows it must avoid the ids it mints (empty resident universe). Used
    /// where no doc is in play.
    #[cfg(test)]
    fn new() -> Self {
        Self::with_reserved(HashSet::new())
    }

    /// A re-mint table seeded with the doc's full resident id universe, so no minted id can ever equal
    /// an id already resident (the collision the twice-merged-template case hit). `reserved` is
    /// collected before the write txn opens (the deadlock-safe hoist).
    fn with_reserved(reserved: HashSet<String>) -> Self {
        Self {
            map: HashMap::new(),
            merged: HashSet::new(),
            taken: reserved,
            seq: 0,
        }
    }

    /// Map `old` onto an existing resident id (dedup): the incoming row IS the resident one. Idempotent
    /// — a later `ensure_fresh` on the same id is a no-op, so the dedup decision wins.
    fn map_to_existing(&mut self, old: &str, resident: &str) {
        self.map.insert(old.to_string(), resident.to_string());
        self.merged.insert(old.to_string());
    }

    /// Reserve a FRESH re-minted id for `old` unless it is already mapped (fresh or deduped). The
    /// minted id is guaranteed absent from `taken` — the resident id universe plus every id already
    /// minted this call — by bumping `seq` past any collision, then recorded in `taken` so no later
    /// mint (this call or a subsequent merge that seeds `taken` from the doc) can reproduce it.
    fn ensure_fresh(&mut self, old: &str) {
        if self.map.contains_key(old) {
            return;
        }
        let fresh = loop {
            self.seq += 1;
            let candidate = format!("mrg-{}-{}", self.seq, old);
            if !self.taken.contains(&candidate) {
                break candidate;
            }
        };
        self.taken.insert(fresh.clone());
        self.map.insert(old.to_string(), fresh);
    }

    /// The id `old` becomes, if it is in the payload's id space.
    fn get(&self, old: &str) -> Option<String> {
        self.map.get(old).cloned()
    }
}

/// T-693 — did this incoming faction/squad id dedup onto a resident row (so no new row is created)?
fn squad_or_faction_is_merged(remint: &RemintMap, old_id: &str) -> bool {
    remint.merged.contains(old_id)
}

/// T-693 — a string field off an `Any::Map` row (from `ordered_rows`), or `None`.
fn any_map_str(m: &HashMap<String, Any>, key: &str) -> Option<String> {
    match m.get(key) {
        Some(Any::String(s)) => Some(s.to_string()),
        _ => None,
    }
}

/// T-693 — a string field off a `serde_json` object row, or `None`.
fn json_str(m: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    m.get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// T-693 — a number field off a `serde_json` object row (0.0 when absent/non-number).
fn json_num(m: &serde_json::Map<String, serde_json::Value>, key: &str) -> f64 {
    m.get(key)
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
}

/// T-693 — `(x, y, z, rotation)` off a `serde_json` row's `position` sub-object (0 for absent keys).
fn json_position(m: &serde_json::Map<String, serde_json::Value>) -> (f64, f64, f64, f64) {
    let pos = m.get("position").and_then(serde_json::Value::as_object);
    let g = |k: &str| {
        pos.and_then(|p| p.get(k))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
    };
    (g("x"), g("y"), g("z"), g("rotation"))
}

/// T-693 — a row's whole `position` sub-object as an owned `Any::Map` payload, so unknown sub-keys
/// (`heading`, `source`, …) survive the offset rewrite exactly as `position_any_merged` preserves them
/// on an in-doc edit. The four known coords are OVERWRITTEN by the caller after this returns.
fn json_position_map(m: &serde_json::Map<String, serde_json::Value>) -> HashMap<String, Any> {
    match m.get("position") {
        Some(serde_json::Value::Object(pos)) => pos
            .iter()
            .map(|(k, v)| (k.clone(), value_to_any(v)))
            .collect(),
        _ => HashMap::new(),
    }
}

/// T-693 — copy every field of a `serde_json` row into a freshly-created yrs entity map, EXCEPT the
/// keys in `skip`. `skip` names `id` plus every field that carries a doc reference (so the caller can
/// write the RE-MINTED value itself) or the `position` (so the caller can apply the offset). Nested
/// objects/arrays go in opaque via [`value_to_any`], exactly like [`load_row`].
fn copy_row_fields_except(
    txn: &mut TransactionMut,
    entity: &MapRef,
    row: &serde_json::Map<String, serde_json::Value>,
    skip: &[&str],
) {
    for (k, v) in row {
        if skip.contains(&k.as_str()) {
            continue;
        }
        entity.insert(txn, k.as_str(), value_to_any(v));
    }
}

/// T-693 — offset a `$defs/shape` object (`{circle:{x,z,r}}` or `{polygon:[[x,z],…]}`) by `(dx, dy)`
/// in world meters, returning a fresh opaque `Any::Map`. Anything that is not a recognized shape is
/// copied verbatim (tolerant: a malformed shape rides along rather than failing the row).
fn offset_shape_any(shape: &serde_json::Value, dx: f64, dy: f64) -> Any {
    let Some(obj) = shape.as_object() else {
        return value_to_any(shape);
    };
    if let Some(circle) = obj.get("circle").and_then(serde_json::Value::as_object) {
        let g = |k: &str| {
            circle
                .get(k)
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
        };
        return circle_shape_any(g("x") + dx, g("z") + dy, g("r"));
    }
    if let Some(poly) = obj.get("polygon").and_then(serde_json::Value::as_array) {
        let ring: Vec<Any> = poly
            .iter()
            .filter_map(serde_json::Value::as_array)
            .filter(|p| p.len() == 2)
            .map(|p| {
                let x = p[0].as_f64().unwrap_or(0.0) + dx;
                let z = p[1].as_f64().unwrap_or(0.0) + dy;
                Any::Array(vec![Any::Number(x), Any::Number(z)].into())
            })
            .collect();
        return Any::Map(Arc::new(HashMap::from([(
            "polygon".to_string(),
            Any::Array(ring.into()),
        )])));
    }
    value_to_any(shape)
}

/// T-693 — merge a run of shape-bearing rows (currently zones) into `map`: re-mint id, copy every
/// other field verbatim, and offset the `shape` geometry. Returns how many landed; malformed rows are
/// pushed onto `skipped` (`kind`, id, reason) rather than panicking. Triggers are written inline in
/// `merge_mission_payload` because they carry the extra `ownerId` re-mint.
#[allow(clippy::too_many_arguments)]
fn merge_shape_rows(
    txn: &mut TransactionMut,
    map: &MapRef,
    rows: Option<&serde_json::Value>,
    remint: &RemintMap,
    dx: f64,
    dy: f64,
    kind: &str,
    skipped: &mut Vec<(String, String, String)>,
) -> u32 {
    let mut added = 0;
    let Some(rows) = rows.and_then(serde_json::Value::as_array) else {
        return 0;
    };
    for row in rows {
        let Some(m) = row.as_object() else {
            skipped.push((kind.to_string(), String::new(), "not an object".to_string()));
            continue;
        };
        let Some(old_id) = json_str(m, "id") else {
            skipped.push((kind.to_string(), String::new(), "missing id".to_string()));
            continue;
        };
        let new_id = remint.get(&old_id).unwrap_or_else(|| old_id.clone());
        let entity = map.insert(
            txn,
            new_id.as_str(),
            MapPrelim::from([("id", new_id.as_str())]),
        );
        for (k, v) in m {
            match k.as_str() {
                "id" => {}
                "shape" => {
                    entity.insert(txn, "shape", offset_shape_any(v, dx, dy));
                }
                _ => {
                    entity.insert(txn, k.as_str(), value_to_any(v));
                }
            };
        }
        added += 1;
    }
    added
}

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

/// T-491 — slot delta apply inside an existing txn (shared by [`MissionDocCore::move_entities`]
/// and [`MissionDocCore::move_entities_and_vehicles`]).
///
/// T-665 — a slot on a **locked** layer (or one whose ancestor layer is locked) is silently skipped:
/// its `position` is not rewritten. "Silently" follows Eden — the move just doesn't happen, there is
/// no error; a caller that wants to surface the refusal reads back the returned skipped count. An
/// unfiled slot has no layer and is always movable.
fn move_entities_in_txn(
    txn: &mut TransactionMut,
    slots: &MapRef,
    editor_layers: &MapRef,
    ids: &[String],
    dx: f64,
    dy: f64,
    zs: &[f64],
) {
    for (i, id) in ids.iter().enumerate() {
        if slot_is_transform_locked(&*txn, editor_layers, id) {
            continue; // transform-locked: refuse the move for this slot
        }
        if let Some(Out::YMap(slot)) = slots.get(&*txn, id.as_str()) {
            let (px, py, _pz, prot) = read_position(txn, &slot);
            let z = zs.get(i).copied().unwrap_or(0.0);
            let existing = read_position_map(txn, &slot);
            slot.insert(
                &mut *txn,
                "position",
                position_any_merged(existing, px + dx, py + dy, z, prot),
            );
        }
    }
}

/// T-665 — read a layer's own boolean flag (`hidden` / `locked`), false when absent or non-bool.
/// The flag is stored only when `true` (the setters remove the key on `false`), so "absent" is the
/// canonical negative — matching the `add_slot` `tag`/`assetId` omit idiom.
fn layer_flag<T: ReadTxn>(txn: &T, editor_layers: &MapRef, layer_id: &str, flag: &str) -> bool {
    matches!(
        editor_layers.get(txn, layer_id).and_then(|o| match o {
            Out::YMap(layer) => layer.get(txn, flag),
            _ => None,
        }),
        Some(Out::Any(Any::Bool(true)))
    )
}

/// T-665 — does `layer_id` OR any of its ancestors carry `flag`? Walks up via `parentId`, so a
/// child inherits an ancestor's hidden/locked state **effectively** without the flag ever being
/// copied down onto the child row (resolve-at-read; hiding/locking a parent covers its whole
/// subtree, and un-flagging the parent reveals/unlocks it again). The `seen` set makes a malformed
/// `parentId` cycle terminate instead of hanging — belt-and-braces beside
/// [`MissionDocCore::is_layer_descendant`]'s own cycle guard on the writer.
fn layer_flag_effective<T: ReadTxn>(
    txn: &T,
    editor_layers: &MapRef,
    layer_id: &str,
    flag: &str,
) -> bool {
    let mut seen: HashSet<String> = HashSet::new();
    let mut cur = Some(layer_id.to_string());
    while let Some(c) = cur {
        if !seen.insert(c.clone()) {
            return false; // cycle — stop rather than loop forever
        }
        if layer_flag(txn, editor_layers, &c, flag) {
            return true;
        }
        cur = match editor_layers.get(txn, &c) {
            Some(Out::YMap(layer)) => match layer.get(txn, "parentId") {
                Some(Out::Any(Any::String(p))) => Some(p.to_string()),
                _ => None,
            },
            _ => None,
        };
    }
    false
}

/// T-665 — the layerId a slot resolves to (its first Outliner folder), or `None` when it is filed
/// nowhere. Identical rule to [`MissionDocCore::materialize`]'s reverse index: the FIRST layer whose
/// `entityIds` lists the slot wins. A slot in no layer is unfiled, hence never hidden or locked by
/// inheritance — flags only reach it through a folder it actually lives in.
fn slot_first_layer<T: ReadTxn>(txn: &T, editor_layers: &MapRef, slot_id: &str) -> Option<String> {
    for (layer_id, out) in editor_layers.iter(txn) {
        if let Out::YMap(layer) = out
            && let Some(Out::Any(Any::Array(arr))) = layer.get(txn, "entityIds")
            && arr
                .iter()
                .any(|a| matches!(a, Any::String(s) if s.as_ref() == slot_id))
        {
            return Some(layer_id.to_string());
        }
    }
    None
}

/// T-665 — is `slot_id`'s resolved layer (or any ancestor) locked? An unfiled slot (no layer) is
/// never locked. Backs the transform-lock refusal shared by [`MissionDocCore::move_entities`],
/// [`MissionDocCore::move_entities_and_vehicles`] and [`MissionDocCore::update_slot_position`].
fn slot_is_transform_locked<T: ReadTxn>(txn: &T, editor_layers: &MapRef, slot_id: &str) -> bool {
    match slot_first_layer(txn, editor_layers, slot_id) {
        Some(layer_id) => layer_flag_effective(txn, editor_layers, &layer_id, "locked"),
        None => false,
    }
}

/// T-491 — vehicle delta apply inside an existing txn (shared by [`MissionDocCore::move_vehicles`]
/// and [`MissionDocCore::move_entities_and_vehicles`]).
fn move_vehicles_in_txn(
    txn: &mut TransactionMut,
    vehicles: &MapRef,
    ids: &[String],
    dx: f64,
    dy: f64,
) {
    for id in ids {
        let Some(Out::YMap(v)) = vehicles.get(&*txn, id.as_str()) else {
            continue;
        };
        if v.get(&*txn, "position").is_none() {
            continue;
        }
        let (px, py, pz, prot) = read_position(txn, &v);
        let existing = read_position_map(txn, &v);
        v.insert(
            &mut *txn,
            "position",
            position_any_merged(existing, px + dx, py + dy, pz, prot),
        );
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
///
/// **T-211 — `zones` is in THIS list and deliberately NOT (yet) in compile.rs's.** The two lists
/// answer different questions and this is the one window where the answers differ:
///
/// * here it means "hydrate UNDERSTANDS this key", and it does — `hydrate` loads `zones[]` into
///   the `zones` root map. Parking it in `payloadExtras` as well would give the wire two sources
///   for one key, and the stale parked copy would beat every edit made after load.
/// * in `compile.rs` it means "compile AUTHORS this key, so never promote it from extras". Compile
///   does not author `zones` yet, so it MUST stay absent there or the projection
///   `small_maps_json` writes would be skipped and every authored zone would be dropped on save.
///
/// The divergence is therefore load-bearing in exactly one direction, and it closes itself: the
/// companion `compile.rs` change adds `zones` to that list AND emits it from `zonesById` in the
/// same edit, at which point both lists agree and the projection retires. Do not "fix" the
/// asymmetry by adding `zones` to compile.rs's list alone — that combination drops authored zones
/// silently, which is the one failure mode this whole arrangement exists to avoid.
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
            // T-211 — hydrate loads top-level `zones[]` into the `zones` root. See this fn's note
            // on why compile.rs's twin list does NOT list this key yet.
            | "zones"
            // T-650 — hydrate loads top-level `compositions[]` into the `compositions` root. Like
            // `zones`, compile.rs's twin list does NOT list this key yet — the transitional
            // `payloadExtras.compositions` projection in `small_maps_json` closes the round trip
            // until it does.
            | "compositions"
            // T-079 — hydrate loads top-level `triggers[]` into the `triggers` root. Like `zones` /
            // `compositions`, compile.rs's twin list does NOT list this key yet: the schema does not
            // declare `triggers` until T-706 (wave 120), so the transitional
            // `payloadExtras.triggers` projection in `small_maps_json` closes the round trip until
            // then. Do NOT add `triggers` to compile.rs's list before it emits from `triggersById`,
            // or every authored trigger drops on save (the exact failure the zones note warns of).
            | "triggers"
            // T-651 — hydrate loads top-level `comments[]` into the `comments` root. compile.rs's
            // twin list does NOT list this key, and — unlike `zones` / `compositions` / `triggers` —
            // it never will: `comments` is EDITOR-ONLY, so `compile_payload` must keep promoting the
            // `payloadExtras.comments` projection forever. Adding `comments` to compile.rs's list
            // without teaching it to author the key would drop every authored comment on save (the
            // exact failure the zones note warns of); adding it WITH an author would be a request to
            // compile an annotation, which is the one thing this collection must never do.
            | "comments"
            // T-672 — hydrate loads top-level `connections[]` into the `connections` root. Exactly
            // the `comments` case, for exactly the `comments` reason: compile.rs's twin list does NOT
            // list this key and never will, because the schema declares no relation collection for
            // an edge to compile INTO. Adding `connections` to compile.rs's list without teaching it
            // to author the key would drop every drawn edge on save; adding it WITH an author would
            // be a request to compile a relation the mod document has no field for.
            | "connections"
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

/// T-211 — a yrs by-id root map as an ordered row array, replaying `entityOrder[order_key]` first
/// and appending anything that order does not name.
///
/// This is the `Any` twin of `mission/compile.rs`'s `values_of_ordered`, and it is deliberately a
/// SEPARATE implementation rather than a shared one: `doc` must not depend on `mission` (the same
/// reason `is_known_editor_payload_top_level` is duplicated). The two must agree on ORDER, and the
/// three-part contract they share is (1) `entityOrder` names the authored sequence, (2) ids in that
/// order but absent from the map are skipped, (3) ids in the map but absent from the order are
/// appended in map-iteration order. `zonesById_and_extras_projection_agree_on_order` pins that.
fn ordered_rows(
    txn: &impl ReadTxn,
    map: &MapRef,
    entity_order: &MapRef,
    order_key: &str,
) -> Vec<Any> {
    let by_id: HashMap<String, Any> = map
        .iter(txn)
        .map(|(id, out)| (id.to_string(), out.to_json(txn)))
        .collect();
    if by_id.is_empty() {
        return Vec::new();
    }
    let order: Option<Vec<String>> = match entity_order.get(txn, order_key) {
        Some(Out::Any(Any::Array(arr))) => Some(
            arr.iter()
                .filter_map(|v| match v {
                    Any::String(s) => Some(s.to_string()),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    };
    let Some(order) = order else {
        // No authored order — map-iteration order, exactly like `values_of_ordered`'s early return.
        return map.iter(txn).map(|(_, out)| out.to_json(txn)).collect();
    };

    let mut seen: HashSet<&str> = HashSet::new();
    let mut out: Vec<Any> = Vec::with_capacity(by_id.len());
    for id in &order {
        if let Some(row) = by_id.get(id.as_str())
            && seen.insert(id.as_str())
        {
            out.push(row.clone());
        }
    }
    for (id, row) in map.iter(txn) {
        if !seen.contains(id) {
            out.push(row.to_json(txn));
        }
    }
    out
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

/// A vehicle's `crew` map (`seat_id → slot_id`) as an OWNED plain map, tolerant of BOTH doc shapes
/// (T-076 wave-103): a freshly-authored vehicle carries `crew` as a tracked `YMap`, but after a
/// server-adopt boot [`load_row`] re-inserts it as an opaque `Any::Map`. The crew mutators must read
/// through this — matching only `Out::YMap` made unboard a no-op and made a board WIPE the loaded
/// crew on any hydrated mission. This is [`read_any_map`] plus the `YMap` case, since crew is the one
/// nested map that is *also* live-tracked before its first hydrate.
fn read_crew_map<T: ReadTxn>(txn: &T, vehicle: &MapRef) -> HashMap<String, Any> {
    match vehicle.get(txn, "crew") {
        Some(Out::Any(Any::Map(m))) => (*m).clone(),
        Some(Out::YMap(crew)) => crew
            .iter(txn)
            .filter_map(|(seat, occ)| match occ {
                Out::Any(a) => Some((seat.to_string(), a)),
                _ => None,
            })
            .collect(),
        _ => HashMap::new(),
    }
}

/// Write a vehicle's `crew` map back WHOLE (the read-modify-write half of [`read_crew_map`]), or drop
/// the key when the map is empty — the `cargo`/`tag` omit idiom, so an emptied crew leaves the row
/// byte-identical to a never-crewed one. Always writing the whole `Any::Map` makes the seat edit
/// hydrate-proof: it does not depend on `crew` being a tracked `YMap`, so a board/unboard on a
/// server-adopted mission behaves exactly like one on a freshly-authored doc.
fn write_crew_map(txn: &mut TransactionMut, vehicle: &MapRef, crew: HashMap<String, Any>) {
    if crew.is_empty() {
        vehicle.remove(txn, "crew");
    } else {
        vehicle.insert(txn, "crew", Any::Map(Arc::new(crew)));
    }
}

/// T-650 — one composition ROW as an OWNED plain map, tolerant of BOTH doc shapes (the crew-reader
/// idiom): a freshly-saved composition is inserted as an opaque `Any::Map`, and after a hydrate the
/// same row comes back through [`load_row`] as a tracked `YMap` (its top-level keys become tracked;
/// the nested `entities` array stays opaque). The metadata-edit mutators must read through this so a
/// rename/recategorize after a reload modifies the loaded row instead of no-opping or wiping it.
/// `None` when the id is absent.
/// T-651 — build a comment row `{id, title, tooltip, position:{x,z}}`. One constructor so a place, a
/// duplicate and a template seed cannot disagree about the shape.
fn comment_row(id: &str, title: &str, tooltip: &str, x: f64, z: f64) -> HashMap<String, Any> {
    let mut pos: HashMap<String, Any> = HashMap::new();
    pos.insert("x".to_string(), Any::Number(x));
    pos.insert("z".to_string(), Any::Number(z));
    let mut row: HashMap<String, Any> = HashMap::new();
    row.insert("id".to_string(), Any::String(id.into()));
    row.insert("title".to_string(), Any::String(title.into()));
    row.insert("tooltip".to_string(), Any::String(tooltip.into()));
    row.insert("position".to_string(), Any::Map(Arc::new(pos)));
    row
}

/// T-651 — read a comment row whole, tolerating BOTH the freshly-written opaque `Any::Map` and the
/// tracked `YMap` a hydrate can materialise. The [`read_composition_map`] idiom, same reason.
fn read_comment_map<T: ReadTxn>(
    txn: &T,
    comments: &MapRef,
    id: &str,
) -> Option<HashMap<String, Any>> {
    match comments.get(txn, id) {
        Some(Out::Any(Any::Map(m))) => Some((*m).clone()),
        Some(Out::YMap(row)) => Some(
            row.iter(txn)
                .map(|(k, out)| match out {
                    Out::Any(a) => (k.to_string(), a),
                    other => (k.to_string(), other.to_json(txn)),
                })
                .collect(),
        ),
        _ => None,
    }
}

/// T-651 — a comment row's `position.{x,z}`; `(0.0, 0.0)` when absent or non-numeric. `Any::BigInt`
/// is accepted because [`json_str_to_any`] encodes integer-valued JSON numbers that way, so a
/// hydrated `"x": 6400` arrives as a `BigInt` and a naive `Number`-only read would silently zero it.
fn comment_xz(row: &HashMap<String, Any>) -> (f64, f64) {
    let Some(Any::Map(pos)) = row.get("position") else {
        return (0.0, 0.0);
    };
    let num = |k: &str| match pos.get(k) {
        Some(Any::Number(n)) => *n,
        #[allow(clippy::cast_precision_loss)] // ids/coords are far inside f64's exact-integer range
        Some(Any::BigInt(i)) => *i as f64,
        _ => 0.0,
    };
    (num("x"), num("z"))
}

/// T-651 — a comment row's string field, or `""`.
fn comment_str(row: &HashMap<String, Any>, key: &str) -> String {
    match row.get(key) {
        Some(Any::String(s)) => s.to_string(),
        _ => String::new(),
    }
}

/* ═══════════════ T-672 — the connection graph's vocabulary, rows, checker + formation ═══════════ */

/// T-672 — the three relations Eden's `Connect ▸` submenu can make. **This enum is the whole
/// vocabulary**, parsed at the one write door ([`MissionDocCore::add_connection`]), so a fourth
/// spelling cannot enter the document by a typo at a call site — the T-241 single-vocabulary rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    /// `Sync to` — a symmetric peer relation between two placed things. UNDIRECTED.
    Sync,
    /// `Group to` — `from` joins `to`'s group. Directed; the graph must stay acyclic.
    Group,
    /// `Set Trigger Owner` — `to` owns `from`. Directed; the graph must stay acyclic.
    TriggerOwner,
}

impl ConnectionKind {
    /// Parse the stored/wire token. `None` for anything else — an unknown kind is refused rather
    /// than coerced, because coercing would silently turn a typo into a relation the author did not
    /// ask for and cannot see the difference of.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "sync" => Some(Self::Sync),
            "group" => Some(Self::Group),
            "triggerOwner" => Some(Self::TriggerOwner),
            _ => None,
        }
    }

    /// The stored token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sync => "sync",
            Self::Group => "group",
            Self::TriggerOwner => "triggerOwner",
        }
    }

    /// Whether `from`→`to` has a direction. `sync` does not: it is a peer relation, which is why it
    /// is normalised at write and excluded from the `CONN-CYCLE` rule (a "cycle" of peers is just a
    /// connected component, and flagging it would be noise on a correct graph).
    #[must_use]
    pub const fn is_directed(self) -> bool {
        !matches!(self, Self::Sync)
    }

    /// Canonical endpoint order for storage: undirected kinds sort their endpoints so `sync(B,A)`
    /// and `sync(A,B)` are the SAME row, which is the only thing that lets the duplicate guard see
    /// a reversed re-draw. Directed kinds are stored verbatim.
    #[must_use]
    fn normalise(self, from: &str, to: &str) -> (String, String) {
        if self.is_directed() || from <= to {
            (from.to_string(), to.to_string())
        } else {
            (to.to_string(), from.to_string())
        }
    }
}

/// T-672 — one connection as every reader wants it. `kind` stays a `String` (not a
/// [`ConnectionKind`]) on purpose: a hydrated document can carry a kind this build does not know,
/// and the listing must still SHOW it — dropping unreadable rows from the inspector is how a graph
/// becomes unauditable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionRow {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
}

/// T-672 — one validation finding. `code` is a stable id (`CONN-SELF`, `CONN-DANGLING`,
/// `CONN-DUPLICATE`, `CONN-CYCLE`, `CONN-KIND`); `detail` is the human half.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionFinding {
    pub code: &'static str,
    pub connection_id: String,
    pub detail: String,
}

/// T-672 (CHECK) — **the graph validator, as a pure function over rows.**
///
/// Pure and taking its inputs explicitly so the rules are tested against hand-built graphs with no
/// yrs document in the way — the FNF v4 warning is specifically that this mechanism's defects hide,
/// and a checker that can only be exercised through a live document is a checker nobody exercises.
/// [`MissionDocCore::connection_findings_json`] is a thin adapter over this.
///
/// `rows` must be in the stable order [`MissionDocCore::connection_rows`] produces: `CONN-DUPLICATE`
/// names the SECOND and later rows of a repeated `(kind, from, to)`, so which row survives is a
/// function of that order.
///
/// Findings are sorted `(code, connection_id)` before returning.
#[must_use]
pub fn validate_connection_rows(
    rows: &[ConnectionRow],
    known_ids: &HashSet<String>,
) -> Vec<ConnectionFinding> {
    let mut out: Vec<ConnectionFinding> = Vec::new();
    let mut seen: HashSet<(String, String, String)> = HashSet::new();

    for r in rows {
        if ConnectionKind::parse(&r.kind).is_none() {
            out.push(ConnectionFinding {
                code: "CONN-KIND",
                connection_id: r.id.clone(),
                detail: format!("unknown connection kind `{}`", r.kind),
            });
        }
        if !r.from.is_empty() && r.from == r.to {
            out.push(ConnectionFinding {
                code: "CONN-SELF",
                connection_id: r.id.clone(),
                detail: format!("`{}` is connected to itself", r.from),
            });
        }
        for (end, id) in [("from", &r.from), ("to", &r.to)] {
            if id.is_empty() || !known_ids.contains(id) {
                out.push(ConnectionFinding {
                    code: "CONN-DANGLING",
                    connection_id: r.id.clone(),
                    detail: format!("{end} endpoint `{id}` is not a placed entity"),
                });
            }
        }
        let key = (r.kind.clone(), r.from.clone(), r.to.clone());
        if !seen.insert(key) {
            out.push(ConnectionFinding {
                code: "CONN-DUPLICATE",
                connection_id: r.id.clone(),
                detail: format!(
                    "`{}` → `{}` is already connected ({})",
                    r.from, r.to, r.kind
                ),
            });
        }
    }

    out.extend(cycle_findings(rows));
    out.sort_by(|a, b| (a.code, &a.connection_id).cmp(&(b.code, &b.connection_id)));
    out
}

/// T-672 — `CONN-CYCLE` over the DIRECTED subgraph only (`group` / `triggerOwner`).
///
/// Iterative three-colour DFS (white = unvisited, grey = on the current stack, black = finished): an
/// edge into a GREY node is a back edge and closes a cycle, so that edge is the finding. Self-links
/// are skipped here — they are already `CONN-SELF`, and reporting the same row twice under two codes
/// would make the panel's count wrong.
///
/// Iterative rather than recursive because the depth is the author's, not ours: a 300-unit chain
/// authored as one ownership line is legal input, and a recursive walk would blow the stack on a
/// document rather than report on it.
fn cycle_findings(rows: &[ConnectionRow]) -> Vec<ConnectionFinding> {
    // node → [(target, connection id)] for directed kinds only.
    let mut adj: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    for r in rows {
        let directed = ConnectionKind::parse(&r.kind).is_some_and(ConnectionKind::is_directed);
        if !directed || r.from == r.to || r.from.is_empty() || r.to.is_empty() {
            continue;
        }
        adj.entry(r.from.as_str())
            .or_default()
            .push((r.to.as_str(), r.id.as_str()));
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Colour {
        Grey,
        Black,
    }
    let mut colour: HashMap<&str, Colour> = HashMap::new();
    let mut out: Vec<ConnectionFinding> = Vec::new();
    let mut roots: Vec<&str> = adj.keys().copied().collect();
    roots.sort_unstable();

    for root in roots {
        if colour.contains_key(root) {
            continue;
        }
        // Stack frames are (node, index of the next outgoing edge to walk).
        let mut stack: Vec<(&str, usize)> = vec![(root, 0)];
        colour.insert(root, Colour::Grey);
        while let Some((node, edge_idx)) = stack.pop() {
            let edges = adj.get(node).map_or(&[][..], Vec::as_slice);
            if edge_idx >= edges.len() {
                colour.insert(node, Colour::Black);
                continue;
            }
            stack.push((node, edge_idx + 1));
            let (target, conn_id) = edges[edge_idx];
            match colour.get(target) {
                Some(Colour::Grey) => out.push(ConnectionFinding {
                    code: "CONN-CYCLE",
                    connection_id: conn_id.to_string(),
                    detail: format!("`{node}` → `{target}` closes an ownership cycle"),
                }),
                Some(Colour::Black) => {}
                None => {
                    colour.insert(target, Colour::Grey);
                    stack.push((target, 0));
                }
            }
        }
    }
    out
}

/// T-672 — build a connection row `{id, kind, from, to}`. One constructor so the write path and any
/// future seed cannot disagree about the shape.
fn connection_row(id: &str, kind: &str, from: &str, to: &str) -> HashMap<String, Any> {
    let mut row: HashMap<String, Any> = HashMap::new();
    row.insert("id".to_string(), Any::String(id.into()));
    row.insert("kind".to_string(), Any::String(kind.into()));
    row.insert("from".to_string(), Any::String(from.into()));
    row.insert("to".to_string(), Any::String(to.into()));
    row
}

/// T-672 — read a connection row whole, tolerating BOTH the freshly-written opaque `Any::Map` and
/// the tracked `YMap` a hydrate materialises. The [`read_comment_map`] idiom, same reason: the
/// listing and the checker must see a reloaded edge exactly as they see a freshly drawn one, or the
/// graph you can check is not the graph you saved.
fn read_connection_map<T: ReadTxn>(
    txn: &T,
    connections: &MapRef,
    id: &str,
) -> Option<HashMap<String, Any>> {
    match connections.get(txn, id) {
        Some(Out::Any(Any::Map(m))) => Some((*m).clone()),
        Some(Out::YMap(row)) => Some(
            row.iter(txn)
                .map(|(k, out)| match out {
                    Out::Any(a) => (k.to_string(), a),
                    other => (k.to_string(), other.to_json(txn)),
                })
                .collect(),
        ),
        _ => None,
    }
}

/// T-672 — metres between neighbouring positions in a formation. One constant so every formation
/// scales together; a per-formation spacing would make `wedge` and `line` incomparable at a glance.
pub const FORMATION_SPACING_M: f64 = 10.0;

/// T-672 (`ACTION-FORM-001`) — **the formation geometry, pure.** `n` BODY-FRAME offsets `(x, y)` in
/// metres for the `n` non-leader members, leader at the origin, `+y` forward and `+x` right.
/// [`MissionDocCore::force_to_formation`] rotates these by the leader's heading; keeping the two
/// apart is what lets the shapes be tested with no document and no trigonometry in the assertions.
///
/// `formation` takes the **schema's own vocabulary** — `$defs/group.formation`'s nine-token enum
/// (`column`, `stagger_column`, `wedge`, `echelon_left`, `echelon_right`, `vee`, `line`, `file`,
/// `diamond`). Reusing that list rather than inventing a parallel one is the T-241 rule: the wire
/// already names these, so the editor must not name them a second way.
///
/// An unknown or empty token falls back to `column` — deliberately the shape whose result is
/// unmistakable at any count (a straight trail behind the leader) and which cannot be confused with
/// any of the other eight. A typo therefore produces a visibly wrong arrangement the operator
/// notices, rather than a plausible formation they did not ask for.
#[must_use]
pub fn formation_offsets(formation: &str, n: usize) -> Vec<(f64, f64)> {
    let s = FORMATION_SPACING_M;
    // Rank (how far back) and pair index for the alternating-side shapes.
    let alternating = |i: usize| -> (f64, f64) {
        #[allow(clippy::cast_precision_loss)] // squad sizes are tens, not 2^53
        let rank = (i / 2 + 1) as f64;
        let side = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
        (side, rank)
    };
    #[allow(clippy::cast_precision_loss)]
    let trail = |i: usize| (i + 1) as f64;

    (0..n)
        .map(|i| match formation {
            // Behind the leader, alternating half a spacing left/right of the line of march.
            "stagger_column" => {
                let side = if i.is_multiple_of(2) { 0.5 } else { -0.5 };
                (side * s, -trail(i) * s)
            }
            // A V opening BACKWARD from the leader.
            "wedge" => {
                let (side, rank) = alternating(i);
                (side * rank * s, -rank * s)
            }
            // A V opening FORWARD — the wedge's mirror, leader at the back of the point.
            "vee" => {
                let (side, rank) = alternating(i);
                (side * rank * s, rank * s)
            }
            // A diagonal trailing to one side.
            "echelon_left" => (-trail(i) * s, -trail(i) * s),
            "echelon_right" => (trail(i) * s, -trail(i) * s),
            // Abreast of the leader, alternating sides so the squad stays centred on him.
            "line" => {
                let (side, rank) = alternating(i);
                (side * rank * s, 0.0)
            }
            // Single file, tight — half spacing, the difference from `column` that makes the two
            // distinguishable on the map instead of two names for one shape.
            "file" => (0.0, -trail(i) * s * 0.5),
            // Right, left, rear — repeating outward one ring at a time.
            "diamond" => {
                #[allow(clippy::cast_precision_loss)]
                let ring = (i / 3 + 1) as f64;
                match i % 3 {
                    0 => (ring * s, -ring * s),
                    1 => (-ring * s, -ring * s),
                    _ => (0.0, -2.0 * ring * s),
                }
            }
            // `column` and the documented fallback for anything unrecognised.
            _ => (0.0, -trail(i) * s),
        })
        .collect()
}

/// T-651 — detach `id` from every Outliner folder's `entityIds`. The "unfile" half shared by
/// [`MissionDocCore::move_slot_to_layer`] (which then appends to the target) and
/// [`MissionDocCore::remove_comment`] (which does not) — one implementation so a deleted entity
/// cannot survive as a dangling id in one path and not the other. Only layers that actually list
/// `id` are rewritten, so this is a no-op transaction-wise on an unfiled entity.
fn remove_id_from_all_layers(txn: &mut TransactionMut, editor_layers: &MapRef, id: &str) {
    let layer_ids: Vec<String> = editor_layers
        .iter(txn)
        .map(|(k, _)| k.to_string())
        .collect();
    for lid in &layer_ids {
        if let Some(Out::YMap(layer)) = editor_layers.get(txn, lid)
            && let Some(Out::Any(Any::Array(arr))) = layer.get(txn, "entityIds")
            && arr
                .iter()
                .any(|a| matches!(a, Any::String(s) if s.as_ref() == id))
        {
            let kept: Vec<Any> = arr
                .iter()
                .filter(|a| !matches!(a, Any::String(s) if s.as_ref() == id))
                .cloned()
                .collect();
            layer.insert(txn, "entityIds", Any::Array(kept.into()));
        }
    }
}

fn read_composition_map<T: ReadTxn>(
    txn: &T,
    compositions: &MapRef,
    id: &str,
) -> Option<HashMap<String, Any>> {
    match compositions.get(txn, id) {
        Some(Out::Any(Any::Map(m))) => Some((*m).clone()),
        Some(Out::YMap(row)) => Some(
            row.iter(txn)
                .map(|(k, out)| match out {
                    Out::Any(a) => (k.to_string(), a),
                    // A nested tracked map/array (yrs can materialize `entities` either way):
                    // re-serialize it whole so the metadata edit preserves it verbatim.
                    other => (k.to_string(), other.to_json(txn)),
                })
                .collect(),
        ),
        _ => None,
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

/// T-701 — read an entity row's own boolean flag (`editorHidden`), `false` when absent or non-bool.
/// The flag is stored only when `true` ([`MissionDocCore::set_slot_editor_hidden`] removes the key on
/// `false`), so "absent" is the canonical negative — the same `tag`/`assetId` omit idiom as
/// [`MissionDocCore::add_slot`], and the twin of the layer-side [`layer_flag`].
fn read_bool<T: ReadTxn>(txn: &T, row: &MapRef, key: &str) -> bool {
    matches!(row.get(txn, key), Some(Out::Any(Any::Bool(true))))
}

/// T-701 — write/clear one slot's `editorHidden` inside an existing txn (shared by the single, batch,
/// and clear-all setters so every path uses one omit rule). `true` inserts the bool; `false` REMOVES
/// the key so a never-hidden row stays byte-identical (absent ⇒ visible). No-op when the id is absent.
fn set_slot_editor_hidden_in_txn(txn: &mut TransactionMut, slots: &MapRef, id: &str, hidden: bool) {
    if let Some(Out::YMap(slot)) = slots.get(&*txn, id) {
        if hidden {
            slot.insert(&mut *txn, "editorHidden", true);
        } else {
            slot.remove(&mut *txn, "editorHidden");
        }
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

    /// T-491 Class-R — the T-425 host defect: `move_entities` then `move_vehicles` is **two** LOCAL
    /// txns, so one Ctrl+Z undoes only one kind. Pins the split-API shape that the host must not
    /// call for a mixed drag (see [`MissionDocCore::move_entities_and_vehicles`]).
    #[test]
    fn mixed_slot_vehicle_two_calls_are_two_undo_steps() {
        let mut doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_slot(
            "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
        );
        doc.add_vehicle(
            "v0",
            "Prefab/Vehicle.et",
            Some(300.0),
            Some(400.0),
            Some(0.0),
            Some(0.0),
        );
        doc.set_origin_init(false);
        assert_eq!(doc.undo_depth(), 0);

        doc.move_entities(vec!["s0".to_string()], 10.0, 20.0, vec![0.0]);
        doc.move_vehicles(&["v0".to_string()], 10.0, 20.0);
        assert_eq!(
            doc.undo_depth(),
            2,
            "T-425 split path: two LOCAL txns for one mixed drag"
        );

        assert!(doc.undo());
        let vehs = vehicles_of(&doc);
        let slots = doc.materialize();
        let i = row_of(&slots, "s0");
        // One undo reverts ONLY the vehicle move — slot stays at the dragged position.
        assert_eq!(
            slots.xs[i], 110.0,
            "slot still at post-drag x after one undo"
        );
        assert_eq!(
            slots.ys[i], 220.0,
            "slot still at post-drag y after one undo"
        );
        assert_eq!(vehs["v0"]["position"]["x"], 300.0, "vehicle undone");
        assert_eq!(vehs["v0"]["position"]["y"], 400.0, "vehicle undone");
        assert!(doc.can_undo(), "slot move still on the stack");
    }

    /// T-491 Class-R — one mixed drag = one LOCAL txn; one undo restores **both** slot and vehicle.
    #[test]
    fn mixed_slot_vehicle_atomic_move_is_one_undo_step() {
        let mut doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_slot(
            "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
        );
        doc.add_vehicle(
            "v0",
            "Prefab/Vehicle.et",
            Some(300.0),
            Some(400.0),
            Some(0.0),
            Some(45.0),
        );
        doc.set_origin_init(false);

        doc.move_entities_and_vehicles(
            vec!["s0".to_string()],
            &["v0".to_string()],
            10.0,
            20.0,
            vec![1.5],
        );
        assert_eq!(doc.undo_depth(), 1, "one mixed drag = one undo step");

        let slots = doc.materialize();
        let i = row_of(&slots, "s0");
        assert_eq!(slots.xs[i], 110.0);
        assert_eq!(slots.ys[i], 220.0);
        assert_eq!(slots.zs[i], 1.5, "slot z from zs[]");
        let vehs = vehicles_of(&doc);
        assert_eq!(vehs["v0"]["position"]["x"], 310.0);
        assert_eq!(vehs["v0"]["position"]["y"], 420.0);
        assert_eq!(vehs["v0"]["position"]["z"], 0.0, "vehicle z preserved");
        assert_eq!(
            vehs["v0"]["position"]["rotation"], 45.0,
            "vehicle rotation preserved"
        );

        assert!(doc.undo());
        assert_eq!(doc.undo_depth(), 0);
        let slots = doc.materialize();
        let i = row_of(&slots, "s0");
        assert_eq!(slots.xs[i], 100.0, "slot restored");
        assert_eq!(slots.ys[i], 200.0, "slot restored");
        let vehs = vehicles_of(&doc);
        assert_eq!(
            vehs["v0"]["position"]["x"], 300.0,
            "vehicle restored with slot"
        );
        assert_eq!(
            vehs["v0"]["position"]["y"], 400.0,
            "vehicle restored with slot"
        );
        assert_eq!(vehs["v0"]["position"]["rotation"], 45.0);
        assert!(!doc.can_undo());
    }

    /// T-574 — the **behavioural** pin that replaces T-491's soft `include_str!` string check.
    ///
    /// **The invariant:** `move_entities_and_vehicles` translates *every* named slot **and** *every*
    /// named placed vehicle by the same world delta inside **one** LOCAL yrs transaction — so a
    /// mixed drag is exactly one undo step that restores both kinds together, slot z comes from
    /// `zs[i]`, vehicle z/rotation survive, and ids that are absent or unplaced are skipped rather
    /// than teleported.
    ///
    /// It *calls the function*, so no source shape satisfies it. A comment-only body, a
    /// `#[cfg(any())]` and a gutted col-0 shadow copy do not compile; `if true == false`,
    /// `loop { break; … }` and a leading `return;` compile but move nothing. Deliberately **two**
    /// slots and **two** vehicles (T-491's test had one of each, so a body that only handled
    /// `ids[0]` would have passed) with distinct `zs`, so `zs[i]` cannot collapse to `zs[0]`.
    ///
    /// The one shape a same-crate unit test cannot see is a `#[cfg(not(test))]` twin: the doctest on
    /// [`MissionDocCore::move_entities_and_vehicles`] covers that, because doctests link the crate
    /// built without `--cfg test`.
    #[test]
    fn move_entities_and_vehicles_moves_every_slot_and_vehicle_in_one_txn() {
        let mut doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_slot(
            "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
        );
        doc.add_slot(
            "s1", "sq", "lyr", 1, "Rifleman", None, None, 500.0, 600.0, 0.0, 0.0,
        );
        doc.add_vehicle(
            "v0",
            "Prefab/Vehicle.et",
            Some(300.0),
            Some(400.0),
            Some(7.0),
            Some(45.0),
        );
        doc.add_vehicle(
            "v1",
            "Prefab/Vehicle.et",
            Some(900.0),
            Some(800.0),
            Some(-3.0),
            Some(270.0),
        );
        // An ORBAT-only vehicle (no `position`) — must stay unplaced, not land at the delta.
        doc.add_vehicle("v_orbat", "Prefab/Vehicle.et", None, None, None, None);
        doc.set_origin_init(false);
        assert_eq!(doc.undo_depth(), 0, "the INIT seed is not an undo step");

        let (dx, dy) = (10.0, -20.0);
        doc.move_entities_and_vehicles(
            vec!["s0".to_string(), "s1".to_string(), "s_absent".to_string()],
            &[
                "v0".to_string(),
                "v1".to_string(),
                "v_orbat".to_string(),
                "v_absent".to_string(),
            ],
            dx,
            dy,
            vec![1.5, 2.5, 3.5],
        );

        // ── every slot moved, each with its own zs[i] ──────────────────────────────────────────
        let slots = doc.materialize();
        for (id, x, y, z) in [("s0", 110.0, 180.0, 1.5), ("s1", 510.0, 580.0, 2.5)] {
            let i = row_of(&slots, id);
            assert_eq!(slots.xs[i], x, "{id}: x moved by dx");
            assert_eq!(slots.ys[i], y, "{id}: y moved by dy");
            assert_eq!(slots.zs[i], z, "{id}: z is zs[i], not zs[0]");
        }
        assert_eq!(slots.len(), 2, "an absent slot id must not mint a row");

        // ── every vehicle moved, z/rotation untouched ──────────────────────────────────────────
        let vehs = vehicles_of(&doc);
        for (id, x, y, z, rot) in [
            ("v0", 310.0, 380.0, 7.0, 45.0),
            ("v1", 910.0, 780.0, -3.0, 270.0),
        ] {
            assert_eq!(vehs[id]["position"]["x"], x, "{id}: x moved by dx");
            assert_eq!(vehs[id]["position"]["y"], y, "{id}: y moved by dy");
            assert_eq!(vehs[id]["position"]["z"], z, "{id}: z preserved");
            assert_eq!(
                vehs[id]["position"]["rotation"], rot,
                "{id}: rotation preserved"
            );
        }
        assert!(
            vehs["v_orbat"]["position"].is_null(),
            "an unplaced vehicle must not be given a position by a drag"
        );
        assert!(
            vehs["v_absent"].is_null(),
            "an absent vehicle id must not mint a row"
        );

        // ── ONE transaction. A body that moved both kinds correctly but under two `begin()` calls
        //    passes every assert above and dies here — that is the T-425 defect T-491 removed. ──
        assert_eq!(
            doc.undo_depth(),
            1,
            "one mixed drag must be ONE LOCAL txn (two ⇒ the T-425 two-Ctrl+Z defect)"
        );

        // ── and one undo restores BOTH kinds together (the user-visible half) ──────────────────
        assert!(doc.undo());
        assert_eq!(doc.undo_depth(), 0, "the whole drag came off in one undo");
        let slots = doc.materialize();
        assert_eq!(slots.xs[row_of(&slots, "s0")], 100.0, "s0 restored");
        assert_eq!(slots.ys[row_of(&slots, "s1")], 600.0, "s1 restored");
        let vehs = vehicles_of(&doc);
        assert_eq!(
            vehs["v0"]["position"]["x"], 300.0,
            "v0 restored by the SAME undo as the slots"
        );
        assert_eq!(
            vehs["v1"]["position"]["y"], 800.0,
            "v1 restored by the SAME undo as the slots"
        );
        assert!(!doc.can_undo(), "nothing left on the stack");

        // ── redo re-applies both kinds together ────────────────────────────────────────────────
        assert!(doc.redo());
        let slots = doc.materialize();
        assert_eq!(slots.xs[row_of(&slots, "s0")], 110.0, "s0 re-applied");
        let vehs = vehicles_of(&doc);
        assert_eq!(vehs["v1"]["position"]["x"], 910.0, "v1 re-applied");
    }

    /* ─────────────────────────────── T-076 — vehicle crew map ─────────────────────────────── */

    /// A doc with two placed vehicles and three placed slots, seeded under INIT so the setup is not
    /// on the undo stack — the crew tests then perturb with a single LOCAL board / unboard.
    fn two_vehicles_three_slots() -> MissionDocCore {
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        for id in ["s0", "s1", "s2"] {
            doc.add_slot(
                id, "sq", "L", 0, "Rifleman", None, None, 10.0, 20.0, 0.0, 0.0,
            );
        }
        doc.add_vehicle(
            "v0",
            "Prefab/A.et",
            Some(1.0),
            Some(2.0),
            Some(0.0),
            Some(0.0),
        );
        doc.add_vehicle(
            "v1",
            "Prefab/B.et",
            Some(3.0),
            Some(4.0),
            Some(0.0),
            Some(0.0),
        );
        doc.set_origin_init(false);
        doc
    }

    /// Read a vehicle's crew map (`{seat_id: slot_id}`) off the same `small_maps_json` the panel
    /// reads, or `Null` when the vehicle has never been crewed.
    fn crew_of(doc: &MissionDocCore, vehicle_id: &str) -> serde_json::Value {
        vehicles_of(doc)[vehicle_id]["crew"].clone()
    }

    /// ASSIGN + CLEAR: board writes `crew[seat] = slot`, the assignment rides `vehiclesById` (so it
    /// is in `small_maps_json` for the panel and a Save), and unboard removes the seat — clearing the
    /// last seat removes the `crew` key entirely, restoring the pre-board row shape.
    #[test]
    fn crew_assign_then_clear_rides_the_vehicle_row() {
        let doc = two_vehicles_three_slots();
        assert!(crew_of(&doc, "v0").is_null(), "no crew before any board");

        doc.assign_crew_seat("v0", "driver", "s0");
        assert_eq!(crew_of(&doc, "v0")["driver"], "s0", "board wrote the seat");

        doc.assign_crew_seat("v0", "gunner", "s1");
        assert_eq!(crew_of(&doc, "v0")["gunner"], "s1", "second seat coexists");
        assert_eq!(crew_of(&doc, "v0")["driver"], "s0", "first seat untouched");

        // Unboard one seat — the other survives.
        doc.clear_crew_seat("v0", "driver");
        assert!(
            crew_of(&doc, "v0")["driver"].is_null(),
            "unboard cleared the seat"
        );
        assert_eq!(crew_of(&doc, "v0")["gunner"], "s1", "other seat survives");

        // Clearing the LAST seat drops the whole `crew` key (omit idiom).
        doc.clear_crew_seat("v0", "gunner");
        assert!(
            crew_of(&doc, "v0").is_null(),
            "empty crew removes the key: {}",
            vehicles_of(&doc)["v0"]
        );
    }

    /// ONE-SEAT-PER-SLOT, fired once. A slot occupies at most one seat across ALL vehicles: boarding
    /// `s0` into `v1` while it already crews `v0` must MOVE it (evict the old seat), not duplicate it.
    /// The perturb (`v1`/driver) proves the eviction reaches a *different vehicle*, then the restore
    /// (re-board into `v0`) proves it reaches back — a rule enforced in the op, not the UI.
    #[test]
    fn crew_slot_occupies_one_seat_across_all_vehicles() {
        let doc = two_vehicles_three_slots();

        // s0 crews v0/driver; s1 crews v0/gunner.
        doc.assign_crew_seat("v0", "driver", "s0");
        doc.assign_crew_seat("v0", "gunner", "s1");

        // Re-board s0 within the SAME vehicle (driver → commander): the old seat is vacated.
        doc.assign_crew_seat("v0", "commander", "s0");
        assert_eq!(crew_of(&doc, "v0")["commander"], "s0", "moved to commander");
        assert!(
            crew_of(&doc, "v0")["driver"].is_null(),
            "s0 no longer double-seated in its own vehicle"
        );

        // ── perturb: board s0 into a DIFFERENT vehicle. The one-seat rule must evict v0/commander. ──
        doc.assign_crew_seat("v1", "driver", "s0");
        assert_eq!(crew_of(&doc, "v1")["driver"], "s0", "s0 now crews v1");
        assert!(
            crew_of(&doc, "v0")["commander"].is_null(),
            "s0 evicted from v0 — no soldier in two vehicles at once: {}",
            vehicles_of(&doc)["v0"]
        );
        // v0 still has its OTHER crew (s1/gunner) — eviction is surgical, not a wipe.
        assert_eq!(crew_of(&doc, "v0")["gunner"], "s1", "v1 board spared s1");

        // ── restore: re-board s0 back into v0. It must vacate v1 the same way. ──
        doc.assign_crew_seat("v0", "driver", "s0");
        assert_eq!(crew_of(&doc, "v0")["driver"], "s0", "s0 back in v0");
        assert!(
            crew_of(&doc, "v1").is_null(),
            "s0 evicted from v1 (its only seat) — the crew key is gone: {}",
            vehicles_of(&doc)["v1"]
        );
    }

    /// UNDO one step. A board is ONE LOCAL transaction; a single Ctrl+Z takes it off and the seat is
    /// vacant again — the crew map is undoable exactly like a cargo edit or a layer-flag flip.
    #[test]
    fn crew_board_is_one_undo_step() {
        let mut doc = two_vehicles_three_slots();
        assert_eq!(doc.undo_depth(), 0, "the INIT seed is not an undo step");

        doc.assign_crew_seat("v0", "driver", "s0");
        assert_eq!(doc.undo_depth(), 1, "one board is ONE LOCAL txn");
        assert_eq!(crew_of(&doc, "v0")["driver"], "s0");

        assert!(doc.undo(), "undo the board");
        assert_eq!(doc.undo_depth(), 0, "nothing left on the stack");
        assert!(
            crew_of(&doc, "v0").is_null(),
            "one undo vacated the seat: {}",
            vehicles_of(&doc)["v0"]
        );

        assert!(doc.redo(), "redo re-boards");
        assert_eq!(
            crew_of(&doc, "v0")["driver"],
            "s0",
            "redo restored the seat"
        );
    }

    /// Guard rails: a board against a missing vehicle, or with an empty seat/slot id, leaves the doc
    /// untouched (an empty key/value would author a crew entry no reader could act on).
    #[test]
    fn crew_assign_ignores_missing_vehicle_and_empty_ids() {
        let doc = two_vehicles_three_slots();
        doc.assign_crew_seat("nope", "driver", "s0"); // unknown vehicle
        doc.assign_crew_seat("v0", "", "s0"); // empty seat
        doc.assign_crew_seat("v0", "driver", ""); // empty slot
        assert!(
            crew_of(&doc, "v0").is_null(),
            "no crew written by any no-op"
        );
        assert_eq!(doc.undo_depth(), 0, "a no-op is not an undo step");
    }

    /// RIGHT-CREW-001 manned/unmanned intent: `set_vehicle_crewed(false)` writes `crewed: false`;
    /// the with-crew default omits the key so a manned row is unchanged; and it round-trips a Save.
    #[test]
    fn crewed_flag_is_written_only_when_unmanned() {
        let doc = two_vehicles_three_slots();
        assert!(
            vehicles_of(&doc)["v0"]["crewed"].is_null(),
            "a fresh vehicle carries no crewed key (with-crew default)"
        );

        doc.set_vehicle_crewed("v0", false);
        assert_eq!(
            vehicles_of(&doc)["v0"]["crewed"],
            false,
            "unmanned intent is stored"
        );

        doc.set_vehicle_crewed("v0", true);
        assert!(
            vehicles_of(&doc)["v0"]["crewed"].is_null(),
            "flipping back to manned removes the key, not stores true"
        );
    }

    /// T-076 — Save → reload keeps the whole crew authoring surface: seat assignments AND the
    /// unmanned intent survive a compile-and-hydrate round-trip (the panel reads what it wrote).
    #[cfg(feature = "mission")]
    #[test]
    fn crew_and_crewed_survive_save_and_reload() {
        let doc = two_vehicles_three_slots();
        doc.assign_crew_seat("v0", "driver", "s0");
        doc.assign_crew_seat("v0", "cargo2", "s1");
        doc.set_vehicle_crewed("v1", false);

        let reloaded = save_and_reload(&doc);
        assert_eq!(
            crew_of(&reloaded, "v0")["driver"],
            "s0",
            "seat assignment round-trips"
        );
        assert_eq!(crew_of(&reloaded, "v0")["cargo2"], "s1", "cargo seat too");
        assert_eq!(
            vehicles_of(&reloaded)["v1"]["crewed"],
            false,
            "unmanned intent round-trips"
        );
    }

    /* ── T-076 wave-103 BLOCKER — crew mutators must survive HYDRATE, not just fresh authoring ──
     *
     * The tests above board/unboard on a freshly-authored doc, where `crew` is a live-tracked
     * `YMap`. The mainline server-adopt path is different: `mission_hydrate::adopt_payload` →
     * `core.hydrate()` re-loads the vehicle row through `load_row`, which stores `crew` as an
     * OPAQUE `Any::Map` (not a `YMap`). The pre-fix mutators matched only `Out::YMap`, so on any
     * mission opened on a second machine unboard NO-OP'd, a board WIPED the loaded crew, and the
     * one-seat scan skipped hydrated crews. These four go through the REAL compile→hydrate→mutate
     * path (`save_and_reload`) so a regression to the `YMap`-only shape fails loudly. */

    /// Board a slot into `v0`, board a different slot into `v1`, and `save_and_reload` so both crew
    /// maps come back as opaque `Any::Map`s — the exact post-server-adopt state the mutators must
    /// handle. Returns the hydrated doc; the caller then mutates it and asserts.
    #[cfg(feature = "mission")]
    fn hydrated_with_crew() -> MissionDocCore {
        let doc = two_vehicles_three_slots();
        doc.assign_crew_seat("v0", "driver", "s0");
        doc.assign_crew_seat("v0", "gunner", "s1");
        doc.assign_crew_seat("v1", "commander", "s2");
        let reloaded = save_and_reload(&doc);
        // Precondition: the crew survived the round-trip as a *read* (the shipped test already pins
        // this) — the point of these tests is that a *mutation* after this is sound.
        assert_eq!(
            crew_of(&reloaded, "v0")["driver"],
            "s0",
            "v0 driver hydrated"
        );
        assert_eq!(
            crew_of(&reloaded, "v0")["gunner"],
            "s1",
            "v0 gunner hydrated"
        );
        reloaded
    }

    /// (1) POST-HYDRATE UNBOARD actually clears the seat. Pre-fix `clear_crew_seat` matched
    /// `Out::YMap`, missed the opaque `Any::Map`, and left the crew `{driver,gunner}` untouched.
    #[cfg(feature = "mission")]
    #[test]
    fn crew_unboard_after_hydrate_clears_the_seat() {
        let doc = hydrated_with_crew();
        doc.clear_crew_seat("v0", "driver");
        assert!(
            crew_of(&doc, "v0")["driver"].is_null(),
            "post-hydrate unboard must vacate the seat, not no-op: {}",
            vehicles_of(&doc)["v0"]
        );
        assert_eq!(
            crew_of(&doc, "v0")["gunner"],
            "s1",
            "the other loaded seat is untouched"
        );
    }

    /// (2) POST-HYDRATE BOARD preserves the existing loaded crew. Pre-fix `assign_crew_seat` hit its
    /// `_ =>` arm on the opaque map and REPLACED the whole crew with `{commander: s2'}`, silently
    /// destroying the loaded driver+gunner (a wipe that then round-tripped into the next save).
    ///
    /// **FIRE-ONCE proof:** temporarily narrow `read_crew_map` to the OLD `Out::YMap`-only shape
    /// (return `HashMap::new()` for the `Any::Map` case) and this assertion fails —
    /// `crew_of(v0) == {"commander":"s0new"}`, driver+gunner gone — reproducing the reported wipe.
    /// Restored to the whole-`Any` reader that ships. (Chosen the cheap perturbation of the fix over
    /// reinstating the whole old mutator body; same defect, same failing assertion.)
    #[cfg(feature = "mission")]
    #[test]
    fn crew_board_after_hydrate_keeps_existing_crew() {
        let doc = hydrated_with_crew();
        // A brand-new slot into a brand-new seat on the already-crewed v0.
        doc.add_slot(
            "s3", "sq", "L", 0, "Rifleman", None, None, 30.0, 40.0, 0.0, 0.0,
        );
        doc.assign_crew_seat("v0", "commander", "s3");
        assert_eq!(
            crew_of(&doc, "v0")["commander"],
            "s3",
            "the new seat was written"
        );
        assert_eq!(
            crew_of(&doc, "v0")["driver"],
            "s0",
            "the loaded driver SURVIVES the post-hydrate board (pre-fix: wiped): {}",
            vehicles_of(&doc)["v0"]
        );
        assert_eq!(
            crew_of(&doc, "v0")["gunner"],
            "s1",
            "the loaded gunner SURVIVES too"
        );
    }

    /// (3) POST-HYDRATE ONE-SEAT-PER-SLOT eviction still fires across vehicles, hydrated crew
    /// included. Pre-fix the scan skipped `Any::Map` crews, so re-boarding `s0` (which crews the
    /// hydrated `v0`) into `v1` left `s0` seated in BOTH vehicles at once.
    #[cfg(feature = "mission")]
    #[test]
    fn crew_eviction_after_hydrate_reaches_hydrated_crews() {
        let doc = hydrated_with_crew();
        // s0 currently crews the hydrated v0/driver. Board it into v1 — the one-seat rule must
        // vacate v0/driver even though v0's crew is now an opaque Any::Map.
        doc.assign_crew_seat("v1", "driver", "s0");
        assert_eq!(crew_of(&doc, "v1")["driver"], "s0", "s0 now crews v1");
        assert!(
            crew_of(&doc, "v0")["driver"].is_null(),
            "s0 evicted from the HYDRATED v0 — no soldier in two vehicles at once: {}",
            vehicles_of(&doc)["v0"]
        );
        assert_eq!(
            crew_of(&doc, "v0")["gunner"],
            "s1",
            "eviction is surgical — v0's gunner is spared"
        );
    }

    /// (4) The fixed behaviour ROUND-TRIPS: hydrate → board → serialize → re-hydrate shows the
    /// merged crew (the loaded seats plus the new one), proving the post-hydrate board is not a
    /// transient in-memory patch that a second save would drop.
    #[cfg(feature = "mission")]
    #[test]
    fn crew_board_after_hydrate_round_trips_merged() {
        let doc = hydrated_with_crew();
        doc.add_slot(
            "s3", "sq", "L", 0, "Rifleman", None, None, 30.0, 40.0, 0.0, 0.0,
        );
        doc.assign_crew_seat("v0", "commander", "s3");

        // Second save/reload — the merged crew must serialize and come back whole.
        let twice = save_and_reload(&doc);
        assert_eq!(
            crew_of(&twice, "v0")["driver"],
            "s0",
            "loaded driver present after the second round-trip"
        );
        assert_eq!(
            crew_of(&twice, "v0")["gunner"],
            "s1",
            "loaded gunner present after the second round-trip"
        );
        assert_eq!(
            crew_of(&twice, "v0")["commander"],
            "s3",
            "the post-hydrate board persisted through the second round-trip: {}",
            vehicles_of(&twice)["v0"]
        );
    }

    /// T-491 Class-R — `pick_slot_or_vehicle`: only a slot in range → slot id.
    #[test]
    fn pick_slot_or_vehicle_slot_only() {
        let cam = mix_test_cam();
        let soa = mix_test_soa(&[("s0", 6400.0, 6400.0)]);
        let vehs: Vec<(String, f64, f64)> = vec![("v0".into(), 7000.0, 7000.0)]; // far
        let hit = MissionDocCore::pick_slot_or_vehicle(&cam, &soa, &vehs, 400.0, 300.0);
        assert_eq!(hit.as_deref(), Some("s0"));
    }

    /// T-491 Class-R — `pick_slot_or_vehicle`: only a vehicle in range → vehicle id.
    #[test]
    fn pick_slot_or_vehicle_vehicle_only() {
        let cam = mix_test_cam();
        let soa = mix_test_soa(&[("s0", 7000.0, 7000.0)]); // far
        let vehs: Vec<(String, f64, f64)> = vec![("v0".into(), 6400.0, 6400.0)];
        let hit = MissionDocCore::pick_slot_or_vehicle(&cam, &soa, &vehs, 400.0, 300.0);
        assert_eq!(hit.as_deref(), Some("v0"));
    }

    /// T-491 Class-R — when both are in the pick radius, the closer world-distance wins.
    #[test]
    fn pick_slot_or_vehicle_closer_wins() {
        let cam = mix_test_cam();
        // Slot at camera target; vehicle 0.2 m east — both inside ~1 m world pick radius @ zoom 2.
        let soa = mix_test_soa(&[("s0", 6400.0, 6400.0)]);
        let vehs: Vec<(String, f64, f64)> = vec![("v0".into(), 6400.2, 6400.0)];
        // Click the exact target → slot is closer (d=0).
        assert_eq!(
            MissionDocCore::pick_slot_or_vehicle(&cam, &soa, &vehs, 400.0, 300.0).as_deref(),
            Some("s0"),
            "exact center prefers the slot at the target"
        );
        // Pixel 1 px east of center: at scale=4, that is 0.25 m east — closer to the vehicle at +0.2 m.
        assert_eq!(
            MissionDocCore::pick_slot_or_vehicle(&cam, &soa, &vehs, 401.0, 300.0).as_deref(),
            Some("v0"),
            "offset toward the vehicle must pick the vehicle"
        );
    }

    /// T-491 Class-R — marquee returns slots first, then vehicles (append order).
    #[test]
    fn marquee_ids_with_vehicles_appends_vehicles_after_slots() {
        let cam = mix_test_cam();
        let soa = mix_test_soa(&[("s0", 6400.0, 6400.0), ("s1", 6410.0, 6410.0)]);
        let vehs: Vec<(String, f64, f64)> = vec![
            ("v0".into(), 6405.0, 6405.0),
            ("v_out".into(), 7000.0, 7000.0),
        ];
        // Press at world (6390,6390); release px that unprojects past (6420,6420).
        let start_wx = 6390.0;
        let start_wy = 6390.0;
        // At zoom 2 / scale 4: world +30 m ≈ +120 px from center(400,300) → (520, …).
        // flipY:false: screen +y is south in world? pan docs say screen +y ⇒ target north.
        // Safer: use cam.unproject to find a px, or form the box via known unproject of corners.
        // End px: unproject of (520, 180) — compute by inverting: we want world ~6420,6420.
        // Center is 6400,6400 at (400,300). Δworld (+20,+20); scale=4 → Δpx (+80, -80) if +y screen = -y world.
        let end_px = 400.0 + 20.0 * cam.scale();
        let end_py = 300.0 - 20.0 * cam.scale(); // screen up → world +y under flipY:false
        let ids = MissionDocCore::marquee_ids_with_vehicles(
            &cam, &soa, &vehs, start_wx, start_wy, end_px, end_py,
        );
        assert!(
            ids.iter().any(|id| id == "s0") && ids.iter().any(|id| id == "s1"),
            "both slots in box: {ids:?}"
        );
        assert!(ids.iter().any(|id| id == "v0"), "vehicle in box: {ids:?}");
        assert!(
            !ids.iter().any(|id| id == "v_out"),
            "far vehicle excluded: {ids:?}"
        );
        let first_veh = ids
            .iter()
            .position(|id| id.starts_with('v'))
            .expect("a vehicle");
        let last_slot = ids
            .iter()
            .rposition(|id| id.starts_with('s'))
            .expect("a slot");
        assert!(
            last_slot < first_veh,
            "vehicles appended after slots: {ids:?}"
        );
    }

    /// T-574 — best-effort Rust **lexer** over a source string: line comments, (nested) block
    /// comments and string / raw-string / byte-string / char literals become spaces; newlines
    /// survive, so line structure and any `split` on real code are unaffected.
    ///
    /// `rustc` discards exactly these before it has a token, so a symbol that survives this pass is
    /// at least a *token* rather than prose. That is the whole of what it buys — it is strictly
    /// weaker than "the host calls the function", and the gap is enumerated on
    /// [`mission_editor_move_commit_names_the_atomic_mix_api`].
    fn strip_rust_lexical_noise(src: &str) -> String {
        let s: Vec<char> = src.chars().collect();
        let n = s.len();
        let mut out = String::with_capacity(src.len());
        let mut i = 0usize;
        // Blank one source char; newlines survive so nothing shifts line.
        fn blank(out: &mut String, c: char) {
            out.push(if c == '\n' { '\n' } else { ' ' });
        }
        let is_ident = |c: char| c.is_alphanumeric() || c == '_';

        while i < n {
            let c = s[i];

            // `//` … end of line (covers `///` and `//!`).
            if c == '/' && i + 1 < n && s[i + 1] == '/' {
                while i < n && s[i] != '\n' {
                    blank(&mut out, s[i]);
                    i += 1;
                }
                continue;
            }
            // `/* … */`, nesting like rustc.
            if c == '/' && i + 1 < n && s[i + 1] == '*' {
                let mut depth = 0usize;
                while i < n {
                    if s[i] == '/' && i + 1 < n && s[i + 1] == '*' {
                        depth += 1;
                        out.push_str("  ");
                        i += 2;
                    } else if s[i] == '*' && i + 1 < n && s[i + 1] == '/' {
                        depth -= 1;
                        out.push_str("  ");
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        blank(&mut out, s[i]);
                        i += 1;
                    }
                }
                continue;
            }
            // Raw string `[b]r#*"…"#*` — no escapes inside, so the hash count is the terminator.
            {
                let mut j = i;
                if s[j] == 'b' {
                    j += 1;
                }
                let mut hashes = 0usize;
                if j < n && s[j] == 'r' {
                    let mut k = j + 1;
                    while k < n && s[k] == '#' {
                        hashes += 1;
                        k += 1;
                    }
                    if k < n && s[k] == '"' && (i == 0 || !is_ident(s[i - 1])) {
                        while i <= k {
                            blank(&mut out, s[i]);
                            i += 1;
                        }
                        while i < n {
                            let closes =
                                s[i] == '"' && (1..=hashes).all(|h| i + h < n && s[i + h] == '#');
                            if closes {
                                for _ in 0..=hashes {
                                    blank(&mut out, s[i]);
                                    i += 1;
                                }
                                break;
                            }
                            blank(&mut out, s[i]);
                            i += 1;
                        }
                        continue;
                    }
                }
            }
            // Normal / byte string `[b]"…"` with backslash escapes.
            {
                let j = if s[i] == 'b' { i + 1 } else { i };
                if j < n && s[j] == '"' && (i == 0 || !is_ident(s[i - 1])) {
                    while i <= j {
                        blank(&mut out, s[i]);
                        i += 1;
                    }
                    while i < n {
                        if s[i] == '\\' {
                            blank(&mut out, s[i]);
                            if i + 1 < n {
                                blank(&mut out, s[i + 1]);
                            }
                            i += 2;
                            continue;
                        }
                        let end = s[i] == '"';
                        blank(&mut out, s[i]);
                        i += 1;
                        if end {
                            break;
                        }
                    }
                    continue;
                }
            }
            // Char / byte-char literal — but NOT a lifetime: `'x'` and `'\n'` close, `'a` does not.
            {
                let q =
                    if c == 'b' && i + 1 < n && s[i + 1] == '\'' && (i == 0 || !is_ident(s[i - 1]))
                    {
                        Some(i + 1)
                    } else if c == '\'' {
                        Some(i)
                    } else {
                        None
                    };
                if let Some(q) = q {
                    let end = if q + 1 < n && s[q + 1] == '\\' {
                        (q + 3..n).find(|&k| s[k] == '\'')
                    } else if q + 2 < n && s[q + 2] == '\'' {
                        Some(q + 2)
                    } else {
                        None // lifetime, or a stray quote — leave it alone
                    };
                    if let Some(end) = end {
                        while i <= end {
                            blank(&mut out, s[i]);
                            i += 1;
                        }
                        continue;
                    }
                }
            }
            out.push(c);
            i += 1;
        }
        out
    }

    /// T-574 — the scrubber, pinned before it is trusted.
    ///
    /// A scrubber that silently did nothing would be the exact defect this ticket exists to remove:
    /// a check reporting PASS over an input it never really examined. So every shape it claims to
    /// eat is proved gone, and live code is proved to survive.
    #[test]
    fn rust_lexical_scrubber_eats_comments_and_literals_but_not_code() {
        // The T-574 attack verbatim: the symbol survives only as prose.
        let out = strip_rust_lexical_noise(
            "// core.move_entities_and_vehicles(slot_ids, &veh_ids, dx, dy, zs);\nlet keep = 1;\n",
        );
        assert!(
            !out.contains("move_entities_and_vehicles"),
            "a line comment must not survive: {out:?}"
        );
        assert!(
            out.contains("let keep = 1;"),
            "live code must survive: {out:?}"
        );

        for (label, src) in [
            (
                "doc comment",
                "/// see move_entities_and_vehicles\nlet keep = 1;",
            ),
            ("inner doc", "//! move_entities_and_vehicles\nlet keep = 1;"),
            (
                "block comment",
                "/* move_entities_and_vehicles */ let keep = 1;",
            ),
            (
                "nested block",
                "/* outer /* move_entities_and_vehicles */ still */ let keep = 1;",
            ),
            (
                "string literal",
                "let s = \"move_entities_and_vehicles\"; let keep = 1;",
            ),
            (
                "escaped string",
                "let s = \"\\\"move_entities_and_vehicles\\\"\"; let keep = 1;",
            ),
            (
                "raw string",
                "let s = r\"move_entities_and_vehicles\"; let keep = 1;",
            ),
            (
                "hashed raw string",
                "let s = r##\"move_entities_and_vehicles \"# \"##; let keep = 1;",
            ),
            (
                "byte string",
                "let s = b\"move_entities_and_vehicles\"; let keep = 1;",
            ),
        ] {
            let out = strip_rust_lexical_noise(src);
            assert!(
                !out.contains("move_entities_and_vehicles"),
                "{label} must not survive scrubbing: {out:?}"
            );
            assert!(
                out.contains("let keep = 1;"),
                "{label}: live code lost: {out:?}"
            );
        }

        // Lifetimes are not char literals — mangling them would corrupt the code we then search.
        let life = strip_rust_lexical_noise("fn f<'a>(x: &'a str, c: char) { let q = '\\''; }");
        assert!(
            life.contains("fn f<'a>(x: &'a str, c: char)"),
            "lifetimes kept: {life:?}"
        );
        assert!(!life.contains("'\\''"), "the char literal went: {life:?}");

        // Comment delimiters *inside* a string are not comment delimiters.
        let tricky = strip_rust_lexical_noise("let s = \"// /*\"; call_me();");
        assert!(
            tricky.contains("call_me();"),
            "string-borne `//` must not eat code: {tricky:?}"
        );

        // Byte offsets and line count are preserved, so `split` on real code still lines up.
        let src = "// a\nlet keep = 1;\n";
        let out = strip_rust_lexical_noise(src);
        assert_eq!(
            out.chars().count(),
            src.chars().count(),
            "char count preserved"
        );
        assert_eq!(
            out.lines().count(),
            src.lines().count(),
            "newlines preserved: {out:?}"
        );
    }

    /// T-491 / T-574 — the host **names** the Class-R SoT rather than a forked copy.
    ///
    /// **Read the name of this test literally.** It is a source check on two files in another
    /// crate, run over lexically scrubbed text ([`strip_rust_lexical_noise`]) so a comment, doc
    /// comment or string literal can no longer satisfy it — that was the T-574 defect, and it is
    /// fixed. What it still cannot decide is whether the token it found is *live code*; a grep
    /// cannot, and `map-engine-core` cannot link the frontend crate to find out (the dependency
    /// runs the other way). So this pin admits, and does **not** detect:
    ///
    ///   * a call site under `#[cfg(…)]` that is never enabled, or inside `if false` / after an
    ///     early `return` — present as a token, dead at runtime;
    ///   * a same-named method on some *other* receiver shadowing `MissionDocCore`'s;
    ///   * the whole Move arm being unreachable because an earlier branch returns.
    ///
    /// The real proof that the mixed drag is one undo step lives where it can be executed:
    /// [`super::MissionDocCore::move_entities_and_vehicles`] is pinned behaviourally by
    /// `move_entities_and_vehicles_moves_every_slot_and_vehicle_in_one_txn` (and by its doctest,
    /// which runs outside `--cfg test`). The residue this test covers — that the *host* is wired to
    /// that SoT — has no runtime signature from a native crate; closing it needs a behavioural test
    /// in `website-frontend`, which owns the pointer-gesture code.
    #[test]
    fn mission_editor_move_commit_names_the_atomic_mix_api() {
        let select = strip_rust_lexical_noise(include_str!(
            "../../../../apps/website/frontend/src/select_tool.rs"
        ));
        assert!(
            select.contains("MissionDocCore::pick_slot_or_vehicle("),
            "select_tool.rs has no `MissionDocCore::pick_slot_or_vehicle(` call token outside \
             comments/strings — the mixed pick was forked or deleted"
        );
        assert!(
            select.contains("MissionDocCore::marquee_ids_with_vehicles("),
            "select_tool.rs has no `MissionDocCore::marquee_ids_with_vehicles(` call token outside \
             comments/strings — the mixed marquee was forked or deleted"
        );

        let editor = strip_rust_lexical_noise(include_str!(
            "../../../../apps/website/frontend/src/mission_editor.rs"
        ));
        // Select the commit arm BY ITS ATOMIC CALL rather than by the destructure's exact
        // spelling: T-647 (wave 106) added `cam` to the pattern and rustfmt re-wrapped it, which
        // silently unmatched the old one-line `LG::Move { ids, dx, dy, .. }` splitter. Field
        // lists will keep changing; the invariant is that exactly ONE LG::Move arm commits, it
        // does so through the atomic mixed API, and no split-txn call rides in that arm.
        let move_arms: Vec<&str> = editor
            .split("LG::Move")
            .skip(1)
            .map(|s| s.split("LG::").next().unwrap_or(s))
            .filter(|arm| arm.contains(".move_entities_and_vehicles("))
            .collect();
        assert!(
            move_arms.len() == 1,
            "expected exactly one LG::Move arm committing via `.move_entities_and_vehicles(` \
             (found {}) — the atomic mixed-move commit was forked, duplicated or deleted",
            move_arms.len()
        );
        let move_arm = move_arms[0];
        assert!(
            move_arm.contains(".move_entities_and_vehicles("),
            "the Move arm has no `.move_entities_and_vehicles(` call token outside comments/\
             strings — T-574's defect was that a bare `contains` here accepted a comment"
        );
        // The T-425 split path must not come back onto the Move commit (two txns → two Ctrl+Z).
        assert!(
            !move_arm.contains("core.move_entities("),
            "Move arm calls move_entities alone (two-txn defect)"
        );
        assert!(
            !move_arm.contains("editor_ops::move_vehicles"),
            "Move arm calls editor_ops::move_vehicles (second txn)"
        );
    }

    /// Camera centred on Everon mid-map @ zoom 2 (scale = 4 px/m). Centre px (400,300) → (6400,6400).
    fn mix_test_cam() -> crate::camera::OrthoCamera {
        let mut cam = crate::camera::OrthoCamera::new(800.0, 600.0, 6400.0, 6400.0, 2.0);
        cam.set_bounds(0.0, 0.0, 12_800.0, 12_800.0);
        cam
    }

    fn mix_test_soa(rows: &[(&str, f32, f32)]) -> SlotSoa {
        let mut soa = SlotSoa::default();
        for &(id, x, y) in rows {
            soa.ids.push(id.to_string());
            soa.xs.push(x);
            soa.ys.push(y);
            soa.xy.push(x);
            soa.xy.push(y);
        }
        soa
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

    // ── T-222 — client id is what makes two peers two peers ─────────────────────────────────────

    /// **The test that matters.** Two peers edit the SAME document concurrently — same empty base,
    /// neither having seen the other's write — then exchange updates. Both edits must survive on
    /// both sides, and the two documents must converge.
    ///
    /// This is the property `const CLIENT_ID: u64 = 1` destroyed. With both peers on one id, each
    /// writes its slot into blocks `(client 1, clock 0..n)`; when the peer's update arrives, `yrs`
    /// compares it against the local state vector, sees clock range `0..n` for client `1` as
    /// **already integrated**, and drops it. `apply_update` returns `Ok`. The slot is simply gone.
    /// That is the T-222 corruption, and it is silent — which is why this asserts the *contents*
    /// after the merge and not merely that the ids differ. (An assertion like "the id is not 1"
    /// would pass on a constant `2`, and every peer would still collide.)
    #[test]
    fn two_peers_with_distinct_ids_merge_concurrent_edits() {
        let a = MissionDocCore::with_client_id(0x00A1_A1A1);
        let b = MissionDocCore::with_client_id(0x00B2_B2B2);
        assert_ne!(a.client_id(), b.client_id(), "two peers, two identities");

        // Concurrent: both author from the same empty base, neither has seen the other.
        a.add_slot(
            "from-a", "sq1", "lyr", 0, "Rifleman", None, None, 10.0, 20.0, 0.0, 90.0,
        );
        b.add_slot(
            "from-b", "sq1", "lyr", 0, "Medic", None, None, 30.0, 40.0, 0.0, 180.0,
        );

        let ua = a.encode_state();
        let ub = b.encode_state();

        // Exchange. Each side integrates the other's concurrent write.
        a.apply_update(&ub).expect("a integrates b");
        b.apply_update(&ua).expect("b integrates a");

        // Both edits survive on BOTH sides — the whole point of a distinguishable client id.
        let sa = a.materialize();
        let sb = b.materialize();
        assert_eq!(
            ids_sorted(&sa),
            vec!["from-a".to_string(), "from-b".to_string()],
            "peer A must hold both concurrent slots"
        );
        assert_eq!(
            ids_sorted(&sb),
            vec!["from-a".to_string(), "from-b".to_string()],
            "peer B must hold both concurrent slots"
        );

        // …and the payloads are the authored ones, not a spliced hybrid of two histories.
        for soa in [&sa, &sb] {
            let ra = row_of(soa, "from-a");
            assert_eq!(soa.xs[ra], 10.0_f32);
            assert_eq!(soa.rotations[ra], 90.0_f32);
            assert_eq!(soa.roles[soa.role_idx[ra] as usize], "Rifleman");
            let rb = row_of(soa, "from-b");
            assert_eq!(soa.xs[rb], 30.0_f32);
            assert_eq!(soa.rotations[rb], 180.0_f32);
            assert_eq!(soa.roles[soa.role_idx[rb] as usize], "Medic");
        }

        // Convergence: the two peers agree. Re-exchanging is a no-op (idempotent).
        a.apply_update(&b.encode_state()).expect("a re-integrates");
        b.apply_update(&a.encode_state()).expect("b re-integrates");
        assert_eq!(ids_sorted(&a.materialize()), ids_sorted(&b.materialize()));
    }

    /// The collision that used to be silent is now loud — when it is detectable at all. Two peers
    /// authoring under the SAME id (the exact T-222 defect, reproduced deliberately) cannot be
    /// merged, so `apply_update` refuses instead of splicing them together.
    #[test]
    fn colliding_client_ids_are_rejected_not_merged() {
        let a = MissionDocCore::with_client_id(7);
        let b = MissionDocCore::with_client_id(7);
        a.add_slot(
            "from-a", "sq1", "lyr", 0, "Rifleman", None, None, 1.0, 2.0, 0.0, 0.0,
        );
        // B runs ahead of A on the shared id — the detectable shape of the collision.
        for (i, id) in ["from-b1", "from-b2", "from-b3"].iter().enumerate() {
            b.add_slot(
                id,
                "sq1",
                "lyr",
                u32::try_from(i).unwrap(),
                "Medic",
                None,
                None,
                3.0,
                4.0,
                0.0,
                0.0,
            );
        }

        let err = a
            .apply_update(&b.encode_state())
            .expect_err("a collision must not merge");
        assert!(err.contains("client id collision"), "{err}");
        // A refused merge leaves the document untouched — no half-integrated history.
        assert_eq!(ids_sorted(&a.materialize()), vec!["from-a".to_string()]);
    }

    /// **Pins the documented limit of the guard, so nobody mistakes it for a proof.** A colliding
    /// writer that is *behind* us on the shared id is undetectable: its blocks fall inside a clock
    /// range we have already issued, `yrs` discards them as already-seen, and a v1 update carries
    /// nothing that distinguishes that from a legitimate echo. The data is lost silently.
    ///
    /// This is precisely the T-222 corruption, and the only real defence against it is that
    /// [`MissionDocCore::new`] gives every peer its own id — which
    /// `two_default_constructed_peers_merge_concurrent_edits` proves it does.
    #[test]
    fn a_colliding_writer_behind_us_is_undetectable_and_silently_loses_its_edits() {
        let a = MissionDocCore::with_client_id(9);
        let b = MissionDocCore::with_client_id(9);
        for (i, id) in ["a1", "a2", "a3"].iter().enumerate() {
            a.add_slot(
                id,
                "sq1",
                "lyr",
                u32::try_from(i).unwrap(),
                "Rifleman",
                None,
                None,
                1.0,
                2.0,
                0.0,
                0.0,
            );
        }
        b.add_slot(
            "b1", "sq1", "lyr", 0, "Medic", None, None, 3.0, 4.0, 0.0, 0.0,
        );

        // Reports success…
        a.apply_update(&b.encode_state())
            .expect("undetectable: reads as an echo");
        // …and B's slot is simply gone. Documenting the hole, not condoning it.
        assert_eq!(
            ids_sorted(&a.materialize()),
            vec!["a1".to_string(), "a2".to_string(), "a3".to_string()],
            "b1 is lost — the silent corruption a shared client id causes"
        );
    }

    /// Rehydration takes a FRESH client id and replays the persisted blob — it does not resurrect
    /// the id the blob was authored under. The restore path builds an empty core, so the guard's
    /// "I have already authored" precondition is false and the blob integrates cleanly even when
    /// the ids happen to coincide; the restored doc then authors under its own identity.
    #[test]
    fn rehydration_replays_a_blob_into_a_fresh_peer_identity() {
        let session1 = MissionDocCore::with_client_id(0x00C3_C3C3);
        session1.add_slot(
            "persisted",
            "sq1",
            "lyr",
            0,
            "Rifleman",
            None,
            None,
            5.0,
            6.0,
            0.0,
            45.0,
        );
        let blob = session1.encode_state();

        // Reload: a NEW id, not the persisted one.
        let session2 = MissionDocCore::with_client_id(0x00D4_D4D4);
        assert_ne!(session2.client_id(), session1.client_id());
        session2.apply_update(&blob).expect("restore ok");
        assert_eq!(
            ids_sorted(&session2.materialize()),
            vec!["persisted".to_string()]
        );

        // The restored session is a first-class peer: its own edits merge with a third writer that
        // also replayed the same blob — i.e. two tabs restoring one blob do NOT collide.
        let other_tab = MissionDocCore::with_client_id(0x00E5_E5E5);
        other_tab.apply_update(&blob).expect("restore ok");
        session2.add_slot(
            "tab-two", "sq1", "lyr", 1, "Medic", None, None, 7.0, 8.0, 0.0, 0.0,
        );
        other_tab.add_slot(
            "tab-three",
            "sq1",
            "lyr",
            2,
            "Engineer",
            None,
            None,
            9.0,
            10.0,
            0.0,
            0.0,
        );
        other_tab
            .apply_update(&session2.encode_state())
            .expect("tabs merge");
        assert_eq!(
            ids_sorted(&other_tab.materialize()),
            vec![
                "persisted".to_string(),
                "tab-three".to_string(),
                "tab-two".to_string()
            ]
        );
    }

    /// The empty-peer replay the codebase actually relies on (`mission_doc::roundtrip_ok`,
    /// `yrs_persist`'s corrupt-blob probe, the boot swap) keeps working even in the worst case: a
    /// doc that has authored nothing adopts a blob authored by *its own* id rather than tripping
    /// the collision guard. This is the case that would break if the guard were merely "does the
    /// update mention my client id".
    #[test]
    fn fresh_peer_replays_a_same_id_blob_without_tripping_the_guard() {
        let a = MissionDocCore::with_client_id(42);
        a.add_slot(
            "s1", "sq1", "lyr", 0, "Rifleman", None, None, 1.0, 2.0, 3.0, 4.0,
        );
        let probe = MissionDocCore::with_client_id(42); // same id, but has authored nothing
        assert_eq!(probe.client_id(), a.client_id());
        probe
            .apply_update(&a.encode_state())
            .expect("probe replays");
        assert_eq!(ids_sorted(&probe.materialize()), vec!["s1".to_string()]);
    }

    /// `new()` — the constructor the editor actually calls — hands out a *different* identity every
    /// time. Before T-222 it returned the constant `1` for every peer on every machine.
    ///
    /// Distinctness alone is a weak claim (a counter would satisfy it), so this also pins the two
    /// properties that make the id usable as a CRDT identity: it is inside the 53-bit Yjs-compatible
    /// range, and it is not the old hardcoded constant. The *merge* behaviour is proved separately
    /// by `two_peers_with_distinct_ids_merge_concurrent_edits`.
    #[test]
    fn new_mints_a_distinct_client_id_per_document() {
        let ids: Vec<u64> = (0..16).map(|_| MissionDocCore::new().client_id()).collect();
        let unique: HashSet<u64> = ids.iter().copied().collect();
        assert_eq!(
            unique.len(),
            ids.len(),
            "16 fresh docs must have 16 distinct client ids, got {ids:?}"
        );
        for id in ids {
            assert_ne!(id, 1, "the T-222 hardcode must not come back");
            assert_eq!(
                id >> CLIENT_ID_BITS,
                0,
                "{id} is not a 53-bit Yjs client id"
            );
        }
    }

    /// Two `new()` peers — no hand-picked ids anywhere — still merge concurrent edits. This is
    /// `two_peers_with_distinct_ids_merge_concurrent_edits` run against the *production* path, so a
    /// regression that only re-hardcodes `new()` cannot hide behind the explicit-id test.
    #[test]
    fn two_default_constructed_peers_merge_concurrent_edits() {
        let a = MissionDocCore::new();
        let b = MissionDocCore::new();
        a.add_slot(
            "from-a", "sq1", "lyr", 0, "Rifleman", None, None, 10.0, 20.0, 0.0, 90.0,
        );
        b.add_slot(
            "from-b", "sq1", "lyr", 0, "Medic", None, None, 30.0, 40.0, 0.0, 180.0,
        );
        let (ua, ub) = (a.encode_state(), b.encode_state());
        a.apply_update(&ub).expect("a integrates b");
        b.apply_update(&ua).expect("b integrates a");
        let want = vec!["from-a".to_string(), "from-b".to_string()];
        assert_eq!(ids_sorted(&a.materialize()), want);
        assert_eq!(ids_sorted(&b.materialize()), want);
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

    /// T-766 — deliberate clear must drop `meta.briefing`; blank `apply_row_meta` must not.
    ///
    /// RED if `clear_meta_briefing` is a no-op, or if blank `Some("")` starts clearing (that would
    /// break hydrate). Behavioural pin — not a source grep.
    #[test]
    fn t766_clear_meta_briefing_drops_key_blank_apply_does_not() {
        let doc = MissionDocCore::new();
        doc.apply_row_meta(
            "Op",
            "everon",
            None,
            None,
            Some("Hold the bridge.\nWait for extract.".into()),
        );
        assert_eq!(
            small_maps(&doc)["meta"]["briefing"],
            "Hold the bridge.\nWait for extract.",
            "precondition: row briefing must land"
        );

        // Hydrate-shaped blank must NOT wipe — the guard T-766 must not weaken.
        doc.apply_row_meta("Op", "everon", None, None, Some("".into()));
        assert_eq!(
            small_maps(&doc)["meta"]["briefing"],
            "Hold the bridge.\nWait for extract.",
            "blank apply_row_meta must stay 'not supplied', not a clear"
        );
        doc.apply_row_meta("Op", "everon", None, None, Some("   \n\t  ".into()));
        assert_eq!(
            small_maps(&doc)["meta"]["briefing"],
            "Hold the bridge.\nWait for extract.",
            "whitespace-only apply_row_meta must stay 'not supplied'"
        );

        doc.clear_meta_briefing();
        assert!(
            small_maps(&doc)["meta"].get("briefing").is_none(),
            "clear_meta_briefing must remove meta.briefing so compile_export emits \"\""
        );

        // Idempotent: clearing an absent key must not invent one.
        doc.clear_meta_briefing();
        assert!(
            small_maps(&doc)["meta"].get("briefing").is_none(),
            "second clear must stay absent"
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

    // ── T-069 — the READ half of the briefing-marker surface ─────────────────────────────────────

    /// `briefing_marker_rows_json` parsed back to a `Vec`.
    fn marker_rows(doc: &MissionDocCore) -> Vec<serde_json::Value> {
        serde_json::from_str::<serde_json::Value>(&doc.briefing_marker_rows_json())
            .expect("the reader emits JSON")
            .as_array()
            .cloned()
            .expect("the reader emits an array")
    }

    /// **T-069 — the reader addresses every marker by the pair its writers take.**
    ///
    /// The two T-345 mutators are keyed `(faction_id, marker_id)`; a dock row must therefore carry
    /// BOTH or it cannot move, re-caption or delete the marker it is showing. This walks a
    /// two-faction document and asserts each row round-trips straight back into
    /// `remove_faction_briefing_marker` — which is the only proof that matters, since a reader whose
    /// ids do not address anything is indistinguishable from a broken one until a delete silently
    /// no-ops in front of the user.
    #[test]
    fn every_listed_marker_is_addressable_by_the_pair_the_mutators_take() {
        let doc = briefing_fixture();
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 100.5, 200.5, "objective", "OBJ");
        doc.set_faction_briefing_marker("faction-OPFOR", "mk-2", 300.5, 400.5, "ambush", "AMB");

        let rows = marker_rows(&doc);
        assert_eq!(rows.len(), 2, "both sides list: {rows:?}");
        // Sorted by factionId, so BLUFOR precedes OPFOR whatever order the hash map walked them in.
        assert_eq!(rows[0]["factionId"], serde_json::json!("faction-BLUFOR"));
        assert_eq!(rows[1]["factionId"], serde_json::json!("faction-OPFOR"));
        assert_eq!(marker_num(&rows[0], "x"), 100.5);
        assert_eq!(marker_num(&rows[0], "z"), 200.5);
        assert_eq!(rows[0]["icon"], serde_json::json!("objective"));
        assert_eq!(rows[0]["label"], serde_json::json!("OBJ"));

        // The pair the reader hands out must be the pair the writers accept.
        for r in &rows {
            let f = r["factionId"].as_str().expect("factionId");
            let id = r["id"].as_str().expect("id");
            doc.remove_faction_briefing_marker(f, id);
        }
        assert!(
            marker_rows(&doc).is_empty(),
            "every listed row deleted through its own (factionId, id)"
        );
    }

    /// **T-069 — the list order is deterministic, and each faction keeps its ARRAY order.**
    ///
    /// `MapRef::iter` is a hash-map walk, so an unsorted reader would shuffle the faction groups
    /// between renders and make the dock rows dance under the cursor for a document that never
    /// changed. Within one faction the order is the ARRAY's, which is load-bearing rather than
    /// cosmetic: `derive_briefings` pushes into the parallel arrays `TBD_MarkerService.Build` sends
    /// in exactly that order, and `set_faction_briefing_marker` replaces IN PLACE so a drag cannot
    /// reorder them — a reader that sorted rows by id would hide that guarantee from the one surface
    /// that displays it.
    #[test]
    fn marker_rows_are_stable_across_calls_and_keep_array_order() {
        let doc = briefing_fixture();
        // Authored deliberately NOT in id order, so an id sort would be visible.
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-z", 1.0, 1.0, "rally", "Z");
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-a", 2.0, 2.0, "rally", "A");
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-m", 3.0, 3.0, "rally", "M");

        let ids: Vec<String> = marker_rows(&doc)
            .iter()
            .map(|r| r["id"].as_str().expect("id").to_string())
            .collect();
        assert_eq!(
            ids,
            vec!["mk-z".to_string(), "mk-a".to_string(), "mk-m".to_string()],
            "array order, not id order"
        );

        // Byte-identical across repeated reads of an unchanged document.
        let first = doc.briefing_marker_rows_json();
        for _ in 0..8 {
            assert_eq!(
                doc.briefing_marker_rows_json(),
                first,
                "the reader must not shuffle an unchanged document"
            );
        }

        // A move (upsert on the same id) keeps the row where it was — the in-place replace.
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-z", 9.0, 9.0, "rally", "Z");
        let after: Vec<String> = marker_rows(&doc)
            .iter()
            .map(|r| r["id"].as_str().expect("id").to_string())
            .collect();
        assert_eq!(after, ids, "a move must not reorder the list");
    }

    /// **T-069 — a marker in the `markers` ROOT map reaches nothing, and the reader does not offer
    /// it.**
    ///
    /// T-069's own registry summary says free marker placement needs generic add/move/remove on
    /// `markersById`. That premise is DEAD, and this is the check that says so rather than asserting
    /// it in prose: `mission.schema.json` declares no top-level `markers` property at all, and
    /// `flatten_to_mod_document` deserialises `EditorPayload { editor: { factions, squads, slots } }`
    /// — which declares no root key whatsoever. So a row authored into the root map hydrates, emits
    /// back through `small_maps_json` as `markersById`, and is then dropped on the floor by the
    /// compiler. Authoring there would have produced markers no mod subsystem can read.
    ///
    /// The briefing marker on the SAME document is compiled in the same breath, which is what makes
    /// this a contrast and not just an empty-output assertion.
    #[cfg(feature = "mission")]
    #[test]
    fn a_marker_in_the_root_map_never_reaches_the_compiled_document() {
        let payload = serde_json::json!({
            "schemaVersion": 1,
            "map": { "terrain": "everon" },
            // The ROOT map — `hydrate` loads it, `small_maps_json` emits it as `markersById`.
            "markers": [{ "id": "root-1", "x": 11.0, "z": 22.0, "icon": "dot", "label": "ROOT" }],
            "editor": {
                "factions": [{ "id": "faction-BLUFOR", "key": "BLUFOR", "name": "US Army",
                               "squadIds": ["sq-a"] }],
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

        // It really is in the document — this is not an empty-doc tautology.
        assert!(
            small_maps(&doc)["markersById"]
                .as_object()
                .is_some_and(|m| !m.is_empty()),
            "the root map hydrated: {:?}",
            small_maps(&doc)["markersById"]
        );
        // …and the reader deliberately does not surface it: it reads briefings only.
        assert!(
            marker_rows(&doc).is_empty(),
            "the root map is not an authoring surface, so it is not listed"
        );

        // The schema-legal placement, on the same document.
        doc.set_faction_briefing_marker("faction-BLUFOR", "mk-1", 33.0, 44.0, "objective", "OBJ");
        assert_eq!(marker_rows(&doc).len(), 1);

        let compiled_payload = crate::mission::compile::compile_payload(
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
            &serde_json::to_vec(&compiled_payload).expect("payload serialises"),
        )
        .expect("the fixture has a slot, so the compile must succeed");
        let doc_json = serde_json::to_value(&compiled).expect("mod document serialises");

        // The briefing marker compiled…
        let rows = doc_json["briefings"]["blufor"]["markers"]
            .as_array()
            .expect("blufor briefing carries markers");
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["label"], serde_json::json!("OBJ"));

        // …and the root-map marker reached nothing. No top-level key, and nothing named ROOT
        // anywhere in the compiled bytes.
        assert!(
            doc_json.get("markers").is_none(),
            "the compiled document has no top-level `markers`: {doc_json:?}"
        );
        assert!(
            !serde_json::to_string(&doc_json)
                .expect("serialises")
                .contains("ROOT"),
            "nothing from the root map may appear in the compiled document"
        );
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

    // ── T-211 — authored zones: round trip, shape fidelity, wire reach ──────────────────────────

    /// A doc carrying one slot (so `compile_payload` has a mission to compile) plus one polygon
    /// and one circle zone, authored through the mutators exactly as a draw tool would.
    ///
    /// **Coordinates are deliberately NON-INTEGRAL.** Yjs encodes integer-valued numbers as
    /// `Any::BigInt` and non-integers as `Any::Number`, so an all-integer fixture would round-trip
    /// identically under a bug that coerced every coordinate to an integer — it would pass over a
    /// polygon silently snapped to a 1 m grid. `-4210.75` cannot survive that, so the fixture can
    /// tell "the geometry came back" from "a number-shaped thing came back". Same reason T-219's
    /// fixture uses `42.5`.
    #[cfg(feature = "mission")]
    fn zones_fixture() -> MissionDocCore {
        let doc = MissionDocCore::new();
        // A COMPLETE editor graph, not just a loose slot: `flatten_to_mod_document` answers
        // `CompileError::NoSlots` on a faction/squad-less document, so a bare slot would fail the
        // end-to-end test for a reason with nothing to do with zones.
        doc.add_faction("f1", "BLUFOR", "US");
        doc.add_squad("sq1", "f1", "Alpha", Some("Alpha".to_string()));
        doc.add_slot(
            "s1", "sq1", "lyr", 0, "Rifleman", None, None, 100.5, 200.5, 0.0, 0.0,
        );
        doc.add_polygon_zone(
            "z_ao",
            "boundary",
            &[
                1000.25, -4210.75, 1600.5, -4210.75, 1600.5, -3800.125, 1000.25, -3800.125,
            ],
        );
        doc.set_zone_label("z_ao", Some("Area of Operations"));
        doc.set_zone_rules(
            "z_ao",
            Some(r#"{"graceSeconds":45.5,"penalty":"kill","warnEverySeconds":7.25}"#),
        );
        doc.add_circle_zone("z_obj", "objective_capture", 1234.5, -3990.25, 175.75);
        doc.set_zone_faction("z_obj", Some("blufor"));
        doc.set_zone_rules("z_obj", Some(r#"{"captureSeconds":180.5}"#));
        doc
    }

    /// Pull the zones array off a compiled wire payload, whichever route put it there. Once
    /// `compile_payload` authors `zones` itself this reads the authored key instead of the
    /// promoted one, with no test change — which is the point.
    #[cfg(feature = "mission")]
    fn wire_zones(payload: &serde_json::Value) -> Vec<serde_json::Value> {
        payload
            .get("zones")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    }

    /// **THE ROUND TRIP — author → persist → reload → identical, geometry included.**
    ///
    /// This is the test the ticket turns on, and it asserts on the WHOLE zone object, not on the
    /// presence of a `zones` key. A test that checked `payload["zones"].is_array()` — or even that
    /// the ids survived — would pass green over a zone whose `shape` was dropped, whose polygon was
    /// truncated to its first vertex, or whose `rules` were replaced with `{}`. Comparing the full
    /// row by value is what makes the geometry non-optional.
    ///
    /// The cycle is run TWICE (`compile → hydrate → compile`) because one pass cannot distinguish
    /// "the doc stored it" from "the doc round-tripped it": a value merely echoed out of the parked
    /// side-channel survives one compile and dies on the second, which is precisely the T-219 class.
    #[cfg(feature = "mission")]
    #[test]
    fn authored_zones_survive_compile_hydrate_compile_whole() {
        let doc = zones_fixture();

        let compiled = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let first = wire_zones(&compiled);
        assert_eq!(
            first.len(),
            2,
            "both authored zones must reach the wire payload: {compiled}"
        );

        // Full-value expectation — every schema key, geometry included, spelled out.
        let expect_ao = serde_json::json!({
            "id": "z_ao",
            "type": "boundary",
            "label": "Area of Operations",
            "shape": { "polygon": [
                [1000.25, -4210.75],
                [1600.5,  -4210.75],
                [1600.5,  -3800.125],
                [1000.25, -3800.125]
            ]},
            "rules": { "graceSeconds": 45.5, "penalty": "kill", "warnEverySeconds": 7.25 }
        });
        let expect_obj = serde_json::json!({
            "id": "z_obj",
            "type": "objective_capture",
            "faction": "blufor",
            "shape": { "circle": { "x": 1234.5, "z": -3990.25, "r": 175.75 } },
            "rules": { "captureSeconds": 180.5 }
        });

        let by_id = |rows: &[serde_json::Value], id: &str| -> serde_json::Value {
            rows.iter()
                .find(|r| r["id"] == id)
                .unwrap_or_else(|| panic!("zone {id} missing from {rows:?}"))
                .clone()
        };

        assert_eq!(
            by_id(&first, "z_ao"),
            expect_ao,
            "compile #1 dropped part of z_ao"
        );
        assert_eq!(
            by_id(&first, "z_obj"),
            expect_obj,
            "compile #1 dropped part of z_obj"
        );

        // Reload from the persisted payload, exactly as the editor does.
        let reloaded = save_and_reload(&doc);
        assert_eq!(reloaded.zone_count(), 2, "hydrate must restore both zones");

        let recompiled = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        let second = wire_zones(&recompiled);
        assert_eq!(
            by_id(&second, "z_ao"),
            expect_ao,
            "z_ao did not survive save→reload→save WHOLE"
        );
        assert_eq!(
            by_id(&second, "z_obj"),
            expect_obj,
            "z_obj did not survive save→reload→save WHOLE"
        );

        // And the side-channel name itself never becomes a wire key (T-432 still holds).
        assert!(
            recompiled.get("payloadExtras").is_none(),
            "payloadExtras must not reach the wire: {recompiled}"
        );
    }

    /// The zones the round trip carries must be a document the DECLARED schema accepts, so the
    /// fixture is pinned against `$defs/zone` key by key. This is a vocabulary check, not a second
    /// validator: it asserts the doc layer emits nothing `additionalProperties: false` would
    /// reject, and that `rules` only ever carries keys T-241 declared.
    #[cfg(feature = "mission")]
    #[test]
    fn authored_zone_rows_use_only_declared_schema_keys() {
        // `mission.schema.json#/$defs/zone` — the six declared properties.
        const ZONE_KEYS: &[&str] = &["id", "type", "shape", "label", "faction", "rules"];
        // `$defs/zone.type` — the six declared enum values.
        const ZONE_TYPES: &[&str] = &[
            "spawn",
            "objective_capture",
            "objective_destroy",
            "objective_hold_until",
            "boundary",
            "base_protection",
        ];
        // `$defs/zoneRules` — T-241's closed 16-key vocabulary, verbatim.
        const RULE_KEYS: &[&str] = &[
            "graceSeconds",
            "warnEverySeconds",
            "penalty",
            "captureSeconds",
            "neutralizeSeconds",
            "contestable",
            "onEmpty",
            "decayRate",
            "holdSeconds",
            "pauseOnEnemy",
            "resetOnEnemy",
            "requireHolderPresent",
            "targetAlias",
            "targetCount",
            "points",
            "announceEverySeconds",
        ];

        let doc = zones_fixture();
        let compiled = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let rows = wire_zones(&compiled);
        assert!(!rows.is_empty(), "fixture authored no zones");

        for row in &rows {
            let obj = row.as_object().expect("zone row is an object");
            for k in obj.keys() {
                assert!(
                    ZONE_KEYS.contains(&k.as_str()),
                    "undeclared zone key `{k}` — `$defs/zone` is additionalProperties:false, so \
                     this row cannot validate: {row}"
                );
            }
            // The three `required` keys.
            for k in ["id", "type", "shape"] {
                assert!(obj.contains_key(k), "zone missing required `{k}`: {row}");
            }
            assert!(
                ZONE_TYPES.contains(&row["type"].as_str().unwrap_or_default()),
                "zone type outside the declared enum: {row}"
            );

            // `$defs/shape` is a oneOf — exactly one branch, never both, never neither.
            let shape = row["shape"].as_object().expect("shape object");
            let has_circle = shape.contains_key("circle");
            let has_polygon = shape.contains_key("polygon");
            assert!(
                has_circle ^ has_polygon,
                "`$defs/shape` is oneOf(circle|polygon) — this row satisfies {} branches: {row}",
                usize::from(has_circle) + usize::from(has_polygon)
            );
            assert_eq!(shape.len(), 1, "shape carries an undeclared sibling: {row}");

            if has_circle {
                let c = shape["circle"].as_object().expect("circle object");
                let mut ks: Vec<&str> = c.keys().map(String::as_str).collect();
                ks.sort_unstable();
                assert_eq!(ks, ["r", "x", "z"], "circle is closed to x/z/r: {row}");
                assert!(
                    c["r"].as_f64().unwrap_or_default() > 0.0,
                    "circle.r is exclusiveMinimum 0: {row}"
                );
            } else {
                let ring = shape["polygon"].as_array().expect("polygon array");
                assert!(ring.len() >= 3, "polygon minItems is 3: {row}");
                for p in ring {
                    let pt = p.as_array().expect("polygon vertex is an array");
                    assert_eq!(pt.len(), 2, "vertex is min/maxItems 2: {row}");
                    assert!(
                        pt.iter().all(serde_json::Value::is_number),
                        "vertex is numeric: {row}"
                    );
                }
            }

            if let Some(rules) = row.get("rules") {
                let r = rules.as_object().expect("rules object");
                assert!(
                    !r.is_empty(),
                    "an empty `rules` must be omitted, not written: {row}"
                );
                for k in r.keys() {
                    assert!(
                        RULE_KEYS.contains(&k.as_str()),
                        "`{k}` is outside T-241's closed zoneRules vocabulary — \
                         additionalProperties:false would reject this document: {row}"
                    );
                }
            }
        }
    }

    /// The two emit routes `small_maps_json` writes (canonical `zonesById`, transitional
    /// `payloadExtras.zones`) must describe the same zones in the same ORDER, or the compile.rs
    /// companion change would silently reorder every mission's zones the day it lands.
    #[cfg(feature = "mission")]
    #[test]
    fn zones_by_id_and_extras_projection_agree_on_order() {
        // Hydrate establishes an authored order that is NOT the map's own iteration order.
        let incoming = serde_json::json!({
            "schemaVersion": 1,
            "map": { "terrain": "everon" },
            "environment": {},
            "zones": [
                { "id": "z_c", "type": "boundary",  "shape": { "circle": { "x": 3.5, "z": 4.5, "r": 5.5 } } },
                { "id": "z_a", "type": "spawn",     "shape": { "circle": { "x": 1.5, "z": 2.5, "r": 6.5 } } },
                { "id": "z_b", "type": "base_protection", "shape": { "circle": { "x": 7.5, "z": 8.5, "r": 9.5 } } }
            ],
            "editor": { "factions": [], "squads": [], "slots": [], "editorLayers": [] }
        });
        let doc = MissionDocCore::new();
        doc.hydrate(&incoming.to_string(), "lyr");

        let small = small_maps(&doc);
        let projected: Vec<&str> = small["payloadExtras"]["zones"]
            .as_array()
            .expect("projected zones array")
            .iter()
            .map(|z| z["id"].as_str().expect("id"))
            .collect();
        assert_eq!(
            projected,
            ["z_c", "z_a", "z_b"],
            "the projection must replay hydrate's authored order, not map order"
        );

        // Every projected row is the same object the canonical by-id map holds.
        for id in &projected {
            assert_eq!(
                small["zonesById"][id],
                *small["payloadExtras"]["zones"]
                    .as_array()
                    .expect("arr")
                    .iter()
                    .find(|z| z["id"] == *id)
                    .expect("row"),
                "zonesById and the projection disagree about {id}"
            );
        }
        assert_eq!(
            small["zonesById"].as_object().expect("zonesById").len(),
            3,
            "canonical by-id emit is missing rows"
        );
    }

    /// Deleting every zone must clear the wire, not leave the last-saved array parked. Without the
    /// explicit `extras.remove("zones")`, a reload-then-delete-all doc would keep re-emitting the
    /// zones it no longer has — a mission the author cannot un-fence.
    #[cfg(feature = "mission")]
    #[test]
    fn deleting_every_zone_clears_them_from_the_wire() {
        let doc = zones_fixture();
        let reloaded = save_and_reload(&doc);
        assert_eq!(reloaded.zone_count(), 2);

        reloaded.remove_zone("z_ao");
        reloaded.remove_zone("z_obj");
        assert_eq!(reloaded.zone_count(), 0);

        let compiled = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        assert!(
            wire_zones(&compiled).is_empty(),
            "deleted zones must not survive on the wire: {compiled}"
        );
    }

    /// **END TO END — a drawn zone reaches the MOD document.** Author → `compile_payload` →
    /// `flatten_to_mod_document`, asserting the exact polygon and circle land in `ModZone`.
    ///
    /// This is the claim the ticket ultimately rests on, and the two preceding tests do not make
    /// it: they prove the zone survives the editor's own save/reload loop, which is a closed
    /// editor↔editor circuit. `flatten` is a THIRD reader with its own `ZoneIn` deserialiser, and a
    /// payload can round-trip perfectly through hydrate while flatten drops it — `ZoneIn` is
    /// `#[serde(default)]`, so a wrong-typed or misnamed field is silently defaulted rather than
    /// refused. Only running the real compiler over the real bytes settles it.
    ///
    /// Note the assertion is on the geometry, not on `zones.len()`: `derive_zones` also synthesises
    /// spawn circles and a terrain boundary, so a count check would be satisfied by zones this
    /// document never authored.
    ///
    /// **MEASURED HERE — the mod boundary QUANTISES zone geometry to 0.1 m.**
    /// `flatten::round_coord` (flatten.rs:1823-1825, `(v * 10.0).round() / 10.0`) is applied to
    /// every polygon vertex and to a circle's `x`/`z`/`r`, deliberately, to match spawn-zone
    /// synthesis and the historical TS flatten. So the drawn `1000.25` reaches the mod as `1000.3`
    /// and `-3800.125` as `-3800.1`.
    ///
    /// The layering is correct and this test pins BOTH halves of it: the editor document and its
    /// payload keep full f64 precision (`authored_zones_survive_compile_hydrate_compile_whole`
    /// asserts `1000.25` exactly), and only the compiled mod document is rounded. The reason to
    /// pin it rather than just tolerate it is that it is invisible from the editor side — a draw
    /// tool that promised finer-than-decimetre vertex placement, or a downstream test that asserted
    /// exact float equality at the mod boundary, would be wrong in a way nothing else reports.
    #[cfg(feature = "mission")]
    #[test]
    fn authored_zones_reach_the_mod_document_through_flatten() {
        use crate::mission::flatten::{ModZoneShape, flatten_to_mod_document};

        let doc = zones_fixture();
        let compiled = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let bytes = serde_json::to_vec(&compiled).expect("serialise payload");

        let meta = crate::mission::flatten::MissionMeta {
            id: "11112222333344445555666677778888".into(),
            title: "T-211 zones".into(),
            author: "maker".into(),
            terrain: "everon".into(),
            custom_terrain_name: String::new(),
            max_players: 64,
            time_of_day: "05:30".into(),
            weather_preset: "clear".into(),
        };
        let mod_doc = flatten_to_mod_document(&meta, &bytes).expect("mission compiles");

        let ao = mod_doc
            .zones
            .iter()
            .find(|z| z.id == "z_ao")
            .expect("authored boundary zone never reached the mod document");
        assert_eq!(ao.kind, "boundary");
        assert_eq!(ao.label, "Area of Operations");
        match &ao.shape {
            // Drawn as .25 / .75 / .125; arrives quantised to 0.1 m by `round_coord`. The vertex
            // COUNT and ORDER are unchanged — this is rounding, not resampling or truncation.
            ModZoneShape::Polygon { polygon } => assert_eq!(
                polygon,
                &vec![
                    [1000.3, -4210.8],
                    [1600.5, -4210.8],
                    [1600.5, -3800.1],
                    [1000.3, -3800.1],
                ],
                "the polygon reached the mod with different vertices than were drawn \
                 (expected only `round_coord`'s 0.1 m quantisation)"
            ),
            other => panic!("boundary zone lost its polygon on the way to the mod: {other:?}"),
        }
        assert_eq!(
            ao.rules.as_ref().expect("rules reached the mod")["penalty"],
            serde_json::json!("kill"),
            "zoneRules must pass through flatten verbatim"
        );

        let obj = mod_doc
            .zones
            .iter()
            .find(|z| z.id == "z_obj")
            .expect("authored objective zone never reached the mod document");
        assert_eq!(obj.kind, "objective_capture");
        assert_eq!(obj.faction, "blufor");
        match &obj.shape {
            ModZoneShape::Circle { circle } => {
                // Same 0.1 m quantisation, and it applies to the RADIUS too — a drawn 175.75 m
                // circle is a 175.8 m circle in the mod.
                assert_eq!(circle.x, 1234.5, "x was exact at 0.1 m already");
                assert_eq!(circle.z, -3990.3, "z quantised from -3990.25");
                assert_eq!(circle.r, 175.8, "r quantised from 175.75");
            }
            other => panic!("objective zone lost its circle on the way to the mod: {other:?}"),
        }

        // T-201's synthesis still runs alongside authored zones: the authored `boundary` suppresses
        // the terrain fallback, and spawn circles are additive.
        assert!(
            !mod_doc.zones.iter().any(|z| z.id == "z_bounds"),
            "an authored boundary must suppress the synthesised terrain fallback"
        );
    }

    /// Reshaping replaces the whole `shape` — a polygon retyped to a circle must not leave BOTH
    /// `oneOf` branches on the row (which is schema-invalid), and vice versa.
    #[test]
    fn reshaping_a_zone_leaves_exactly_one_oneof_branch() {
        let doc = MissionDocCore::new();
        doc.add_polygon_zone("z", "boundary", &[0.5, 0.5, 10.5, 0.5, 10.5, 10.5]);
        doc.set_zone_circle("z", 50.5, 60.5, 25.5);

        let zones: serde_json::Value = serde_json::from_str(&doc.zones_json()).expect("zones_json");
        let shape = zones["z"]["shape"].as_object().expect("shape");
        assert!(
            shape.contains_key("circle"),
            "reshape did not take: {shape:?}"
        );
        assert!(
            !shape.contains_key("polygon"),
            "the polygon branch survived a reshape to circle — row is oneOf-invalid: {shape:?}"
        );

        doc.set_zone_polygon("z", &[1.5, 1.5, 2.5, 1.5, 2.5, 2.5]);
        let zones: serde_json::Value = serde_json::from_str(&doc.zones_json()).expect("zones_json");
        let shape = zones["z"]["shape"].as_object().expect("shape");
        assert!(shape.contains_key("polygon"));
        assert!(
            !shape.contains_key("circle"),
            "the circle branch survived a reshape to polygon: {shape:?}"
        );
    }

    /// Optional keys must be REMOVABLE, and an empty `rules` must vanish rather than persist as
    /// `{}` — the doc has to be able to express "unauthored", not just "authored something".
    #[test]
    fn clearing_optional_zone_fields_removes_the_keys() {
        let doc = MissionDocCore::new();
        doc.add_circle_zone("z", "objective_hold_until", 5.5, 6.5, 7.5);
        doc.set_zone_label("z", Some("Hill 402"));
        doc.set_zone_faction("z", Some("opfor"));
        doc.set_zone_rules("z", Some(r#"{"holdSeconds":600.5}"#));

        let row = |d: &MissionDocCore| -> serde_json::Value {
            serde_json::from_str::<serde_json::Value>(&d.zones_json()).expect("zones_json")["z"]
                .clone()
        };
        let r = row(&doc);
        assert_eq!(r["label"], "Hill 402");
        assert_eq!(r["faction"], "opfor");
        assert_eq!(r["rules"]["holdSeconds"], 600.5);

        doc.set_zone_label("z", None);
        doc.set_zone_faction("z", None);
        doc.set_zone_rules("z", None);
        let r = row(&doc);
        assert!(r.get("label").is_none(), "label not removed: {r}");
        assert!(r.get("faction").is_none(), "faction not removed: {r}");
        assert!(r.get("rules").is_none(), "rules not removed: {r}");

        // An empty rules object is "unauthored", not a third state.
        doc.set_zone_rules("z", Some("{}"));
        assert!(
            row(&doc).get("rules").is_none(),
            "empty rules must not be written"
        );
        // Malformed JSON clears rather than writing a scalar into an object-typed key.
        doc.set_zone_rules("z", Some(r#"{"holdSeconds":1.5}"#));
        doc.set_zone_rules("z", Some("not json"));
        assert!(
            row(&doc).get("rules").is_none(),
            "malformed rules must clear the key"
        );

        // An empty label is authorable and distinct from absent (schema has no minLength).
        doc.set_zone_label("z", Some(""));
        assert_eq!(
            row(&doc)["label"],
            "",
            "empty label must be a writable state"
        );
    }

    /// Zones are undo-scoped like every other root, and a hydrate (INIT) is not an undo step.
    #[test]
    fn zone_edits_are_undoable_and_hydrate_is_not() {
        let mut doc = MissionDocCore::new();
        doc.add_circle_zone("z1", "boundary", 1.5, 2.5, 3.5);
        assert_eq!(doc.zone_count(), 1);
        assert!(doc.can_undo(), "a drawn zone must be undoable");
        assert!(doc.undo());
        assert_eq!(doc.zone_count(), 0, "undo did not remove the zone");
        assert!(doc.redo());
        assert_eq!(doc.zone_count(), 1, "redo did not restore the zone");

        // A load is not a user gesture.
        let fresh = MissionDocCore::new();
        fresh.set_origin_init(true);
        fresh.hydrate(
            &serde_json::json!({
                "zones": [ { "id": "z", "type": "spawn",
                             "shape": { "circle": { "x": 1.5, "z": 2.5, "r": 3.5 } } } ],
                "editor": { "factions": [], "squads": [], "slots": [], "editorLayers": [] }
            })
            .to_string(),
            "lyr",
        );
        fresh.set_origin_init(false);
        assert_eq!(fresh.zone_count(), 1);
        assert!(!fresh.can_undo(), "hydrate must not create an undo step");
    }

    /// A zone-only document has unsaved content — the conflict gate must not discard it.
    #[test]
    fn a_zone_alone_counts_as_local_content() {
        let doc = MissionDocCore::new();
        assert!(!doc.has_content(), "fresh doc has no content");
        doc.add_circle_zone("z", "base_protection", 10.5, 20.5, 30.5);
        assert!(
            doc.has_content(),
            "a drawn play area is authored work and must count as local content"
        );
    }

    /// A flat ring with an odd coordinate count drops the unpaired tail rather than writing a
    /// 1-element vertex, which `$defs/polygon` (`minItems: 2`, `maxItems: 2`) would reject.
    #[test]
    fn an_unpaired_trailing_coordinate_is_dropped() {
        let doc = MissionDocCore::new();
        doc.add_polygon_zone("z", "boundary", &[0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5]);
        let zones: serde_json::Value = serde_json::from_str(&doc.zones_json()).expect("zones_json");
        let ring = zones["z"]["shape"]["polygon"]
            .as_array()
            .expect("polygon array");
        assert_eq!(ring.len(), 3, "expected 3 whole vertices: {ring:?}");
        for pt in ring {
            assert_eq!(pt.as_array().expect("vertex").len(), 2, "{pt:?}");
        }
    }

    /* ─────────────────────── T-079 — triggers (editor half): round trip, owner edge, wire ─────────────────────── */

    /// A doc carrying a complete editor graph (so `compile_payload` has a mission to compile) plus
    /// two triggers authored through the mutators exactly as the draw tool + panel would: a polygon
    /// presence trigger owned by a placed slot, and a circle timer trigger with no owner but with
    /// rules. **Coordinates are deliberately NON-INTEGRAL** for the same reason the zones fixture is —
    /// an all-integer fixture would survive a bug that coerced every coordinate to `Any::BigInt` and
    /// back, so `-4210.75` is what lets the round-trip prove the geometry itself came back.
    #[cfg(feature = "mission")]
    fn triggers_fixture() -> MissionDocCore {
        let doc = MissionDocCore::new();
        // A COMPLETE editor graph (a bare slot fails `flatten_to_mod_document` on NoSlots, which
        // would fail the round trip for a reason unrelated to triggers). The slot `s1` is the owner
        // the first trigger links to.
        doc.add_faction("f1", "BLUFOR", "US");
        doc.add_squad("sq1", "f1", "Alpha", Some("Alpha".to_string()));
        doc.add_slot(
            "s1", "sq1", "lyr", 0, "Rifleman", None, None, 100.5, 200.5, 0.0, 0.0,
        );

        // A polygon PRESENCE trigger, named, OWNED by the placed slot, with rules.
        doc.add_polygon_trigger(
            "t_amb",
            "presence",
            &[
                1000.25, -4210.75, 1600.5, -4210.75, 1600.5, -3800.125, 1000.25, -3800.125,
            ],
        );
        doc.set_trigger_name("t_amb", Some("Ambush"));
        doc.set_trigger_owner("t_amb", Some("s1"));
        doc.set_trigger_rules(
            "t_amb",
            Some(r#"{"graceSeconds":45.5,"contestable":false}"#),
        );

        // A circle TIMER trigger, unowned, with a rule.
        doc.add_circle_trigger("t_timer", "timer", 1234.5, -3990.25, 175.75);
        doc.set_trigger_rules("t_timer", Some(r#"{"announceEverySeconds":12.5}"#));
        doc
    }

    /// Pull the triggers array off a compiled wire payload, whichever route put it there. Once T-706
    /// declares `triggers` and `compile_payload` authors it, this reads the authored key instead of
    /// the promoted one, with no test change — which is the point (mirrors `wire_zones`).
    #[cfg(feature = "mission")]
    fn wire_triggers(payload: &serde_json::Value) -> Vec<serde_json::Value> {
        payload
            .get("triggers")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    }

    /// **THE ROUND TRIP — author → persist → reload → identical, through the REAL compile→hydrate.**
    ///
    /// Asserts on the WHOLE trigger object, not the presence of a `triggers` key: a test that only
    /// checked `is_array()` would pass green over a trigger whose `shape` was dropped, whose polygon
    /// was truncated, whose `ownerId` / `activation` were lost, or whose `rules` became `{}`. The
    /// cycle runs TWICE (`compile → hydrate → compile`) because one pass cannot tell "the doc stored
    /// it" from "the doc round-tripped it" — a value merely echoed out of the parked side-channel
    /// survives one compile and dies on the second (the T-219 class). This is the ticket's
    /// "store round-trip through REAL compile→hydrate" acceptance, geometry + owner + activation
    /// + rules included.
    #[cfg(feature = "mission")]
    #[test]
    fn authored_triggers_survive_compile_hydrate_compile_whole() {
        let doc = triggers_fixture();

        let compiled = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let first = wire_triggers(&compiled);
        assert_eq!(
            first.len(),
            2,
            "both authored triggers must reach the wire payload: {compiled}"
        );

        // Full-value expectation — every key spelled out: geometry, owner edge, activation, rules.
        let expect_amb = serde_json::json!({
            "id": "t_amb",
            "name": "Ambush",
            "ownerId": "s1",
            "activation": "presence",
            "shape": { "polygon": [
                [1000.25, -4210.75],
                [1600.5,  -4210.75],
                [1600.5,  -3800.125],
                [1000.25, -3800.125]
            ]},
            "rules": { "graceSeconds": 45.5, "contestable": false }
        });
        let expect_timer = serde_json::json!({
            "id": "t_timer",
            "activation": "timer",
            "shape": { "circle": { "x": 1234.5, "z": -3990.25, "r": 175.75 } },
            "rules": { "announceEverySeconds": 12.5 }
        });

        let by_id = |rows: &[serde_json::Value], id: &str| -> serde_json::Value {
            rows.iter()
                .find(|r| r["id"] == id)
                .unwrap_or_else(|| panic!("trigger {id} missing from {rows:?}"))
                .clone()
        };

        assert_eq!(
            by_id(&first, "t_amb"),
            expect_amb,
            "compile #1 dropped part of t_amb (geometry / owner / activation / rules)"
        );
        assert_eq!(
            by_id(&first, "t_timer"),
            expect_timer,
            "compile #1 dropped part of t_timer"
        );

        // Reload from the persisted payload, exactly as the editor does, then recompile.
        let reloaded = save_and_reload(&doc);
        assert_eq!(
            reloaded.trigger_count(),
            2,
            "hydrate must restore both triggers"
        );

        let recompiled = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        let second = wire_triggers(&recompiled);
        assert_eq!(
            by_id(&second, "t_amb"),
            expect_amb,
            "t_amb did not survive save→reload→save WHOLE"
        );
        assert_eq!(
            by_id(&second, "t_timer"),
            expect_timer,
            "t_timer did not survive save→reload→save WHOLE"
        );

        // T-432 still holds: the side-channel name never becomes a wire key.
        assert!(
            recompiled.get("payloadExtras").is_none(),
            "payloadExtras must not reach the wire: {recompiled}"
        );
    }

    /// **CONN-TRG-OWNER-001 — the owner edge is assignable, clearable, and TOLERATES a dangling
    /// owner.** The data edge is a plain `ownerId` write; deleting the owning entity must leave the
    /// trigger intact with an id that now resolves to nothing — never a panic, never a cascade that
    /// removes the trigger. FIRES the rule (perturb / fail / restore): a delete that also removed the
    /// trigger, or a `set_trigger_owner` that refused an unknown id, would fail the middle assertion.
    #[test]
    fn owner_edge_assigns_clears_and_tolerates_dangling() {
        let doc = MissionDocCore::new();
        doc.add_faction("f1", "BLUFOR", "US");
        doc.add_squad("sq1", "f1", "Alpha", Some("Alpha".to_string()));
        doc.add_slot(
            "s1", "sq1", "lyr", 0, "Rifleman", None, None, 10.5, 20.5, 0.0, 0.0,
        );
        doc.add_circle_trigger("t1", "presence", 50.5, 60.5, 25.5);

        let row = |d: &MissionDocCore| -> serde_json::Value {
            serde_json::from_str::<serde_json::Value>(&d.triggers_json()).expect("triggers_json")["t1"]
                .clone()
        };
        // Baseline: assign the edge to the placed slot.
        doc.set_trigger_owner("t1", Some("s1"));
        assert_eq!(row(&doc)["ownerId"], "s1", "owner edge did not record");

        // Perturb the world OUT from under the edge: delete the owning slot. The edge is allowed to
        // dangle — the trigger survives with an ownerId that now resolves to no entity.
        doc.remove_slots(vec!["s1".to_string()]);
        assert_eq!(
            doc.trigger_count(),
            1,
            "deleting the owner must NOT delete the trigger (a dangling edge, not a cascade)"
        );
        assert_eq!(
            row(&doc)["ownerId"],
            "s1",
            "the dangling ownerId stays on the row; readers resolve it to nothing, they do not \
             rewrite it"
        );
        // The slot really is gone (so the edge really is dangling, not just claimed to be).
        let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
        assert!(
            slots.get("s1").is_none(),
            "the owner slot must actually be removed for this to be a dangling edge: {slots}"
        );

        // Restore: clearing the edge removes the key entirely (unowned), a distinct state from "owns
        // a deleted entity".
        doc.set_trigger_owner("t1", None);
        assert!(
            row(&doc).get("ownerId").is_none(),
            "clearing the owner must remove the key: {}",
            row(&doc)
        );
    }

    /// Reshaping a trigger replaces the whole `shape` — a polygon retyped to a circle must not leave
    /// BOTH `oneOf` branches (schema-invalid), and it must KEEP `name` / `ownerId` / `activation` /
    /// `rules` (the whole reason reshape exists vs delete-and-redraw). Trigger geometry is the SECOND
    /// CONSUMER of the same `circle_shape_any`/`polygon_shape_any` the zone tool uses.
    #[test]
    fn reshaping_a_trigger_keeps_metadata_and_one_oneof_branch() {
        let doc = MissionDocCore::new();
        doc.add_polygon_trigger("t", "radio", &[0.5, 0.5, 10.5, 0.5, 10.5, 10.5]);
        doc.set_trigger_name("t", Some("Extract"));
        doc.set_trigger_owner("t", Some("veh1"));
        doc.set_trigger_rules("t", Some(r#"{"holdSeconds":90.5}"#));

        doc.set_trigger_circle("t", 50.5, 60.5, 25.5);
        let row = |d: &MissionDocCore| -> serde_json::Value {
            serde_json::from_str::<serde_json::Value>(&d.triggers_json()).expect("triggers_json")["t"]
                .clone()
        };
        let r = row(&doc);
        let shape = r["shape"].as_object().expect("shape");
        assert!(
            shape.contains_key("circle"),
            "reshape did not take: {shape:?}"
        );
        assert!(
            !shape.contains_key("polygon"),
            "the polygon branch survived a reshape to circle — oneOf-invalid: {shape:?}"
        );
        // Every non-geometry field survived the reshape.
        assert_eq!(r["name"], "Extract", "reshape wiped the name");
        assert_eq!(r["ownerId"], "veh1", "reshape wiped the owner edge");
        assert_eq!(r["activation"], "radio", "reshape wiped the activation");
        assert_eq!(r["rules"]["holdSeconds"], 90.5, "reshape wiped the rules");
    }

    /// Optional keys must be REMOVABLE and an empty `rules` must vanish rather than persist as `{}`
    /// — triggers reuse the zone-rules storage discipline verbatim. `activation` is NOT optional
    /// (every trigger carries one), so it is not cleared here.
    #[test]
    fn clearing_optional_trigger_fields_removes_the_keys() {
        let doc = MissionDocCore::new();
        doc.add_circle_trigger("t", "presence", 5.5, 6.5, 7.5);
        doc.set_trigger_name("t", Some("Alarm"));
        doc.set_trigger_owner("t", Some("s9"));
        doc.set_trigger_rules("t", Some(r#"{"points":3.5}"#));

        let row = |d: &MissionDocCore| -> serde_json::Value {
            serde_json::from_str::<serde_json::Value>(&d.triggers_json()).expect("triggers_json")["t"]
                .clone()
        };
        let r = row(&doc);
        assert_eq!(r["name"], "Alarm");
        assert_eq!(r["ownerId"], "s9");
        assert_eq!(r["rules"]["points"], 3.5);
        assert_eq!(r["activation"], "presence", "activation is always present");

        doc.set_trigger_name("t", None);
        doc.set_trigger_owner("t", None);
        doc.set_trigger_rules("t", None);
        let r = row(&doc);
        assert!(r.get("name").is_none(), "name not removed: {r}");
        assert!(r.get("ownerId").is_none(), "ownerId not removed: {r}");
        assert!(r.get("rules").is_none(), "rules not removed: {r}");
        assert_eq!(
            r["activation"], "presence",
            "activation must survive clearing the optionals"
        );

        // An empty rules object is "unauthored", not a third state (the zone-rules identity).
        doc.set_trigger_rules("t", Some("{}"));
        assert!(
            row(&doc).get("rules").is_none(),
            "empty rules must not be written"
        );
        // Retyping the activation is a plain overwrite.
        doc.set_trigger_activation("t", "timer");
        assert_eq!(
            row(&doc)["activation"],
            "timer",
            "activation retype did not take"
        );
    }

    /// Triggers are undo-scoped like every other root, and a hydrate (INIT) is not an undo step.
    #[test]
    fn trigger_edits_are_undoable_and_hydrate_is_not() {
        let mut doc = MissionDocCore::new();
        doc.add_circle_trigger("t1", "presence", 1.5, 2.5, 3.5);
        assert_eq!(doc.trigger_count(), 1);
        assert!(doc.can_undo(), "a drawn trigger must be undoable");
        assert!(doc.undo());
        assert_eq!(doc.trigger_count(), 0, "undo did not remove the trigger");
        assert!(doc.redo());
        assert_eq!(doc.trigger_count(), 1, "redo did not restore the trigger");

        // A load is not a user gesture.
        let fresh = MissionDocCore::new();
        fresh.set_origin_init(true);
        fresh.hydrate(
            &serde_json::json!({
                "triggers": [ { "id": "t", "activation": "radio",
                                "shape": { "circle": { "x": 1.5, "z": 2.5, "r": 3.5 } } } ],
                "editor": { "factions": [], "squads": [], "slots": [], "editorLayers": [] }
            })
            .to_string(),
            "lyr",
        );
        fresh.set_origin_init(false);
        assert_eq!(fresh.trigger_count(), 1);
        assert!(!fresh.can_undo(), "hydrate must not create an undo step");
    }

    /// Deleting every trigger must clear the wire, not leave the last-saved array parked (the zones
    /// absence rule, mirrored). Without the explicit `extras.remove("triggers")`, a
    /// reload-then-delete-all doc would keep re-emitting triggers it no longer has.
    #[cfg(feature = "mission")]
    #[test]
    fn deleting_every_trigger_clears_them_from_the_wire() {
        let doc = triggers_fixture();
        let reloaded = save_and_reload(&doc);
        assert_eq!(reloaded.trigger_count(), 2);

        reloaded.remove_trigger("t_amb");
        reloaded.remove_trigger("t_timer");
        assert_eq!(reloaded.trigger_count(), 0);

        let compiled = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        assert!(
            wire_triggers(&compiled).is_empty(),
            "deleted triggers must not survive on the wire: {compiled}"
        );
    }

    /* ─────────────────────── T-650 — saved compositions: round trip, edit, wire ─────────────────────── */

    /// A composition row's self-contained JSON, exactly as `editor_ops::save_composition` builds it:
    /// three metadata fields plus a multi-entity `entities` array of RELATIVE-OFFSET entries — a
    /// slot, a vehicle (carrying a `crew` SHAPE), and an object. **Offsets are deliberately
    /// NON-INTEGRAL** for the same reason the zones fixture is: an all-integer fixture would survive
    /// a bug that coerced every coordinate to `Any::BigInt` and back, so `-12.75` is what lets the
    /// round-trip prove the geometry itself came back rather than merely a number-shaped thing.
    #[cfg(feature = "mission")]
    fn composition_row_json() -> String {
        serde_json::json!({
            "id": "c1",
            "title": "Fireteam + Technical",
            "author": "Sam",
            "category": "Infantry",
            "entities": [
                { "kind": "slot",    "dx":  -12.75, "dz": 8.5,   "rotation": 45.5,
                  "role": "Squad Leader", "tag": "SL", "assetId": "Prefab/SL.et", "stance": "crouch",
                  "loadout": { "gear": { "primary": "M4" } } },
                { "kind": "slot",    "dx":   12.25, "dz": -8.5,  "rotation": 0.0,
                  "role": "Rifleman", "tag": "", "assetId": "Prefab/Rifleman.et", "stance": "stand" },
                { "kind": "vehicle", "dx":    0.5,  "dz": 30.125, "rotation": 270.75,
                  "resourceName": "Prefab/Technical.et", "crewed": true,
                  "crew": { "driver": "s0", "gunner": "s1" } },
                { "kind": "object",  "dx":  -30.5,  "dz": 0.25,  "rotation": 90.0,
                  "alias": "sandbag_wall", "resourceName": "Prefab/Sandbag.et", "faction": "blufor" }
            ]
        })
        .to_string()
    }

    /// A doc with a complete editor graph (so `compile_payload` has a mission to compile) plus one
    /// saved composition authored through the mutator.
    #[cfg(feature = "mission")]
    fn compositions_fixture() -> MissionDocCore {
        let doc = MissionDocCore::new();
        doc.add_faction("f1", "BLUFOR", "US");
        doc.add_squad("sq1", "f1", "Alpha", Some("Alpha".to_string()));
        doc.add_slot(
            "s1", "sq1", "lyr", 0, "Rifleman", None, None, 100.5, 200.5, 0.0, 0.0,
        );
        doc.add_composition("c1", &composition_row_json());
        doc
    }

    /// Pull the compositions array off a compiled wire payload, whichever route put it there — the
    /// `wire_zones` idiom. Once `compile_payload` authors `compositions` itself this reads the
    /// authored key instead of the promoted one, with no test change.
    #[cfg(feature = "mission")]
    fn wire_compositions(payload: &serde_json::Value) -> Vec<serde_json::Value> {
        payload
            .get("compositions")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    }

    /// Canonicalise a JSON value for the round-trip comparisons: coerce every INTEGER-VALUED
    /// `Number(f64)` to its `i64` form. This is not a fudge — it absorbs exactly the documented
    /// `Any::BigInt` vs `Any::Number` encoding (`any_to_f64`'s reason to exist): yrs re-emits a
    /// stored `45.5` as `Number(45.5)` but a stored `0.0` as an integer `0`, so a `serde_json::json!`
    /// expectation of `0.0` (`Number(0.0)`) would never `Value`-equal the round-tripped `Number(0)`
    /// even though the two are the same number and the same JSON text. Nothing else is normalised —
    /// a dropped entity, a truncated offset, or a lost nested blob still fails the comparison, which
    /// is the whole point of comparing by value.
    #[cfg(feature = "mission")]
    fn canon(v: &serde_json::Value) -> serde_json::Value {
        use serde_json::Value;
        match v {
            Value::Number(n) => match n.as_f64() {
                // Integer-valued float → integer form, matching yrs's BigInt re-emit.
                Some(f) if f.fract() == 0.0 && f.is_finite() => Value::from(f as i64),
                _ => v.clone(),
            },
            Value::Array(a) => Value::Array(a.iter().map(canon).collect()),
            Value::Object(o) => {
                Value::Object(o.iter().map(|(k, val)| (k.clone(), canon(val))).collect())
            }
            _ => v.clone(),
        }
    }

    /// **THE ROUND TRIP — save → persist → reload → identical, entities included.** The wave-103
    /// class: run `compile_payload → hydrate → compile_payload` through the REAL compiler (hence
    /// `--features doc,mission`), and compare the WHOLE composition row by value. A test that only
    /// checked the `compositions` key was an array — or that the ids survived — would pass green over
    /// a composition whose `entities` were dropped, whose offsets were snapped to an integer grid, or
    /// whose nested `loadout`/`crew` blobs were flattened. Two passes are required because one cannot
    /// distinguish "the doc stored it" from "the doc echoed it out of the parked side-channel": a
    /// value merely relayed survives one compile and dies on the second (the T-219 class).
    #[cfg(feature = "mission")]
    #[test]
    fn saved_composition_survives_compile_hydrate_compile_whole() {
        let doc = compositions_fixture();

        let compiled = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let first = wire_compositions(&compiled);
        assert_eq!(
            first.len(),
            1,
            "one composition on the first compile: {compiled}"
        );
        let expected: serde_json::Value =
            serde_json::from_str(&composition_row_json()).expect("expected row");
        assert_eq!(
            canon(&first[0]),
            canon(&expected),
            "the first compile changed the row"
        );

        // Reload a fresh doc from the compiled payload, then compile AGAIN.
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&compiled.to_string(), "lyr");
        assert_eq!(
            reloaded.composition_count(),
            1,
            "hydrate lost the composition"
        );

        let recompiled = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        let second = wire_compositions(&recompiled);
        assert_eq!(
            second.len(),
            1,
            "the composition died on the SECOND compile — echoed, not stored: {recompiled}"
        );
        assert_eq!(
            canon(&second[0]),
            canon(&expected),
            "the row is not identical after the round trip (nested entities/offsets lost?): {}",
            second[0]
        );
    }

    /// **FIRE-ONCE (the offset rule) — perturb / fail / restore.** The relative-offset geometry is
    /// the load-bearing content: prove a corrupted offset is actually caught by the by-value
    /// comparison the round-trip test relies on, rather than papered over. Perturb one entry's `dx`,
    /// assert the round trip now DIFFERS (RED), then restore and assert it matches again (GREEN).
    #[cfg(feature = "mission")]
    #[test]
    fn composition_offset_perturbation_is_caught_by_the_round_trip() {
        // Baseline: the honest fixture round-trips equal.
        let good = compositions_fixture();
        let good_wire = wire_compositions(&crate::mission::compile::compile_payload(
            &good.small_maps_json(),
            &good.slots_json(),
            false,
        ));
        let expected: serde_json::Value =
            serde_json::from_str(&composition_row_json()).expect("expected");
        assert_eq!(
            canon(&good_wire[0]),
            canon(&expected),
            "baseline must match before perturbing"
        );

        // ── PERTURB: author a row whose first entity's `dx` is shifted by 100 m. ──
        let mut perturbed_row: serde_json::Value =
            serde_json::from_str(&composition_row_json()).expect("row");
        perturbed_row["entities"][0]["dx"] =
            serde_json::json!(perturbed_row["entities"][0]["dx"].as_f64().unwrap() + 100.0);
        let bad = compositions_fixture();
        bad.add_composition("c1", &perturbed_row.to_string()); // overwrite c1 with the perturbed row
        let bad_wire = wire_compositions(&crate::mission::compile::compile_payload(
            &bad.small_maps_json(),
            &bad.slots_json(),
            false,
        ));
        assert_ne!(
            canon(&bad_wire[0]),
            canon(&expected),
            "a shifted offset must be OBSERVABLE in the compiled row — the by-value check is real"
        );

        // ── RESTORE: re-author the honest row; the difference is gone. ──
        bad.add_composition("c1", &composition_row_json());
        let restored_wire = wire_compositions(&crate::mission::compile::compile_payload(
            &bad.small_maps_json(),
            &bad.slots_json(),
            false,
        ));
        assert_eq!(
            canon(&restored_wire[0]),
            canon(&expected),
            "restoring the honest offset must bring the row back to identical"
        );
    }

    /// The two emit routes `small_maps_json` writes (canonical `compositionsById`, transitional
    /// `payloadExtras.compositions`) must describe the same compositions in the same ORDER, or the
    /// compile.rs companion change would silently reorder every mission's compositions the day it
    /// lands. Mirrors `zones_by_id_and_extras_projection_agree_on_order`.
    #[cfg(feature = "mission")]
    #[test]
    fn compositions_by_id_and_extras_projection_agree_on_order() {
        // Hydrate establishes an authored order that is NOT the map's own iteration order.
        let incoming = serde_json::json!({
            "schemaVersion": 1,
            "map": { "terrain": "everon" },
            "environment": {},
            "compositions": [
                { "id": "c_c", "title": "C", "author": "a", "category": "x", "entities": [] },
                { "id": "c_a", "title": "A", "author": "a", "category": "x", "entities": [] },
                { "id": "c_b", "title": "B", "author": "a", "category": "x", "entities": [] }
            ],
            "editor": { "factions": [], "squads": [], "slots": [], "editorLayers": [] }
        });
        let doc = MissionDocCore::new();
        doc.hydrate(&incoming.to_string(), "lyr");

        let small = small_maps(&doc);
        let projected: Vec<&str> = small["payloadExtras"]["compositions"]
            .as_array()
            .expect("projected compositions array")
            .iter()
            .map(|c| c["id"].as_str().expect("id"))
            .collect();
        assert_eq!(
            projected,
            ["c_c", "c_a", "c_b"],
            "the projection must replay hydrate's authored order, not map order"
        );
        // Every projected row is the same object the canonical by-id map holds.
        for id in &projected {
            assert_eq!(
                small["compositionsById"][id],
                *small["payloadExtras"]["compositions"]
                    .as_array()
                    .expect("arr")
                    .iter()
                    .find(|c| c["id"] == *id)
                    .expect("row"),
                "compositionsById and the projection disagree about {id}"
            );
        }
        assert_eq!(
            small["compositionsById"]
                .as_object()
                .expect("compositionsById")
                .len(),
            3,
            "canonical by-id emit is missing rows"
        );
    }

    /// Deleting every composition must clear the wire, not leave the last-saved array parked — the
    /// `deleting_every_zone_clears_them_from_the_wire` rule.
    #[cfg(feature = "mission")]
    #[test]
    fn deleting_every_composition_clears_them_from_the_wire() {
        let doc = compositions_fixture();
        let reloaded = save_and_reload(&doc);
        assert_eq!(reloaded.composition_count(), 1);

        reloaded.remove_composition("c1");
        assert_eq!(reloaded.composition_count(), 0);

        let compiled = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        assert!(
            wire_compositions(&compiled).is_empty(),
            "a deleted composition must not survive on the wire: {compiled}"
        );
    }

    /// EDIT (COMP-EDIT-001) — rename / recategorize edit the metadata WHOLE-`Any` and preserve the
    /// `entities` payload, INCLUDING after a hydrate (where the row is no longer a freshly-authored
    /// map). This is the crew post-hydrate class: a naive edit that matched only the fresh shape
    /// would either no-op or wipe `entities` on a reloaded mission.
    #[cfg(feature = "mission")]
    #[test]
    fn composition_rename_recategorize_after_hydrate_preserves_entities() {
        let doc = save_and_reload(&compositions_fixture());
        // Precondition: entities survived the reload as a read.
        let before = serde_json::from_str::<serde_json::Value>(&doc.compositions_json())
            .expect("compositions_json");
        assert_eq!(
            before["c1"]["entities"].as_array().expect("entities").len(),
            4,
            "the reloaded composition must carry all four entities"
        );

        doc.set_composition_title("c1", "Renamed Squad");
        doc.set_composition_category("c1", "Armor");

        let after = serde_json::from_str::<serde_json::Value>(&doc.compositions_json())
            .expect("compositions_json");
        assert_eq!(
            after["c1"]["title"], "Renamed Squad",
            "rename did not apply"
        );
        assert_eq!(
            after["c1"]["category"], "Armor",
            "recategorize did not apply"
        );
        assert_eq!(
            after["c1"]["author"], "Sam",
            "the untouched author metadata must survive the edit"
        );
        assert_eq!(
            after["c1"]["entities"], before["c1"]["entities"],
            "a metadata edit on a HYDRATED row must not wipe the captured entities"
        );
    }

    /// A saved composition is authored work: it counts as local content (the conflict gate must not
    /// discard it) and a save/rename/delete is UNDOABLE, while a hydrate is not — the zones-undo
    /// class.
    #[cfg(feature = "mission")]
    #[test]
    fn composition_edits_are_undoable_and_count_as_content() {
        let mut doc = MissionDocCore::new();
        assert!(!doc.has_content(), "fresh doc has no content");
        doc.add_composition("c1", &composition_row_json());
        assert!(
            doc.has_content(),
            "a saved composition is authored work and must count as local content"
        );
        assert_eq!(doc.composition_count(), 1);
        assert!(doc.can_undo(), "a saved composition must be undoable");
        assert!(doc.undo());
        assert_eq!(
            doc.composition_count(),
            0,
            "undo did not remove the composition"
        );
        assert!(doc.redo());
        assert_eq!(
            doc.composition_count(),
            1,
            "redo did not restore the composition"
        );

        // A rename is its own undo step.
        doc.set_composition_title("c1", "Renamed");
        assert!(doc.undo(), "the rename must be undoable");
        let after = serde_json::from_str::<serde_json::Value>(&doc.compositions_json())
            .expect("compositions_json");
        assert_eq!(
            after["c1"]["title"], "Fireteam + Technical",
            "undo did not restore the original title"
        );

        // A load is not a user gesture.
        let fresh = MissionDocCore::new();
        fresh.set_origin_init(true);
        fresh.hydrate(
            &serde_json::json!({
                "compositions": [ { "id": "c", "title": "T", "author": "a", "category": "x", "entities": [] } ],
                "editor": { "factions": [], "squads": [], "slots": [], "editorLayers": [] }
            })
            .to_string(),
            "lyr",
        );
        fresh.set_origin_init(false);
        assert_eq!(fresh.composition_count(), 1);
        assert!(!fresh.can_undo(), "hydrate must not create an undo step");
    }

    /// **PLACE (COMP-PLACE-001) — relative-offset math + ONE undo step, multi-entity fixture.**
    /// Placing the four-entity fixture at a drop point must (1) re-anchor every entry to
    /// `drop + (dx, dz)` so the RELATIVE OFFSETS between entities are preserved, (2) write the slot
    /// into `slots`, the vehicle into `vehicles` (carrying its crew SHAPE), and the object into
    /// `entities`, and (3) be a SINGLE undo step (one Ctrl+Z removes the whole placement).
    #[cfg(feature = "mission")]
    #[test]
    fn placing_a_composition_preserves_offsets_in_one_undo_step() {
        let mut doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_editor_layer("L", "Layer", None);
        doc.set_origin_init(false);

        let row: serde_json::Value = serde_json::from_str(&composition_row_json()).expect("row");
        let entities = row["entities"].to_string();
        // Four entities → four minted ids (slot, slot, vehicle, object — the fixture order).
        let ids = vec![
            "p0".to_string(),
            "p1".to_string(),
            "p2".to_string(),
            "p3".to_string(),
        ];
        let (drop_x, drop_y) = (1000.0, 2000.0);
        let written = doc.place_composition(
            &entities, &ids, "BLUFOR", "L", drop_x, drop_y, 12800.0, 12800.0,
        );
        assert_eq!(written.len(), 4, "all four entities placed");

        // (1) Offsets preserved: each world position is drop + the entry's (dx, dz).
        let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
        // Entity 0 was the SL slot at (dx -12.75, dz 8.5).
        assert_eq!(
            slots["p0"]["position"]["x"],
            drop_x - 12.75,
            "slot0 x offset"
        );
        assert_eq!(slots["p0"]["position"]["y"], drop_y + 8.5, "slot0 y offset");
        assert_eq!(slots["p0"]["role"], "Squad Leader");
        assert_eq!(
            slots["p0"]["position"]["rotation"], 45.5,
            "slot0 heading kept"
        );
        // The captured loadout blob survived onto the placed slot.
        assert_eq!(slots["p0"]["loadout"]["gear"]["primary"], "M4");
        // Entity 1, the Rifleman, at (dx 12.25, dz -8.5): the RELATIVE spacing to slot0 is intact.
        assert_eq!(slots["p1"]["position"]["x"], drop_x + 12.25);
        assert_eq!(slots["p1"]["position"]["y"], drop_y - 8.5);

        // (2) The vehicle landed in `vehicles` with its heading, side, and crew SHAPE.
        let vehs = vehicles_of(&doc);
        assert_eq!(vehs["p2"]["resourceName"], "Prefab/Technical.et");
        assert_eq!(vehs["p2"]["position"]["x"], drop_x + 0.5);
        assert_eq!(
            vehs["p2"]["position"]["rotation"], 270.75,
            "vehicle heading kept"
        );
        assert_eq!(vehs["p2"]["factionId"], "faction-BLUFOR");
        assert_eq!(
            vehs["p2"]["crew"]["driver"], "s0",
            "crew shape carried verbatim"
        );
        // The object landed in `entities`.
        let small = small_maps(&doc);
        assert_eq!(small["entitiesById"]["p3"]["alias"], "sandbag_wall");
        assert_eq!(small["entitiesById"]["p3"]["faction"], "blufor");
        assert_eq!(small["entitiesById"]["p3"]["position"]["x"], drop_x - 30.5);

        // (3) ONE undo step: a single undo removes the whole placement.
        assert!(doc.undo(), "the placement must be undoable");
        let slots_after: serde_json::Value =
            serde_json::from_str(&doc.slots_json()).expect("slots_json");
        assert!(
            slots_after.get("p0").is_none() && slots_after.get("p1").is_none(),
            "one undo must remove every placed slot: {slots_after}"
        );
        assert!(
            vehicles_of(&doc).get("p2").is_none(),
            "one undo must remove the placed vehicle too — it was the same step"
        );
        assert!(
            small_maps(&doc)["entitiesById"].get("p3").is_none(),
            "one undo must remove the placed object too"
        );
    }

    /// A malformed composition JSON (a non-object) is refused — no row is written, mirroring the
    /// malformed-zone-rules refusal.
    #[cfg(feature = "mission")]
    #[test]
    fn a_malformed_composition_json_writes_no_row() {
        let doc = MissionDocCore::new();
        doc.add_composition("c1", "\"just a string\"");
        assert_eq!(
            doc.composition_count(),
            0,
            "a non-object row must be refused"
        );
        doc.add_composition("c2", "not json at all");
        assert_eq!(doc.composition_count(), 0, "invalid JSON must be refused");
    }

    /* ───────────────────────────── T-665 — editor layer flags ───────────────────────────── */

    /// Build a doc with one layer holding one slot, seeded under INIT so the setup is not on the
    /// undo stack — every T-665 test then perturbs with a single LOCAL flag flip / move.
    fn one_slot_one_layer() -> MissionDocCore {
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_editor_layer("L", "Layer", None);
        doc.add_slot(
            "s0", "sq", "L", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
        );
        doc.set_origin_init(false);
        doc
    }

    /// VISIBILITY, fired once: a slot on a hidden layer is ABSENT from `materialize()` while still
    /// PRESENT in the doc (no data loss) — then un-hiding restores the row. Hide is a view state.
    #[test]
    fn hidden_layer_slot_is_filtered_from_materialize_but_kept_in_the_doc() {
        let doc = one_slot_one_layer();
        assert_eq!(doc.materialize().len(), 1, "visible by default");

        // perturb — hide the layer
        doc.set_editor_layer_hidden("L", true);
        assert_eq!(
            doc.materialize().len(),
            0,
            "hidden layer's slot dropped from the render SoA"
        );
        // …but the slot is still in the doc — no data loss, hide is a VIEW state.
        let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
        assert_eq!(
            slots["s0"]["role"], "Rifleman",
            "slot survives hide: {slots}"
        );
        assert_eq!(slots["s0"]["position"]["x"], 100.0, "position untouched");

        // restore — un-hide brings the row straight back
        doc.set_editor_layer_hidden("L", false);
        assert_eq!(doc.materialize().len(), 1, "un-hide restores the slot");
    }

    /// TRANSFORM LOCK on the drag path (`move_entities`), fired once: a locked layer's slot refuses
    /// the delta (position unchanged), then unlocking lets the same move land.
    #[test]
    fn locked_layer_refuses_move_entities_then_unlock_allows_it() {
        let doc = one_slot_one_layer();

        // perturb — lock, then attempt a drag delta
        doc.set_editor_layer_locked("L", true);
        doc.move_entities(vec!["s0".to_string()], 50.0, 60.0, vec![0.0]);
        let soa = doc.materialize();
        let i = row_of(&soa, "s0");
        assert_eq!(
            (soa.xs[i], soa.ys[i]),
            (100.0, 200.0),
            "locked slot did not move"
        );

        // restore — unlock, the same move now lands
        doc.set_editor_layer_locked("L", false);
        doc.move_entities(vec!["s0".to_string()], 50.0, 60.0, vec![0.0]);
        let soa = doc.materialize();
        let i = row_of(&soa, "s0");
        assert_eq!(
            (soa.xs[i], soa.ys[i]),
            (150.0, 260.0),
            "unlocked slot moved"
        );
    }

    /// TRANSFORM LOCK covers the mixed slot+vehicle drag too (`move_entities_and_vehicles`): the
    /// locked slot is skipped while the vehicle in the SAME drag still moves (the lock is per-slot,
    /// resolved through the slot's layer — vehicles are not layer-filed, so they are unaffected).
    #[test]
    fn locked_layer_refuses_slot_in_mixed_move_but_vehicle_still_moves() {
        let doc = one_slot_one_layer();
        doc.set_origin_init(true);
        doc.add_vehicle(
            "v0",
            "Prefab/Vehicle.et",
            Some(300.0),
            Some(400.0),
            Some(0.0),
            Some(0.0),
        );
        doc.set_origin_init(false);

        doc.set_editor_layer_locked("L", true);
        doc.move_entities_and_vehicles(
            vec!["s0".to_string()],
            &["v0".to_string()],
            10.0,
            20.0,
            vec![0.0],
        );
        let soa = doc.materialize();
        let i = row_of(&soa, "s0");
        assert_eq!(
            (soa.xs[i], soa.ys[i]),
            (100.0, 200.0),
            "locked slot stayed put"
        );
        let vehs = vehicles_of(&doc);
        assert_eq!(vehs["v0"]["position"]["x"], 310.0, "vehicle still moved");
        assert_eq!(vehs["v0"]["position"]["y"], 420.0, "vehicle still moved");
    }

    /// T-082 — `slot_layer_is_locked` is the READ half of the same predicate
    /// `update_slot_position` enforces, so the two must agree at every step. Fired against the
    /// mutator itself rather than against a re-derived expectation: lock, assert BOTH the query and
    /// the refusal; unlock, assert BOTH the query and the acceptance. An unfiled slot is never
    /// locked even while the layer is.
    #[test]
    fn slot_layer_is_locked_agrees_with_the_update_slot_position_refusal() {
        let doc = one_slot_one_layer();
        doc.add_slot(
            "unfiled", "sq", "", 1, "Rifleman", None, None, 10.0, 20.0, 0.0, 0.0,
        );
        assert!(!doc.slot_layer_is_locked("s0"), "unlocked by default");

        doc.set_editor_layer_locked("L", true);
        assert!(doc.slot_layer_is_locked("s0"), "layer locked ⇒ slot locked");
        assert!(
            !doc.slot_layer_is_locked("unfiled"),
            "a slot in no layer is never locked"
        );
        doc.update_slot_position("s0", Some(777.0), Some(888.0), None, None, 12800.0, 12800.0);
        let soa = doc.materialize();
        let i = row_of(&soa, "s0");
        assert_eq!(
            (soa.xs[i], soa.ys[i]),
            (100.0, 200.0),
            "the query said locked and the mutator refused"
        );

        doc.set_editor_layer_locked("L", false);
        assert!(
            !doc.slot_layer_is_locked("s0"),
            "unlock is visible to the query"
        );
        doc.update_slot_position("s0", Some(777.0), Some(888.0), None, None, 12800.0, 12800.0);
        let soa = doc.materialize();
        let i = row_of(&soa, "s0");
        assert_eq!(
            (soa.xs[i], soa.ys[i]),
            (777.0, 888.0),
            "the query said unlocked and the mutator accepted"
        );
    }

    /// T-082 — `update_slot_object` is the ONLY writer for the two Attributes OBJECT fields, and the
    /// whole reason it is not `update_slot_role_character` is the `None` discipline. Fired on both
    /// keys: set, then edit ONE of them with the other `None` and assert the untouched key survived;
    /// `Some("")` clears; and — the multi-edit invariant — a call with both `None` writes nothing
    /// and leaves `role`/`tag` alone.
    #[test]
    fn update_slot_object_sets_clears_and_leaves_none_fields_alone() {
        let doc = one_slot_one_layer();
        doc.update_slot("s0", None, Some("MED".into()), None);

        doc.update_slot_object(
            "s0",
            Some("Character_US_Rifleman".into()),
            Some("Point man. Takes the lead on entry.".into()),
        );
        let row = slots_map(&doc);
        assert_eq!(row["s0"]["assetId"], "Character_US_Rifleman");
        assert_eq!(
            row["s0"]["description"],
            "Point man. Takes the lead on entry."
        );

        // Edit ONE key; `None` must leave the other — and role/tag — exactly as they were.
        doc.update_slot_object("s0", None, Some("Now the breacher.".into()));
        let row = slots_map(&doc);
        assert_eq!(
            row["s0"]["assetId"], "Character_US_Rifleman",
            "a None assetId left the type alone"
        );
        assert_eq!(row["s0"]["description"], "Now the breacher.");
        assert_eq!(row["s0"]["role"], "Rifleman", "role untouched");
        assert_eq!(row["s0"]["tag"], "MED", "tag untouched");

        // `Some("")` clears — absent, not empty-string (the add_slot omit idiom).
        doc.update_slot_object("s0", Some(String::new()), None);
        let row = slots_map(&doc);
        assert!(
            row["s0"].get("assetId").is_none(),
            "empty clears the key rather than storing \"\""
        );
        assert_eq!(row["s0"]["description"], "Now the breacher.");

        // Both None writes nothing at all.
        doc.update_slot_object("s0", None, None);
        let row = slots_map(&doc);
        assert_eq!(row["s0"]["description"], "Now the breacher.");
        assert_eq!(row["s0"]["role"], "Rifleman");
    }

    /// TRANSFORM LOCK on the Attributes-tab path (`update_slot_position`), fired once: a numeric
    /// position edit is refused on a locked layer, then allowed after unlock.
    #[test]
    fn locked_layer_refuses_update_slot_position_then_unlock_allows_it() {
        let doc = one_slot_one_layer();

        doc.set_editor_layer_locked("L", true);
        doc.update_slot_position("s0", Some(777.0), Some(888.0), None, None, 12800.0, 12800.0);
        let soa = doc.materialize();
        let i = row_of(&soa, "s0");
        assert_eq!(
            (soa.xs[i], soa.ys[i]),
            (100.0, 200.0),
            "locked slot refused the Attributes edit"
        );

        doc.set_editor_layer_locked("L", false);
        doc.update_slot_position("s0", Some(777.0), Some(888.0), None, None, 12800.0, 12800.0);
        let soa = doc.materialize();
        let i = row_of(&soa, "s0");
        assert_eq!(
            (soa.xs[i], soa.ys[i]),
            (777.0, 888.0),
            "unlocked slot took the edit"
        );
    }

    /// INHERITANCE, fired once for each flag: a child layer with the flag ABSENT inherits its
    /// parent's hidden AND locked state effectively — without the flag ever being written onto the
    /// child row (resolve-at-read). Un-flagging the parent reveals/unlocks the child again.
    #[test]
    fn child_layer_inherits_parent_hidden_and_locked() {
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_editor_layer("parent", "Parent", None);
        doc.add_editor_layer("child", "Child", Some("parent".to_string()));
        doc.add_slot(
            "s0", "sq", "child", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
        );
        doc.set_origin_init(false);
        assert_eq!(doc.materialize().len(), 1, "visible before any flag");

        // Hide the PARENT — the child's slot disappears though the child row has no `hidden` key.
        doc.set_editor_layer_hidden("parent", true);
        assert_eq!(doc.materialize().len(), 0, "child inherits parent's hidden");
        let layers: serde_json::Value =
            serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json");
        assert!(
            layers["editorLayersById"]["child"].get("hidden").is_none(),
            "flag not copied down onto the child row: {}",
            layers["editorLayersById"]["child"]
        );
        doc.set_editor_layer_hidden("parent", false);
        assert_eq!(doc.materialize().len(), 1, "un-hiding parent reveals child");

        // Lock the PARENT — the child's slot refuses a move with no `locked` key of its own.
        doc.set_editor_layer_locked("parent", true);
        doc.move_entities(vec!["s0".to_string()], 5.0, 5.0, vec![0.0]);
        let soa = doc.materialize();
        let i = row_of(&soa, "s0");
        assert_eq!(
            (soa.xs[i], soa.ys[i]),
            (100.0, 200.0),
            "child inherits parent's lock"
        );
    }

    /// UNDO, fired once for each flag: a LOCAL flag flip is one undo step (undoable like a rename),
    /// and undo restores the prior VIEW state — so a hidden layer's slot reappears after Ctrl+Z.
    #[test]
    fn hidden_and_locked_flag_flips_are_one_undo_step_each() {
        let mut doc = one_slot_one_layer();
        assert_eq!(doc.undo_depth(), 0, "INIT setup is not on the stack");

        // hidden: one flip = one step; undo reverts the flip AND the view state.
        doc.set_editor_layer_hidden("L", true);
        assert_eq!(doc.undo_depth(), 1, "hide is one LOCAL step");
        assert_eq!(doc.materialize().len(), 0, "hidden now");
        assert!(doc.undo());
        assert_eq!(doc.materialize().len(), 1, "undo un-hid the layer");
        assert_eq!(doc.undo_depth(), 0);

        // locked: one flip = one step; undo removes the lock.
        doc.set_editor_layer_locked("L", true);
        assert_eq!(doc.undo_depth(), 1, "lock is one LOCAL step");
        assert!(doc.undo());
        assert_eq!(doc.undo_depth(), 0);
        // After undo the lock is gone, so a move lands again.
        doc.move_entities(vec!["s0".to_string()], 3.0, 4.0, vec![0.0]);
        let soa = doc.materialize();
        let i = row_of(&soa, "s0");
        assert_eq!(
            (soa.xs[i], soa.ys[i]),
            (103.0, 204.0),
            "undo removed the lock"
        );
    }

    /// SHAPE: `false` REMOVES the key rather than storing `false`, so a never-flagged layer's row is
    /// byte-identical to a pre-T-665 doc (absent ⇒ visible/unlocked). Belt for the omit idiom.
    #[test]
    fn clearing_a_layer_flag_removes_the_key() {
        let doc = one_slot_one_layer();
        doc.set_editor_layer_hidden("L", true);
        doc.set_editor_layer_locked("L", true);
        doc.set_editor_layer_hidden("L", false);
        doc.set_editor_layer_locked("L", false);
        let layers: serde_json::Value =
            serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json");
        let row = &layers["editorLayersById"]["L"];
        assert!(row.get("hidden").is_none(), "hidden key removed: {row}");
        assert!(row.get("locked").is_none(), "locked key removed: {row}");
    }

    /* ─────────────────────────── T-701 — per-entity editorHidden flag ─────────────────────────── */

    /// Build a doc with ONE visible layer holding TWO slots on a BLUFOR squad, seeded under INIT so
    /// the setup is not on the undo stack — the T-701 tests then perturb with a single LOCAL flag
    /// flip. The faction/squad give the wire test a mission to compile + flatten; the layer stays
    /// VISIBLE so the per-entity flag is the ONLY thing hiding a slot (isolating it from the T-665
    /// layer path). Slot `s0` is the leader so the flattened ORBAT is well-formed.
    #[cfg(feature = "mission")]
    fn two_slots_visible_layer() -> MissionDocCore {
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_editor_layer("L", "Layer", None);
        doc.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
        doc.add_squad("sq", "faction-BLUFOR", "Alpha", Some("A1".into()));
        doc.add_slot("s0", "sq", "L", 0, "SL", None, None, 100.0, 200.0, 0.0, 0.0);
        doc.add_slot(
            "s1", "sq", "L", 1, "Rifleman", None, None, 110.0, 210.0, 0.0, 0.0,
        );
        doc.set_leader("sq", "s0");
        doc.set_origin_init(false);
        doc
    }

    /// FLAG SHAPE + VISIBILITY, fired once: `editorHidden` rides the slot row ONLY WHEN TRUE, and a
    /// hidden entity is ABSENT from `materialize()` while still PRESENT in the doc (hide is a VIEW,
    /// no data loss) — even though its LAYER is visible. Clearing the flag restores the row and
    /// removes the key (byte-identical to a pre-T-701 row: absent ⇒ visible).
    #[cfg(feature = "mission")]
    #[test]
    fn editor_hidden_rides_the_row_only_when_true_and_filters_materialize() {
        let doc = two_slots_visible_layer();
        assert_eq!(doc.materialize().len(), 2, "both visible by default");
        // Absent ⇒ visible: the key is not written on a never-hidden slot.
        let before: serde_json::Value =
            serde_json::from_str(&doc.slots_json()).expect("slots_json");
        assert!(
            before["s1"].get("editorHidden").is_none(),
            "flag omitted until set: {}",
            before["s1"]
        );

        // Perturb — hide ONE entity on the (still-visible) layer.
        doc.set_slot_editor_hidden("s1", true);
        let soa = doc.materialize();
        assert_eq!(
            soa.ids.len(),
            1,
            "hidden entity dropped from the render SoA"
        );
        assert_eq!(soa.ids[0], "s0", "the un-hidden entity survives");

        // …written only-when-true, and the row is otherwise untouched (VIEW, not delete).
        let after: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
        assert_eq!(
            after["s1"]["editorHidden"], true,
            "flag on the row: {}",
            after["s1"]
        );
        assert_eq!(after["s1"]["role"], "Rifleman", "row survives hide");
        assert_eq!(after["s1"]["position"]["x"], 110.0, "position untouched");

        // Restore — clearing removes the key AND brings the row back.
        doc.set_slot_editor_hidden("s1", false);
        assert_eq!(doc.materialize().len(), 2, "un-hide restores the entity");
        let cleared: serde_json::Value =
            serde_json::from_str(&doc.slots_json()).expect("slots_json");
        assert!(
            cleared["s1"].get("editorHidden").is_none(),
            "false removes the key: {}",
            cleared["s1"]
        );
    }

    /// EFFECTIVE = layer OR entity, fired once across all four corners of the union: a slot is in the
    /// SoA iff its layer is visible AND its own flag is unset. Proves the per-entity check JOINS the
    /// per-layer check at the one filter site (neither masks the other).
    #[cfg(feature = "mission")]
    #[test]
    fn effective_hidden_is_layer_or_entity() {
        let doc = two_slots_visible_layer();

        // (layer visible, entity visible) → present.
        assert_eq!(
            ids_sorted(&doc.materialize()),
            vec!["s0", "s1"],
            "both visible"
        );

        // (layer visible, entity hidden) → s1 dropped by its OWN flag alone.
        doc.set_slot_editor_hidden("s1", true);
        assert_eq!(
            ids_sorted(&doc.materialize()),
            vec!["s0"],
            "entity flag hides s1"
        );

        // (layer hidden, entity hidden) → both gone; the two conditions compose (OR), s0 by layer.
        doc.set_editor_layer_hidden("L", true);
        assert!(
            doc.materialize().ids.is_empty(),
            "layer OR entity hides both"
        );

        // (layer hidden, entity visible) → s1's own flag cleared, but the layer still hides it.
        doc.set_slot_editor_hidden("s1", false);
        assert!(
            doc.materialize().ids.is_empty(),
            "layer alone still hides both even with entity flags clear"
        );

        // Reveal the layer → only the un-flagged slots come back (s0 & s1 both clear now).
        doc.set_editor_layer_hidden("L", false);
        assert_eq!(
            ids_sorted(&doc.materialize()),
            vec!["s0", "s1"],
            "revealing the layer restores the un-flagged entities"
        );
    }

    /// HYDRATE ROUND-TRIP, fired once: `editorHidden` survives compile → hydrate into a fresh doc
    /// verbatim (it is a lossless `editor.slots` field), so a hidden entity reloads hidden and is
    /// filtered out of the reloaded doc's SoA. Belt for the omit idiom on the reload path.
    #[cfg(feature = "mission")]
    #[test]
    fn editor_hidden_survives_hydrate_round_trip() {
        let doc = two_slots_visible_layer();
        doc.set_slot_editor_hidden("s1", true);
        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );

        // Reload into a pristine doc.
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&serde_json::to_string(&payload).expect("payload json"), "L");

        // The flag rode the payload and reloaded onto the row.
        let slots: serde_json::Value =
            serde_json::from_str(&reloaded.slots_json()).expect("slots_json");
        assert_eq!(
            slots["s1"]["editorHidden"], true,
            "flag reloaded: {}",
            slots["s1"]
        );
        assert!(
            slots["s0"].get("editorHidden").is_none(),
            "un-hidden slot has no key after reload: {}",
            slots["s0"]
        );
        // …and the reloaded doc filters it out of the SoA exactly like the source doc.
        assert_eq!(
            ids_sorted(&reloaded.materialize()),
            vec!["s0"],
            "hidden entity stays hidden after reload"
        );
    }

    /// NEVER COMPILES (fired once — perturb / fail / restore): the flag is present in the editor
    /// `editor.slots` block (editor-only state, reloaded verbatim like `slot.tag`) but STRUCTURALLY
    /// absent from the compiled MOD document. Flatten deserializes slots into `SlotIn` (whose fixed
    /// field list omits `editorHidden`) and emits `ModSlot` (which has no such field), so the flag
    /// cannot cross to the wire. FIRING the rule: perturb the compiled payload to smuggle
    /// `editorHidden` onto a slot, re-flatten, and PROVE the MOD bytes still never contain the token —
    /// then restore and confirm the honest compile is clean too.
    #[cfg(feature = "mission")]
    #[test]
    fn editor_hidden_never_reaches_mod_wire() {
        let doc = two_slots_visible_layer();
        doc.set_slot_editor_hidden("s1", true);

        // The EDITOR payload (Save/Export) DOES carry the flag — it is editor-block state, reloaded
        // losslessly. This is the "carries it in the editor block only" half of the contract.
        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let s1 = payload["editor"]["slots"]
            .as_array()
            .expect("editor.slots")
            .iter()
            .find(|s| s["id"] == "s1")
            .expect("s1 in editor.slots");
        assert_eq!(
            s1["editorHidden"], true,
            "editor block keeps the flag: {s1}"
        );

        // The MOD wire (flatten) must NOT: compile the editor payload to the mod document and assert
        // the token is nowhere in the bytes.
        let meta = br#"{"id":"11112222333344445555666677778888","title":"t","author":"a",
            "terrain":"everon","customTerrainName":"","maxPlayers":8,"timeOfDay":"05:30",
            "weatherPreset":"clear"}"#;
        let payload_bytes = serde_json::to_vec(&payload).expect("payload bytes");
        let mod_bytes = crate::mission::flatten::flatten_mod_document_json(meta, &payload_bytes)
            .expect("flatten compiles");
        let mod_text = String::from_utf8(mod_bytes).expect("utf-8");
        assert!(
            !mod_text.contains("editorHidden"),
            "editorHidden leaked onto the MOD wire: {mod_text}"
        );

        // ── FIRE THE RULE — perturb: force `editorHidden` onto EVERY compiled slot, re-flatten, and
        // prove the wire STILL never sees it (the guarantee is structural in `SlotIn`/`ModSlot`, not a
        // property of this one input). A test that only ever feeds clean input can't fail if the
        // guard regresses; this one would.
        let mut perturbed = payload.clone();
        for slot in perturbed["editor"]["slots"]
            .as_array_mut()
            .expect("editor.slots")
        {
            slot["editorHidden"] = serde_json::Value::Bool(true);
        }
        let perturbed_bytes = serde_json::to_vec(&perturbed).expect("perturbed bytes");
        let perturbed_mod =
            crate::mission::flatten::flatten_mod_document_json(meta, &perturbed_bytes)
                .expect("perturbed flatten compiles");
        let perturbed_text = String::from_utf8(perturbed_mod).expect("utf-8");
        assert!(
            !perturbed_text.contains("editorHidden"),
            "perturbed editorHidden must STILL be stripped by SlotIn/ModSlot: {perturbed_text}"
        );

        // ── restore: an honest compile with NO flag anywhere is likewise clean (control — the token
        // is absent because it was never authored, not because a filter removed a present one).
        let clean = two_slots_visible_layer();
        let clean_payload = crate::mission::compile::compile_payload(
            &clean.small_maps_json(),
            &clean.slots_json(),
            false,
        );
        let clean_bytes = serde_json::to_vec(&clean_payload).expect("clean bytes");
        let clean_mod = crate::mission::flatten::flatten_mod_document_json(meta, &clean_bytes)
            .expect("clean flatten compiles");
        assert!(
            !String::from_utf8(clean_mod)
                .expect("utf-8")
                .contains("editorHidden"),
            "control: a doc with no hidden entity has no editorHidden token on the wire"
        );
    }

    /// UNDO, fired once: a single flag flip is ONE undo step (undoable like a rename / the layer eye),
    /// and undo restores the prior VIEW state — a hidden entity reappears in the SoA after Ctrl+Z.
    #[cfg(feature = "mission")]
    #[test]
    fn editor_hidden_flip_is_one_undo_step() {
        let mut doc = two_slots_visible_layer();
        assert_eq!(doc.undo_depth(), 0, "INIT setup is not on the stack");

        doc.set_slot_editor_hidden("s1", true);
        assert_eq!(doc.undo_depth(), 1, "hide is one LOCAL step");
        assert_eq!(doc.materialize().len(), 1, "hidden now");

        assert!(doc.undo());
        assert_eq!(doc.materialize().len(), 2, "undo un-hid the entity");
        assert_eq!(doc.undo_depth(), 0, "one flip = one step");
    }

    /// SHOW-ALL in ONE txn, fired once: hiding several entities (each its own step) then `clear_all_
    /// editor_hidden()` reveals EVERY one in a SINGLE undo step, and one undo re-hides them all —
    /// proving the reveal-all is atomic (one txn), not per-slot. Returns the count un-hidden.
    #[cfg(feature = "mission")]
    #[test]
    fn show_all_clears_every_flag_in_one_txn() {
        let mut doc = two_slots_visible_layer();
        doc.set_slot_editor_hidden("s0", true); // step 1
        doc.set_slot_editor_hidden("s1", true); // step 2
        assert_eq!(doc.undo_depth(), 2, "two individual hides = two steps");
        assert!(doc.materialize().ids.is_empty(), "both hidden");

        // Reveal all — ONE txn, one step, and it reports the two it cleared.
        let cleared = doc.clear_all_editor_hidden();
        assert_eq!(cleared, 2, "both entities un-hidden");
        assert_eq!(doc.undo_depth(), 3, "show-all is exactly ONE more step");
        assert_eq!(doc.materialize().len(), 2, "everything visible again");
        let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
        assert!(slots["s0"].get("editorHidden").is_none(), "s0 key removed");
        assert!(slots["s1"].get("editorHidden").is_none(), "s1 key removed");

        // A SINGLE undo re-hides BOTH — the reveal-all was one atomic transaction.
        assert!(doc.undo());
        assert!(
            doc.materialize().ids.is_empty(),
            "one undo restored the whole reveal-all: both hidden again"
        );
        assert_eq!(doc.undo_depth(), 2, "back to the two individual hides");
    }

    /// BATCH hide, fired once: `set_slots_editor_hidden` flips MANY slots in ONE undo step (the
    /// H-key affordance over a multi-selection is one Eden action). One undo restores all of them —
    /// this is the per-entity one-txn batch the T-732 position lane lacks, present because store.rs
    /// is authored in-slice.
    #[cfg(feature = "mission")]
    #[test]
    fn batch_hide_selection_is_one_undo_step() {
        let mut doc = two_slots_visible_layer();
        doc.set_slots_editor_hidden(&["s0".to_string(), "s1".to_string()], true);
        assert_eq!(
            doc.undo_depth(),
            1,
            "hiding the whole selection is ONE step"
        );
        assert!(doc.materialize().ids.is_empty(), "both hidden by the batch");

        assert!(doc.undo());
        assert_eq!(
            doc.materialize().len(),
            2,
            "one undo un-hid the whole batch"
        );
    }

    /// ISOLATION: an individual field edit (role/tag/stance via `update_slot`) does NOT wipe a
    /// sibling `editorHidden` — the slot mutators write per-key on the tracked YMap, so the flag
    /// (and every other sibling) survives an unrelated edit. Guards against a future whole-row
    /// rewrite regressing the flag off.
    #[cfg(feature = "mission")]
    #[test]
    fn editor_hidden_survives_an_unrelated_slot_edit() {
        let doc = two_slots_visible_layer();
        doc.set_slot_editor_hidden("s1", true);
        // Unrelated edit to the SAME slot.
        doc.update_slot(
            "s1",
            Some("MED".to_string()),
            None,
            Some("prone".to_string()),
        );
        assert_eq!(
            doc.materialize().len(),
            1,
            "s1 still hidden after an unrelated edit"
        );
        let slots: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("slots_json");
        assert_eq!(
            slots["s1"]["editorHidden"], true,
            "flag preserved: {}",
            slots["s1"]
        );
        assert_eq!(
            slots["s1"]["role"], "MED",
            "the unrelated edit still landed"
        );
    }

    // ── T-693 merge_mission_payload ──────────────────────────────────────────────────────────────

    /// A second mission's EDITOR payload, authored in a fresh doc and compiled to the exact JSON the
    /// merge consumes — the "real compile→payload from a second doc" the ticket requires. One BLUFOR
    /// squad Alpha with a leader slot + a follower, and a vehicle crewed by the leader slot.
    #[cfg(feature = "mission")]
    fn template_payload_blufor_alpha() -> serde_json::Value {
        let src = MissionDocCore::new();
        src.set_origin_init(true);
        src.add_editor_layer("lyr", "Layer", None);
        src.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
        src.add_squad("sq-a", "faction-BLUFOR", "Alpha", Some("A1".into()));
        src.add_slot(
            "s0", "sq-a", "lyr", 0, "SL", None, None, 100.0, 200.0, 0.0, 0.0,
        );
        src.add_slot(
            "s1", "sq-a", "lyr", 1, "Rifleman", None, None, 110.0, 210.0, 0.0, 0.0,
        );
        src.set_leader("sq-a", "s0");
        src.add_vehicle(
            "v0",
            "Prefab/Truck.et",
            Some(300.0),
            Some(400.0),
            Some(0.0),
            Some(0.0),
        );
        // Crew: a seat occupied by the leader SLOT id — the re-mint edge the ticket names.
        src.set_vehicle_faction("v0", "faction-BLUFOR");
        src.assign_crew_seat("v0", "driver", "s0");
        src.set_origin_init(false);
        crate::mission::compile::compile_payload(&src.small_maps_json(), &src.slots_json(), false)
    }

    /// T-693.T1 — merge into an EMPTY doc: everything lands, and the graph is internally consistent
    /// under the re-minted ids (squad → its slots, slot → its squad, vehicle crew → the slot).
    #[cfg(feature = "mission")]
    #[test]
    fn merge_into_empty_doc_lands_everything() {
        let payload = template_payload_blufor_alpha();
        let doc = MissionDocCore::new();
        let report = doc.merge_mission_payload(&payload, MergeOpts::default());

        assert_eq!(report.slots_added, 2, "both slots landed");
        assert_eq!(report.factions_created, 1);
        assert_eq!(report.squads_created, 1);
        assert_eq!(report.vehicles_added, 1);
        assert!(
            report.skipped.is_empty(),
            "clean payload: {:?}",
            report.skipped
        );

        let root = small_maps(&doc);
        let squads = root["squadsById"].as_object().expect("squads");
        assert_eq!(squads.len(), 1);
        let (_sq_id, squad) = squads.iter().next().unwrap();
        let slot_ids = squad["slotIds"].as_array().expect("slotIds");
        assert_eq!(slot_ids.len(), 2, "squad owns both merged slots");
        // Every slotIds entry resolves to a real slot whose squadId points back at this squad.
        let slots = slots_map(&doc);
        for sid in slot_ids {
            let sid = sid.as_str().unwrap();
            assert!(
                slots.get(sid).is_some(),
                "slotIds entry {sid} is a real slot"
            );
        }
        // The squad's leader is one of its own (re-minted) slots.
        let leader = squad["leaderSlotId"].as_str().expect("leaderSlotId");
        assert!(
            slot_ids.iter().any(|s| s.as_str() == Some(leader)),
            "leaderSlotId points at a member slot"
        );
    }

    /// T-693.T2 — ORBAT dedup by name+side: merging the SAME template into a doc that already has a
    /// BLUFOR "Alpha" MERGES the slots into the resident squad (no second Alpha), while a squad with a
    /// new name is CREATED.
    #[cfg(feature = "mission")]
    #[test]
    fn merge_dedups_squad_by_name_and_side() {
        // Resident doc: BLUFOR Alpha with one slot.
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_editor_layer("lyr", "Layer", None);
        doc.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
        doc.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
        doc.add_slot(
            "res0", "sq-a", "lyr", 0, "SL", None, None, 1.0, 2.0, 0.0, 0.0,
        );
        doc.set_leader("sq-a", "res0");
        doc.set_origin_init(false);

        // Incoming: BLUFOR Alpha (dedups) + BLUFOR Bravo (created). Both under the same faction name.
        let src = MissionDocCore::new();
        src.set_origin_init(true);
        src.add_editor_layer("lyr", "Layer", None);
        src.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
        src.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
        src.add_squad("sq-b", "faction-BLUFOR", "Bravo", None);
        src.add_slot(
            "s0", "sq-a", "lyr", 0, "Rifleman", None, None, 5.0, 6.0, 0.0, 0.0,
        );
        src.add_slot(
            "s1", "sq-b", "lyr", 0, "Rifleman", None, None, 7.0, 8.0, 0.0, 0.0,
        );
        src.set_leader("sq-a", "s0");
        src.set_leader("sq-b", "s1");
        src.set_origin_init(false);
        let payload = crate::mission::compile::compile_payload(
            &src.small_maps_json(),
            &src.slots_json(),
            false,
        );

        let report = doc.merge_mission_payload(&payload, MergeOpts::default());
        assert_eq!(report.squads_merged, 1, "Alpha deduped onto resident");
        assert_eq!(report.squads_created, 1, "Bravo created");
        assert_eq!(report.factions_merged, 1, "BLUFOR deduped onto resident");
        assert_eq!(report.factions_created, 0);
        assert_eq!(report.slots_added, 2);

        let root = small_maps(&doc);
        let squads = root["squadsById"].as_object().expect("squads");
        assert_eq!(
            squads.len(),
            2,
            "one resident Alpha + one new Bravo, not two Alphas"
        );
        // The resident Alpha now owns two slots (its own + the merged-in one).
        let alpha = &root["squadsById"]["sq-a"];
        assert_eq!(
            alpha["slotIds"].as_array().unwrap().len(),
            2,
            "incoming Alpha slot merged into resident Alpha: {alpha}"
        );
        // Exactly one faction, still resident.
        assert_eq!(root["factionsById"].as_object().unwrap().len(), 1);
    }

    /// T-693.T3 — id re-mint consistency: after a merge into a doc that ALREADY uses the incoming
    /// ids, the trigger `ownerId` and the vehicle crew seat point at the RE-MINTED slot ids, not the
    /// stale originals, and not the resident doc's identically-named rows.
    #[cfg(feature = "mission")]
    #[test]
    fn merge_remints_references_consistently() {
        // Resident doc uses "s0" / "v0" already — the collision the re-mint must survive.
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_editor_layer("lyr", "Layer", None);
        doc.add_faction("faction-OPFOR", "OPFOR", "Resident");
        doc.add_squad("sq", "faction-OPFOR", "Resident Squad", None);
        doc.add_slot("s0", "sq", "lyr", 0, "SL", None, None, 9.0, 9.0, 0.0, 0.0);
        doc.set_leader("sq", "s0");
        doc.add_vehicle("v0", "Prefab/Resident.et", Some(1.0), Some(1.0), None, None);
        doc.set_origin_init(false);

        // Incoming template: crew seat + a trigger both reference the incoming leader slot "s0".
        let src = MissionDocCore::new();
        src.set_origin_init(true);
        src.add_editor_layer("lyr", "Layer", None);
        src.add_faction("faction-BLUFOR", "BLUFOR", "Incoming");
        src.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
        src.add_slot(
            "s0", "sq-a", "lyr", 0, "SL", None, None, 100.0, 100.0, 0.0, 0.0,
        );
        src.set_leader("sq-a", "s0");
        src.add_vehicle("v0", "Prefab/Truck.et", Some(2.0), Some(2.0), None, None);
        src.assign_crew_seat("v0", "driver", "s0");
        src.add_circle_trigger("t0", "presence", 3.0, 3.0, 5.0);
        src.set_trigger_owner("t0", Some("s0"));
        src.set_origin_init(false);
        let payload = crate::mission::compile::compile_payload(
            &src.small_maps_json(),
            &src.slots_json(),
            false,
        );

        let report = doc.merge_mission_payload(&payload, MergeOpts::default());
        assert_eq!(report.slots_added, 1);
        assert_eq!(report.vehicles_added, 1);
        assert_eq!(report.triggers_added, 1);

        let root = small_maps(&doc);
        // The merged slot is NOT "s0" (that id is the resident's); it is a fresh re-minted id.
        let merged_slot_id = root["squadsById"]
            .as_object()
            .unwrap()
            .values()
            .find(|sq| sq["name"] == "Alpha")
            .and_then(|sq| sq["slotIds"][0].as_str())
            .expect("merged Alpha slot")
            .to_string();
        assert_ne!(
            merged_slot_id, "s0",
            "merged slot id must be re-minted, not the resident s0"
        );

        // The merged vehicle's crew driver seat points at the RE-MINTED slot id.
        let merged_vehicle = root["vehiclesById"]
            .as_object()
            .unwrap()
            .values()
            .find(|v| v["resourceName"] == "Prefab/Truck.et")
            .expect("merged truck");
        assert_eq!(
            merged_vehicle["crew"]["driver"].as_str(),
            Some(merged_slot_id.as_str()),
            "crew seat re-minted to the merged slot: {merged_vehicle}"
        );

        // The trigger ownerId points at the same re-minted slot id.
        let trig = root["triggersById"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap();
        assert_eq!(
            trig["ownerId"].as_str(),
            Some(merged_slot_id.as_str()),
            "trigger ownerId re-minted to the merged slot: {trig}"
        );
    }

    /// T-693.T4 — one undo step: the whole merge is a single transaction, so ONE undo restores the
    /// pre-merge document exactly (byte-equal slots digest + content-equal `small_maps_json`).
    #[cfg(feature = "mission")]
    #[test]
    fn merge_is_one_undo_step_and_undo_restores_exactly() {
        let mut doc = seeded_core(); // 8 INIT slots, empty undo stack
        // `small_maps_json`'s top-level map serializes in yrs HashMap order (non-deterministic per
        // call), so compare the PARSED value, which is key-order-independent, not the raw string.
        let before_small = small_maps(&doc);
        let before_digest = slots_digest(&doc.materialize());

        let payload = template_payload_blufor_alpha();
        let report = doc.merge_mission_payload(&payload, MergeOpts::default());
        assert!(report.slots_added > 0, "merge changed the doc");
        assert_eq!(
            doc.undo_depth(),
            1,
            "the whole merge is exactly one undo step"
        );
        assert_ne!(
            slots_digest(&doc.materialize()),
            before_digest,
            "merge added slots"
        );

        assert!(doc.undo(), "undo the merge");
        assert_eq!(
            small_maps(&doc),
            before_small,
            "one undo restores the pre-merge document exactly"
        );
        assert_eq!(
            slots_digest(&doc.materialize()),
            before_digest,
            "undo restores the exact pre-merge slot bits"
        );
        assert!(!doc.can_undo(), "the merge was the only stack item");
    }

    /// T-693.T5 — tolerance: malformed incoming rows are SKIPPED and recorded, never panicked on, and
    /// the well-formed rows around them still land.
    #[test]
    fn merge_records_skipped_malformed_rows() {
        let payload = serde_json::json!({
            "vehicles": [
                { "id": "v-ok", "resourceName": "Prefab/Ok.et", "position": {"x": 1.0, "y": 2.0} },
                { "resourceName": "Prefab/NoId.et" },      // missing id → skipped
                "not-an-object"                               // not an object → skipped
            ],
            "editor": {
                "factions": [],
                "squads": [],
                "slots": [
                    { "id": "s-ok", "role": "Rifleman", "position": {"x": 3.0, "y": 4.0} },
                    { "role": "NoId" }                        // missing id → skipped
                ],
                "editorLayers": []
            }
        });
        let doc = MissionDocCore::new();
        let report = doc.merge_mission_payload(&payload, MergeOpts::default());

        assert_eq!(report.vehicles_added, 1, "the well-formed vehicle landed");
        assert_eq!(report.slots_added, 1, "the well-formed slot landed");
        // Two malformed vehicles + one malformed slot = three skips.
        assert_eq!(report.skipped.len(), 3, "skips: {:?}", report.skipped);
        assert!(
            report
                .skipped
                .iter()
                .any(|(k, _, r)| k == "vehicle" && r.contains("missing id")),
            "missing-id vehicle recorded: {:?}",
            report.skipped
        );
        assert!(
            report
                .skipped
                .iter()
                .any(|(k, _, r)| k == "vehicle" && r.contains("not an object")),
            "non-object vehicle recorded: {:?}",
            report.skipped
        );
        assert!(
            report
                .skipped
                .iter()
                .any(|(k, _, r)| k == "slot" && r.contains("missing id")),
            "missing-id slot recorded: {:?}",
            report.skipped
        );
    }

    /// T-693.T5b — the JSON wrapper reports a parse failure in `skipped` rather than erroring.
    #[test]
    fn merge_json_wrapper_reports_parse_failure() {
        let doc = MissionDocCore::new();
        let out = doc.merge_mission_payload_json("{not valid json", None);
        let v: serde_json::Value = serde_json::from_str(&out).expect("wrapper returns JSON");
        assert_eq!(v["slots_added"], 0);
        assert!(
            v["skipped"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["reason"].as_str().unwrap().contains("invalid JSON")),
            "parse failure recorded: {v}"
        );
    }

    /// T-693.T6 — offset opt: `Some((dx, dy))` shifts every merged entity by the world delta; the
    /// default keeps authored coordinates. Slots, vehicles and a zone circle all move.
    #[cfg(feature = "mission")]
    #[test]
    fn merge_offset_shifts_all_placed_entities() {
        let src = MissionDocCore::new();
        src.set_origin_init(true);
        src.add_editor_layer("lyr", "Layer", None);
        src.add_faction("faction-BLUFOR", "BLUFOR", "B");
        src.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
        src.add_slot(
            "s0", "sq-a", "lyr", 0, "SL", None, None, 100.0, 200.0, 0.0, 0.0,
        );
        src.set_leader("sq-a", "s0");
        src.add_vehicle(
            "v0",
            "Prefab/Truck.et",
            Some(300.0),
            Some(400.0),
            None,
            None,
        );
        src.add_circle_zone("z0", "boundary", 500.0, 600.0, 50.0);
        src.set_origin_init(false);
        let payload = crate::mission::compile::compile_payload(
            &src.small_maps_json(),
            &src.slots_json(),
            false,
        );

        let doc = MissionDocCore::new();
        let report = doc.merge_mission_payload(
            &payload,
            MergeOpts {
                offset: Some((1000.0, 2000.0)),
            },
        );
        assert_eq!(report.slots_added, 1);
        assert_eq!(report.vehicles_added, 1);
        assert_eq!(report.zones_added, 1);

        // Compare by numeric value (`as_f64`), not JSON literal: an integral offset result serializes
        // as `1100` (BigInt path) vs the `1100.0` a `json!` literal would carry, both == 1100.0.
        let slots = slots_map(&doc);
        let slot = slots.as_object().unwrap().values().next().unwrap();
        assert_eq!(
            slot["position"]["x"].as_f64(),
            Some(1100.0),
            "slot x offset"
        );
        assert_eq!(
            slot["position"]["y"].as_f64(),
            Some(2200.0),
            "slot y offset"
        );

        let root = small_maps(&doc);
        let veh = root["vehiclesById"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap();
        assert_eq!(
            veh["position"]["x"].as_f64(),
            Some(1300.0),
            "vehicle x offset"
        );
        assert_eq!(
            veh["position"]["y"].as_f64(),
            Some(2400.0),
            "vehicle y offset"
        );

        let zone = root["zonesById"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap();
        assert_eq!(
            zone["shape"]["circle"]["x"].as_f64(),
            Some(1500.0),
            "zone x offset"
        );
        assert_eq!(
            zone["shape"]["circle"]["z"].as_f64(),
            Some(2600.0),
            "zone z offset"
        );
    }

    /// T-693.T7 — a full compile→merge→compile round trip proves the merged payload is itself
    /// re-emittable and reloadable (the ORBAT-from-template end state a Save must survive).
    #[cfg(feature = "mission")]
    #[test]
    fn merged_doc_round_trips_through_compile_and_hydrate() {
        let payload = template_payload_blufor_alpha();
        let doc = MissionDocCore::new();
        let _ = doc.merge_mission_payload(&payload, MergeOpts::default());
        let reloaded = save_and_reload(&doc);
        // The reloaded doc has the same shape: one squad with two slots, one vehicle.
        let root = small_maps(&reloaded);
        assert_eq!(root["squadsById"].as_object().unwrap().len(), 1);
        let squad = root["squadsById"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap();
        assert_eq!(squad["slotIds"].as_array().unwrap().len(), 2);
        assert_eq!(root["vehiclesById"].as_object().unwrap().len(), 1);
    }

    /// Every entry of a `slotIds`/`squadIds`/`entityIds`-style id array, in order.
    fn id_array(v: &serde_json::Value) -> Vec<String> {
        v.as_array()
            .expect("id array")
            .iter()
            .map(|s| s.as_str().expect("string id").to_string())
            .collect()
    }

    /// True iff `ids` holds no id twice (the dedup-append invariant).
    fn has_no_duplicates(ids: &[String]) -> bool {
        let mut seen = HashSet::new();
        ids.iter().all(|id| seen.insert(id.clone()))
    }

    /// T-693.T8 (BLOCKER-1 + MINOR-4) — **merging the SAME template twice into a doc with a matching
    /// resident squad lands the second merge's rows ALONGSIDE the first's** (no silent overwrite), the
    /// resident squad's `slotIds` has no duplicates, and the report counts are true.
    ///
    /// This is the ticket's NEW-F4 primary scenario and the exact case the pre-fix per-call `seq`
    /// corrupted: merge 1 and merge 2 make identical dedup decisions, so `<old>` reached `ensure_fresh`
    /// at the same `<seq>` both times and minted the SAME `mrg-<seq>-<old>` — the second
    /// `MapRef::insert` overwrote merge 1's slot row (its count net-zero) while `append_id`
    /// double-appended. The fix seeds the re-mint's collision guard with the doc's whole id universe
    /// (including merge 1's resident `mrg-…` ids), so merge 2 mints fresh ids and both merges' rows
    /// coexist. Perturbing [`RemintMap::ensure_fresh`] to ignore `taken` (the pre-fix minting) fails
    /// this test at the row-count and slotIds-length assertions — see
    /// [`mint_is_collision_proof_against_resident_ids`] for the mechanism fired in isolation.
    #[cfg(feature = "mission")]
    #[test]
    fn merge_same_template_twice_lands_alongside_no_overwrite() {
        // Resident doc: BLUFOR Alpha with one slot — the "resident matching squad" the template dedups
        // onto, so every incoming slot MERGES into Alpha (the seq-alignment case, not a create case).
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_editor_layer("lyr", "Layer", None);
        doc.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
        doc.add_squad("sq-a", "faction-BLUFOR", "Alpha", Some("A1".into()));
        doc.add_slot(
            "res0", "sq-a", "lyr", 0, "SL", None, None, 1.0, 2.0, 0.0, 0.0,
        );
        doc.set_leader("sq-a", "res0");
        doc.set_origin_init(false);

        // The template compiles to two slots (s0, s1) under BLUFOR Alpha + a vehicle. Merging it once
        // grows Alpha to 3 slots; merging it AGAIN must grow it to 5 — not leave it at 3.
        let payload = template_payload_blufor_alpha();

        let slot_rows = |d: &MissionDocCore| slots_map(d).as_object().unwrap().len();

        let rep1 = doc.merge_mission_payload(&payload, MergeOpts::default());
        assert_eq!(rep1.slots_added, 2, "merge 1 adds both template slots");
        assert_eq!(rep1.squads_merged, 1, "Alpha deduped onto resident");
        assert_eq!(
            slot_rows(&doc),
            3,
            "merge 1: resident res0 + two merged slots"
        );

        let rep2 = doc.merge_mission_payload(&payload, MergeOpts::default());
        assert_eq!(rep2.slots_added, 2, "merge 2 adds two MORE slots");
        assert_eq!(rep2.squads_merged, 1);
        // The load-bearing assertion: the doc now holds FIVE distinct slot rows. Pre-fix, merge 2's
        // ids equalled merge 1's, `insert` overwrote, and this stayed at 3 while the report claimed +2.
        assert_eq!(
            slot_rows(&doc),
            5,
            "merge 2's rows land ALONGSIDE merge 1's — no overwrite"
        );

        let root = small_maps(&doc);
        let alpha = &root["squadsById"]["sq-a"];
        let member_ids = id_array(&alpha["slotIds"]);
        assert_eq!(
            member_ids.len(),
            5,
            "Alpha owns res0 + 4 merged slots: {member_ids:?}"
        );
        assert!(
            has_no_duplicates(&member_ids),
            "slotIds has no duplicate id after two merges: {member_ids:?}"
        );
        // Every membership id resolves to a REAL, distinct slot row (the overwrite would leave a
        // dangling id whose row another append clobbered).
        let slots = slots_map(&doc);
        for sid in &member_ids {
            assert!(
                slots.get(sid).is_some(),
                "slotIds entry {sid} is a live slot row: {member_ids:?}"
            );
        }
        // Report truth: the two merges added exactly 4 slots total, and the doc grew by exactly 4.
        assert_eq!(
            rep1.slots_added + rep2.slots_added,
            4,
            "reports sum to the real net row growth (5 − 1 resident)"
        );
    }

    /// T-693.T9 (MINOR-4 second half) — **two incoming slot rows that share one id do not
    /// double-append.** The re-mint reserves one fresh id for the shared `old` (a duplicate id is one
    /// id), both rows resolve to it, and the dedup-append in [`append_id`] files it into the squad's
    /// `slotIds` exactly once — no `[..., id, id]` over-count. (Only one slot row lands under that id;
    /// the report counts each incoming row it wrote, and the membership array stays a set.)
    #[cfg(feature = "mission")]
    #[test]
    fn merge_duplicate_in_payload_ids_do_not_double_append() {
        // A resident squad the two dup-id slots dedup onto, so they append into a real `slotIds`.
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_editor_layer("lyr", "Layer", None);
        doc.add_faction("faction-BLUFOR", "BLUFOR", "1st Battalion");
        doc.add_squad("sq-a", "faction-BLUFOR", "Alpha", None);
        doc.set_origin_init(false);

        // Raw payload: one BLUFOR Alpha squad (dedups) with TWO slot rows sharing id "dup".
        let payload = serde_json::json!({
            "editor": {
                "factions": [
                    { "id": "faction-BLUFOR", "key": "BLUFOR", "name": "1st Battalion", "squadIds": ["sq-a"] }
                ],
                "squads": [
                    { "id": "sq-a", "factionId": "faction-BLUFOR", "name": "Alpha", "slotIds": ["dup"] }
                ],
                "slots": [
                    { "id": "dup", "squadId": "sq-a", "role": "SL", "position": {"x": 1.0, "y": 1.0} },
                    { "id": "dup", "squadId": "sq-a", "role": "Rifleman", "position": {"x": 2.0, "y": 2.0} }
                ],
                "editorLayers": []
            }
        });

        let _ = doc.merge_mission_payload(&payload, MergeOpts::default());

        let root = small_maps(&doc);
        let alpha = &root["squadsById"]["sq-a"];
        let member_ids = id_array(&alpha["slotIds"]);
        assert!(
            has_no_duplicates(&member_ids),
            "a duplicate incoming id is filed once, not twice: {member_ids:?}"
        );
        // The shared id maps to exactly one re-minted membership entry (one id, one append).
        assert_eq!(
            member_ids.len(),
            1,
            "two rows sharing one id contribute one membership id: {member_ids:?}"
        );
    }

    /// T-693.T10 (fired proof) — the minting mechanism in isolation: **the pre-fix per-call `seq`
    /// minted colliding ids; the collision guard prevents it.** Two independent re-mint tables with an
    /// EMPTY resident universe (the pre-fix state — `RemintMap::new`) BOTH mint `mrg-1-s0` for the same
    /// `old`: identical output, the exact overwrite-cause of BLOCKER-1. Seeding the second table with
    /// the first's mint (what `with_reserved` does with the doc's id universe) forces it past the
    /// collision to a fresh id.
    #[test]
    fn mint_is_collision_proof_against_resident_ids() {
        // Pre-fix: two separate merges, each a fresh table over an empty doc, mint the SAME id.
        let mut first = RemintMap::new();
        first.ensure_fresh("s0");
        let merge1_id = first.get("s0").expect("minted");
        assert_eq!(merge1_id, "mrg-1-s0", "first merge mints mrg-1-s0");

        let mut naive_second = RemintMap::new();
        naive_second.ensure_fresh("s0");
        assert_eq!(
            naive_second.get("s0").as_deref(),
            Some("mrg-1-s0"),
            "an unseeded second table reproduces the SAME id — the collision the fix removes"
        );

        // With the fix: the second table is seeded with the resident id universe (here, merge 1's id),
        // so `ensure_fresh` bumps past `mrg-1-s0` to a fresh id.
        let mut guarded_second = RemintMap::with_reserved(HashSet::from([merge1_id.clone()]));
        guarded_second.ensure_fresh("s0");
        let merge2_id = guarded_second.get("s0").expect("minted");
        assert_ne!(
            merge2_id, merge1_id,
            "the guarded second mint avoids the resident id"
        );
        assert_eq!(merge2_id, "mrg-2-s0", "it takes the next free seq");

        // And it also avoids an arbitrary resident `mrg-…` id at a higher seq (multi-merge chains).
        let mut deep = RemintMap::with_reserved(HashSet::from([
            "mrg-1-x".to_string(),
            "mrg-2-x".to_string(),
            "mrg-3-x".to_string(),
        ]));
        deep.ensure_fresh("x");
        assert_eq!(
            deep.get("x").as_deref(),
            Some("mrg-4-x"),
            "mint skips every resident collision, not just seq 1"
        );
    }

    /* ═══════════════════════ T-651 — editor comments / annotations ═══════════════════════ */

    /// A doc with a complete editor graph (flatten needs one — a bare slot fails on `NoSlots`) plus
    /// one comment carrying a token that could not occur by accident.
    #[cfg(feature = "mission")]
    fn doc_with_one_comment() -> (MissionDocCore, &'static str) {
        const TOKEN: &str = "CMT-TOKEN-ZZQ";
        let doc = two_slots_visible_layer();
        doc.add_comment(
            "c1",
            TOKEN,
            "tooltip body for CMT-TOKEN-ZZQ",
            1_234.5,
            6_789.5,
        );
        (doc, TOKEN)
    }

    /// **THE LOAD-BEARING RULE, FIRED.** A comment rides the EDITOR payload (or it would not survive
    /// a Save) and is STRUCTURALLY ABSENT from the compiled MOD document, because
    /// `mission::flatten::EditorPayload` declares no `comments` key and serde drops what it does not
    /// declare.
    ///
    /// The rule is an ABSENCE, so a test that only feeds clean input proves nothing: the token would
    /// be missing from the mod bytes even if the search were broken. This one therefore FIRES the
    /// rule — it re-routes the same comment through `entities[]`, a root flatten DOES read, and
    /// asserts the token then APPEARS in the mod bytes. That is what makes the absence assertion
    /// above a real assertion rather than a tautology, and it is the exact failure a future edit
    /// would cause by "helpfully" storing comments in a compiled collection.
    #[cfg(feature = "mission")]
    #[test]
    fn comments_never_reach_the_mod_document() {
        let (doc, token) = doc_with_one_comment();

        // 1. The EDITOR payload carries it — `commentsById` (canonical) and the `comments[]` root the
        //    `payloadExtras` promotion puts there. Without this half the feature does not persist.
        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let comments = payload["comments"]
            .as_array()
            .expect("comments[] at the editor-payload root");
        assert_eq!(comments.len(), 1, "one authored comment: {payload}");
        assert_eq!(comments[0]["title"], token);
        assert_eq!(comments[0]["position"]["x"], 1_234.5);
        assert_eq!(comments[0]["position"]["z"], 6_789.5);

        // 2. …and it round-trips: a pristine doc hydrated from that payload has the comment back.
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&serde_json::to_string(&payload).expect("payload json"), "L");
        let rows: serde_json::Value =
            serde_json::from_str(&reloaded.comments_json()).expect("comments_json");
        assert_eq!(rows["c1"]["title"], token, "comment reloaded: {rows}");
        assert_eq!(reloaded.comment_count(), 1);

        // 3. The MOD document must not contain it — anywhere, in any field.
        let meta = br#"{"id":"11112222333344445555666677778888","title":"t","author":"a",
            "terrain":"everon","customTerrainName":"","maxPlayers":8,"timeOfDay":"05:30",
            "weatherPreset":"clear"}"#;
        let payload_bytes = serde_json::to_vec(&payload).expect("payload bytes");
        let mod_text = String::from_utf8(
            crate::mission::flatten::flatten_mod_document_json(meta, &payload_bytes)
                .expect("flatten compiles"),
        )
        .expect("utf-8");
        assert!(
            !mod_text.contains(token) && !mod_text.contains("comment"),
            "a comment reached the compiled mission: {mod_text}"
        );

        // 4. ── FIRE THE RULE. Same comment, routed instead through `entities[]` — a root
        //    `EditorPayload` DOES declare. If this does not turn the mod bytes dirty then step 3's
        //    assertion cannot detect a leak and the whole guard is decorative.
        let mut leaked = payload.clone();
        leaked["entities"] = serde_json::json!([{
            "id": "c1",
            "alias": token,
            "resourceName": "",
            "position": { "x": 1_234.5, "z": 6_789.5 },
            "faction": "",
        }]);
        let leaked_text = String::from_utf8(
            crate::mission::flatten::flatten_mod_document_json(
                meta,
                &serde_json::to_vec(&leaked).expect("leaked bytes"),
            )
            .expect("leaked flatten compiles"),
        )
        .expect("utf-8");
        assert!(
            leaked_text.contains(token),
            "the leak probe did not reach the mod document, so step 3 proves nothing: {leaked_text}"
        );

        // 5. …and restore: with the comment back in its own root ONLY, the wire is clean again.
        assert!(!mod_text.contains(token));
    }

    /// The three ATTR-FIELD-CMT-* fields are authored, editable and survive a hydrate — including an
    /// edit made AFTER the hydrate, when the row is an opaque `Any::Map` and not a tracked `YMap`
    /// (the whole point of `read_comment_map`).
    #[cfg(feature = "mission")]
    #[test]
    fn comment_title_tooltip_position_edit_before_and_after_hydrate() {
        let doc = two_slots_visible_layer();
        doc.add_comment("c1", "t0", "b0", 10.0, 20.0);
        doc.set_comment_title("c1", "t1");
        doc.set_comment_tooltip("c1", "b1");
        doc.set_comment_position("c1", 30.0, 40.0);
        let rows: serde_json::Value =
            serde_json::from_str(&doc.comments_json()).expect("comments_json");
        assert_eq!(rows["c1"]["title"], "t1");
        assert_eq!(rows["c1"]["tooltip"], "b1");
        assert_eq!(rows["c1"]["position"]["x"], 30.0);
        assert_eq!(rows["c1"]["position"]["z"], 40.0);

        // Reload, then edit the POST-HYDRATE row — the shape a naive tracked-YMap write would drop.
        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&serde_json::to_string(&payload).expect("json"), "L");
        reloaded.set_comment_title("c1", "t2");
        reloaded.set_comment_position("c1", 50.0, 60.0);
        let after: serde_json::Value =
            serde_json::from_str(&reloaded.comments_json()).expect("comments_json");
        assert_eq!(after["c1"]["title"], "t2");
        assert_eq!(
            after["c1"]["tooltip"], "b1",
            "the untouched field survives a whole-row rewrite: {after}"
        );
        assert_eq!(after["c1"]["position"]["x"], 50.0);
        assert_eq!(after["c1"]["position"]["z"], 60.0);

        // Unknown ids are no-ops, not panics or ghost rows.
        reloaded.set_comment_title("nope", "x");
        assert_eq!(reloaded.comment_count(), 1);
    }

    /// COPY: `duplicate_comment` clones title + tooltip and offsets the position, in ONE undo step;
    /// an unknown source writes nothing. Also pins the `Any::BigInt` hazard — a hydrated
    /// integer-valued coordinate must offset from its real value, not from a silently-zeroed one.
    #[cfg(feature = "mission")]
    #[test]
    fn duplicate_comment_copies_fields_and_offsets_even_after_a_hydrate() {
        let doc = two_slots_visible_layer();
        // Integer-valued coords on purpose: `json_str_to_any` re-encodes these as `Any::BigInt`.
        doc.add_comment("c1", "title", "body", 6_400.0, 6_400.0);
        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&serde_json::to_string(&payload).expect("json"), "L");

        assert!(reloaded.duplicate_comment("c1", "c2", 25.0, -25.0));
        let rows: serde_json::Value =
            serde_json::from_str(&reloaded.comments_json()).expect("comments_json");
        assert_eq!(rows["c2"]["title"], "title");
        assert_eq!(rows["c2"]["tooltip"], "body");
        assert_eq!(
            rows["c2"]["position"]["x"], 6_425.0,
            "a BigInt-encoded coordinate must offset from 6400, not from 0: {rows}"
        );
        assert_eq!(rows["c2"]["position"]["z"], 6_375.0);

        assert!(
            !reloaded.duplicate_comment("ghost", "c3", 0.0, 0.0),
            "an unknown source id writes nothing"
        );
        assert_eq!(reloaded.comment_count(), 2);
    }

    /// LAYERS + one-undo-step: filing a comment uses the SAME `entityIds` mechanism a slot does, a
    /// delete unfiles it (no dangling id), and each gesture is exactly one undo step.
    #[cfg(feature = "mission")]
    #[test]
    fn comments_file_into_layers_and_each_gesture_is_one_undo_step() {
        let mut doc = two_slots_visible_layer();
        doc.add_editor_layer("L2", "Notes", None);
        let depth0 = doc.undo_depth();

        doc.add_comment("c1", "t", "b", 1.0, 2.0);
        assert_eq!(doc.undo_depth(), depth0 + 1, "a place is one undo step");
        doc.move_comment_to_layer("c1", "L2");
        assert_eq!(doc.undo_depth(), depth0 + 2, "a refile is one undo step");

        let layers: serde_json::Value =
            serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json");
        let ents = layers["editorLayersById"]["L2"]["entityIds"]
            .as_array()
            .expect("entityIds");
        assert!(
            ents.iter().any(|v| v == "c1"),
            "the comment files exactly like a slot: {ents:?}"
        );

        // A delete removes the row AND unfiles it — no dangling id left in the folder.
        doc.remove_comment("c1");
        assert_eq!(doc.comment_count(), 0);
        let after: serde_json::Value =
            serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json");
        let ents_after = after["editorLayersById"]["L2"]["entityIds"]
            .as_array()
            .expect("entityIds");
        assert!(
            !ents_after.iter().any(|v| v == "c1"),
            "a deleted comment must not survive as a dangling folder id: {ents_after:?}"
        );

        // …and it is undoable: editor-only is not untracked.
        assert!(doc.undo());
        assert_eq!(doc.comment_count(), 1, "Ctrl+Z brings the annotation back");
        // Wave-135 H1: undo must restore L2 filing, not only the commentsById row.
        let restored: serde_json::Value =
            serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json");
        let ents_restored = restored["editorLayersById"]["L2"]["entityIds"]
            .as_array()
            .expect("entityIds after undo");
        assert!(
            ents_restored.iter().any(|v| v == "c1"),
            "Ctrl+Z must put c1 back in L2 entityIds, not leave a orphaned comment row: {ents_restored:?}"
        );
    }

    /// THE NEW-MISSION TEMPLATE: an empty doc seeds exactly TWO comments (what survived FNF v4's
    /// rewrite — one community across two eras, not a four-way convergence), and the seed declines
    /// on any doc that already has one, so it can never duplicate itself on a reload.
    #[cfg(feature = "mission")]
    #[test]
    fn new_mission_template_seeds_exactly_two_comments_and_is_idempotent() {
        let doc = MissionDocCore::new();
        doc.set_origin_init(true);
        let ids = doc.seed_template_comments();
        doc.set_origin_init(false);
        assert_eq!(ids.len(), 2, "the surviving FNF v4 onboarding is two notes");
        assert_eq!(doc.comment_count(), 2);
        let rows: serde_json::Value =
            serde_json::from_str(&doc.comments_json()).expect("comments_json");
        for id in &ids {
            assert!(
                rows[id.as_str()]["title"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty()),
                "a seeded note has a title: {rows}"
            );
            assert!(
                rows[id.as_str()]["tooltip"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty()),
                "a seeded note has a body: {rows}"
            );
        }
        // Seeded under INIT ⇒ not an undo step (a template is not a user gesture).
        assert_eq!(
            doc.undo_depth(),
            0,
            "the template is never on the undo stack"
        );

        // Idempotent: a doc that already carries comments is not a new mission.
        assert!(doc.seed_template_comments().is_empty());
        assert_eq!(doc.comment_count(), 2);
    }

    /// The seeded template survives compile → hydrate (so a new mission that is saved untouched
    /// reopens with its notes) and still reaches no mod document.
    #[cfg(feature = "mission")]
    #[test]
    fn seeded_template_comments_round_trip_and_stay_off_the_mod_wire() {
        let doc = two_slots_visible_layer();
        doc.set_origin_init(true);
        doc.seed_template_comments();
        doc.set_origin_init(false);

        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&serde_json::to_string(&payload).expect("json"), "L");
        assert_eq!(reloaded.comment_count(), 2, "template survives a save/load");

        let meta = br#"{"id":"11112222333344445555666677778888","title":"t","author":"a",
            "terrain":"everon","customTerrainName":"","maxPlayers":8,"timeOfDay":"05:30",
            "weatherPreset":"clear"}"#;
        let mod_text = String::from_utf8(
            crate::mission::flatten::flatten_mod_document_json(
                meta,
                &serde_json::to_vec(&payload).expect("bytes"),
            )
            .expect("flatten compiles"),
        )
        .expect("utf-8");
        assert!(
            !mod_text.contains("Start here") && !mod_text.contains("Mission notes"),
            "the template's own notes must not compile either: {mod_text}"
        );
    }

    /// Deleting every comment CLEARS the wire rather than re-emitting the array a reload parked —
    /// the zones absence rule, which is the failure mode a "non-empty only" projection would have.
    #[cfg(feature = "mission")]
    #[test]
    fn deleting_every_comment_clears_the_payload_array() {
        let doc = two_slots_visible_layer();
        doc.add_comment("c1", "t", "b", 1.0, 2.0);
        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&serde_json::to_string(&payload).expect("json"), "L");
        assert_eq!(reloaded.comment_count(), 1);

        reloaded.remove_comment("c1");
        let after = crate::mission::compile::compile_payload(
            &reloaded.small_maps_json(),
            &reloaded.slots_json(),
            false,
        );
        assert!(
            after
                .get("comments")
                .is_none_or(|c| c.as_array().is_some_and(Vec::is_empty)),
            "a deleted comment must not be re-emitted from the parked copy: {after}"
        );
    }

    /* ══════════════════ T-672 — the connection graph: SEE, CHECK, then WRITE ═════════════════ */

    /// A doc with two slots (`s0` leads `sq`, `s1` is its member) plus a vehicle and an object, so
    /// every endpoint class the `CONN-DANGLING` universe covers is present and an edge can be drawn
    /// between things that are not both slots.
    #[cfg(feature = "mission")]
    fn doc_with_connectable_things() -> MissionDocCore {
        let doc = two_slots_visible_layer();
        doc.set_origin_init(true);
        doc.add_vehicle(
            "v0",
            "truck",
            Some(300.0),
            Some(400.0),
            Some(0.0),
            Some(0.0),
        );
        doc.add_entity("e0", "Crate", "crate_res", 500.0, 600.0, 0.0, 0.0);
        doc.set_origin_init(false);
        doc
    }

    /// **SEE — the listing is stable, total and addressable.** A connection has no map glyph in this
    /// slice, so this array IS the operator's only view of the graph; if its order were a function of
    /// `serde_json`'s map iteration the rows would dance between reads and "delete the third one"
    /// would be a lie. Asserted by reading it repeatedly and byte-comparing, and by drawing the same
    /// three edges into a SECOND document in a DIFFERENT order and getting the identical array —
    /// order is a function of content, not of authoring history.
    #[cfg(feature = "mission")]
    #[test]
    fn the_connection_listing_is_stable_addressable_and_content_ordered() {
        let a = doc_with_connectable_things();
        assert!(a.add_connection("k1", "sync", "s1", "v0"));
        assert!(a.add_connection("k2", "group", "s1", "s0"));
        assert!(a.add_connection("k3", "triggerOwner", "e0", "v0"));

        let first = a.connection_rows_json();
        for _ in 0..8 {
            assert_eq!(
                a.connection_rows_json(),
                first,
                "listing must be byte-stable"
            );
        }
        let rows: serde_json::Value = serde_json::from_str(&first).expect("rows json");
        let rows = rows.as_array().expect("array");
        assert_eq!(rows.len(), 3, "every edge is listed: {first}");
        // Every row is addressable by the id the delete verb takes.
        for r in rows {
            let id = r["id"].as_str().expect("id");
            assert!(["k1", "k2", "k3"].contains(&id), "unknown row id {id}");
            assert!(!r["from"].as_str().expect("from").is_empty());
            assert!(!r["to"].as_str().expect("to").is_empty());
        }

        // Same graph, authored in the reverse order, listed identically.
        let b = doc_with_connectable_things();
        assert!(b.add_connection("k3", "triggerOwner", "e0", "v0"));
        assert!(b.add_connection("k2", "group", "s1", "s0"));
        assert!(b.add_connection("k1", "sync", "s1", "v0"));
        assert_eq!(
            b.connection_rows_json(),
            first,
            "listing order must be a function of CONTENT, not of authoring order"
        );
    }

    /// **CHECK — every rule FIRES on a graph built to break it, and a clean graph is silent.**
    /// Pure, over `validate_connection_rows`, because the point of the FNF v4 warning is that these
    /// defects hide: a checker only reachable through a live document is a checker nobody runs.
    ///
    /// The clean half is not decoration — it is what proves the four positives are the RULES firing
    /// and not the checker shouting at everything.
    #[test]
    fn the_connection_checker_fires_every_rule_and_stays_silent_on_a_clean_graph() {
        let known: HashSet<String> = ["a", "b", "c"].iter().map(|s| (*s).to_string()).collect();
        let row = |id: &str, kind: &str, from: &str, to: &str| ConnectionRow {
            id: id.to_string(),
            kind: kind.to_string(),
            from: from.to_string(),
            to: to.to_string(),
        };

        // CLEAN: a sync pair, an ownership chain a→b→c. No findings at all.
        let clean = vec![
            row("ok1", "sync", "a", "b"),
            row("ok2", "group", "a", "b"),
            row("ok3", "group", "b", "c"),
        ];
        assert_eq!(
            validate_connection_rows(&clean, &known),
            Vec::new(),
            "a correct graph must produce NO findings, or the positives below prove nothing"
        );

        let codes = |rows: &[ConnectionRow]| -> Vec<(&'static str, String)> {
            validate_connection_rows(rows, &known)
                .into_iter()
                .map(|f| (f.code, f.connection_id))
                .collect()
        };

        // CONN-SELF.
        assert!(
            codes(&[row("x", "sync", "a", "a")]).contains(&("CONN-SELF", "x".to_string())),
            "a self-link must fire CONN-SELF"
        );
        // CONN-DANGLING — an endpoint that is not a placed entity.
        assert!(
            codes(&[row("x", "sync", "a", "ghost")]).contains(&("CONN-DANGLING", "x".to_string())),
            "an unplaced endpoint must fire CONN-DANGLING"
        );
        // CONN-DUPLICATE — the SECOND row of a repeated triple, so the survivor is deterministic.
        let dupes = codes(&[
            row("first", "sync", "a", "b"),
            row("second", "sync", "a", "b"),
        ]);
        assert!(
            dupes.contains(&("CONN-DUPLICATE", "second".to_string()))
                && !dupes.contains(&("CONN-DUPLICATE", "first".to_string())),
            "duplicate must name the LATER row, not the survivor: {dupes:?}"
        );
        // CONN-CYCLE — a→b→c→a in the directed subgraph.
        let cyc = codes(&[
            row("e1", "group", "a", "b"),
            row("e2", "group", "b", "c"),
            row("e3", "group", "c", "a"),
        ]);
        assert!(
            cyc.iter().any(|(code, _)| *code == "CONN-CYCLE"),
            "an ownership cycle must fire CONN-CYCLE: {cyc:?}"
        );
        // …and the SAME three edges as `sync` are legal — sync is undirected, so a closed loop of
        // peers is a connected component, not a defect. This is the rule's discrimination.
        let sync_loop = codes(&[
            row("e1", "sync", "a", "b"),
            row("e2", "sync", "b", "c"),
            row("e3", "sync", "a", "c"),
        ]);
        assert!(
            !sync_loop.iter().any(|(code, _)| *code == "CONN-CYCLE"),
            "a sync loop is legal — CONN-CYCLE is for DIRECTED kinds only: {sync_loop:?}"
        );
        // CONN-KIND — a vocabulary a hydrate can carry in but `add_connection` would refuse.
        assert!(
            codes(&[row("x", "attachedTo", "a", "b")]).contains(&("CONN-KIND", "x".to_string())),
            "an unknown kind must fire CONN-KIND"
        );

        // Findings are stably ordered for the same reason the rows are.
        let messy = vec![
            row("z", "sync", "a", "a"),
            row("y", "group", "a", "ghost"),
            row("x", "sync", "ghost2", "b"),
        ];
        let once = validate_connection_rows(&messy, &known);
        for _ in 0..8 {
            assert_eq!(validate_connection_rows(&messy, &known), once);
        }
    }

    /// **CHECK, through the live document.** The pure checker above is only useful if the doc-side
    /// adapter feeds it the real graph and the real endpoint universe — so this drives the same
    /// rules through `connection_findings_json` on a hydrated doc, which is the ONLY path a
    /// self-link / duplicate / bad kind can actually reach the store (the mutator refuses all three,
    /// the generic row loader does not).
    #[cfg(feature = "mission")]
    #[test]
    fn hydrated_junk_edges_survive_the_load_and_are_reported_not_silently_dropped() {
        let doc = doc_with_connectable_things();
        assert!(doc.add_connection("good", "sync", "s0", "s1"));
        let mut payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        // Junk this editor cannot author, arriving the way a foreign payload would.
        payload["connections"] = serde_json::json!([
            {"id": "good", "kind": "sync", "from": "s0", "to": "s1"},
            {"id": "selfie", "kind": "sync", "from": "s0", "to": "s0"},
            {"id": "ghosted", "kind": "group", "from": "s1", "to": "nope"},
            // Named to sort AFTER `good` in the `(kind, from, to, id)` listing order, because
            // CONN-DUPLICATE names the later row and `good` is the survivor.
            {"id": "zz-dupe", "kind": "sync", "from": "s0", "to": "s1"},
            {"id": "weird", "kind": "attachedTo", "from": "s0", "to": "s1"},
        ]);

        let reloaded = doc_with_connectable_things();
        reloaded.hydrate(&serde_json::to_string(&payload).expect("json"), "L");
        assert_eq!(
            reloaded.connection_count(),
            5,
            "a bad row must SURVIVE the load — rejecting at hydrate destroys an author's data"
        );
        let findings: serde_json::Value =
            serde_json::from_str(&reloaded.connection_findings_json()).expect("findings");
        let pairs: Vec<(String, String)> = findings
            .as_array()
            .expect("array")
            .iter()
            .map(|f| {
                (
                    f["code"].as_str().unwrap_or_default().to_string(),
                    f["connectionId"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        for want in [
            ("CONN-SELF", "selfie"),
            ("CONN-DANGLING", "ghosted"),
            ("CONN-DUPLICATE", "zz-dupe"),
            ("CONN-KIND", "weird"),
        ] {
            assert!(
                pairs.contains(&(want.0.to_string(), want.1.to_string())),
                "{want:?} missing from the live findings: {pairs:?}"
            );
        }
        assert!(
            !pairs.iter().any(|(_, id)| id == "good"),
            "the sound edge must not be flagged: {pairs:?}"
        );
    }

    /// **WRITE — every refusal, fired.** `add_connection` returns false AND writes nothing for the
    /// empty id, the unknown kind, the self-link and the duplicate; and `sync` normalises its
    /// endpoints so a REVERSED re-draw is caught as the duplicate it is (the case a verbatim store
    /// would let through as a second edge, which is the FNF v4 shape exactly).
    #[cfg(feature = "mission")]
    #[test]
    fn add_connection_refuses_junk_and_normalises_sync_endpoints() {
        let doc = doc_with_connectable_things();
        for (id, kind, from, to, why) in [
            ("", "sync", "s0", "s1", "empty id"),
            ("k", "sync", "", "s1", "empty from"),
            ("k", "sync", "s0", "", "empty to"),
            ("k", "attachedTo", "s0", "s1", "unknown kind"),
            ("k", "sync", "s0", "s0", "self-link"),
        ] {
            assert!(
                !doc.add_connection(id, kind, from, to),
                "must refuse: {why}"
            );
            assert_eq!(doc.connection_count(), 0, "…and write nothing: {why}");
        }

        assert!(doc.add_connection("k1", "sync", "s1", "s0"));
        let rows: serde_json::Value =
            serde_json::from_str(&doc.connection_rows_json()).expect("rows");
        assert_eq!(
            (rows[0]["from"].as_str(), rows[0]["to"].as_str()),
            (Some("s0"), Some("s1")),
            "sync endpoints are sorted at write: {rows}"
        );
        // The reversed re-draw, under a fresh id, is the same edge.
        assert!(
            !doc.add_connection("k2", "sync", "s0", "s1"),
            "sync(A,B) after sync(B,A) is a DUPLICATE, not a second edge"
        );
        assert_eq!(doc.connection_count(), 1);
        // A DIRECTED kind keeps its direction: group(A,B) and group(B,A) are two real relations.
        assert!(doc.add_connection("g1", "group", "s1", "s0"));
        assert!(doc.add_connection("g2", "group", "s0", "s1"));
        assert_eq!(
            doc.connection_count(),
            3,
            "directed edges are not normalised"
        );
    }

    /// **CONN-DEL-001 + the cascade.** Deleting one edge is one Ctrl+Z; deleting an ENTITY takes all
    /// of its edges with it in ONE undo step, so a Ctrl+Z restores the unit and its connections
    /// together. A cascade split across transactions would be a half-applied undo — the unit back,
    /// its relations gone — which is exactly the class of defect the ticket's warning names.
    #[cfg(feature = "mission")]
    #[test]
    fn deleting_a_connection_and_cascading_an_entity_are_each_one_undo_step() {
        let mut doc = doc_with_connectable_things();
        assert!(doc.add_connection("k1", "sync", "s0", "s1"));
        assert!(doc.add_connection("k2", "group", "s1", "v0"));
        assert!(doc.add_connection("k3", "triggerOwner", "e0", "v0"));
        let before = doc.undo_depth();

        doc.remove_connection("k1");
        assert_eq!(doc.connection_count(), 2);
        assert_eq!(doc.undo_depth(), before + 1, "one delete ⇒ one undo step");
        doc.undo();
        assert_eq!(doc.connection_count(), 3, "Ctrl+Z brings the edge back");

        let depth = doc.undo_depth();
        let cascaded = doc.remove_connections_touching("v0");
        assert_eq!(cascaded.len(), 2, "both edges touching v0: {cascaded:?}");
        assert_eq!(doc.connection_count(), 1);
        assert_eq!(
            doc.undo_depth(),
            depth + 1,
            "the whole cascade is ONE transaction, not one per edge"
        );
        doc.undo();
        assert_eq!(
            doc.connection_count(),
            3,
            "one Ctrl+Z restores every cascaded edge"
        );

        assert!(
            doc.remove_connections_touching("not-a-thing").is_empty(),
            "an unknown id removes nothing"
        );
    }

    /// **The rule that keeps a drawn edge alive: it rides the EDITOR payload and is STRUCTURALLY
    /// ABSENT from the compiled MOD document** — `mission::flatten::EditorPayload` declares no
    /// `connections` key, and the schema declares no relation collection for one to land in.
    ///
    /// The rule is an ABSENCE, so clean input proves nothing: the token would be missing from the mod
    /// bytes even if the search were broken. This FIRES it — the same endpoint id is re-routed
    /// through `entities[]`, a root flatten DOES read, and the token then APPEARS. That is what makes
    /// step 3 an assertion instead of a tautology, and it is the exact failure a future edit would
    /// cause by "helpfully" storing relations in a compiled collection.
    #[cfg(feature = "mission")]
    #[test]
    fn connections_never_reach_the_mod_document() {
        // Two slots only — no vehicle / object, so the flatten under test is exercised on a document
        // whose ONLY unusual content is the connection graph. (`doc_with_connectable_things` seeds a
        // placeholder vehicle whose `resourceName` T-425's kit-alias guard rightly refuses to
        // compile; that guard is not what this test is about.)
        let doc = two_slots_visible_layer();
        assert!(doc.add_connection("conn-1", "triggerOwner", "s1", "s0"));

        // 1. The EDITOR payload carries it — without this half the feature does not persist.
        let payload = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        let conns = payload["connections"]
            .as_array()
            .expect("connections[] at the editor-payload root");
        assert_eq!(conns.len(), 1, "one drawn edge: {payload}");
        assert_eq!(conns[0]["kind"], "triggerOwner");
        assert_eq!(conns[0]["from"], "s1", "a DIRECTED kind stores verbatim");
        assert_eq!(conns[0]["to"], "s0");

        // 2. …and it round-trips: a pristine doc hydrated from that payload has the edge back.
        let reloaded = MissionDocCore::new();
        reloaded.hydrate(&serde_json::to_string(&payload).expect("payload json"), "L");
        assert_eq!(reloaded.connection_count(), 1);
        let rows: serde_json::Value =
            serde_json::from_str(&reloaded.connection_rows_json()).expect("rows");
        assert_eq!(rows[0]["id"], "conn-1", "edge reloaded: {rows}");

        // 3. The MOD document must not contain the relation — anywhere, in any field.
        let meta = br#"{"id":"11112222333344445555666677778888","title":"t","author":"a",
            "terrain":"everon","customTerrainName":"","maxPlayers":8,"timeOfDay":"05:30",
            "weatherPreset":"clear"}"#;
        let payload_bytes = serde_json::to_vec(&payload).expect("payload bytes");
        let mod_text = String::from_utf8(
            crate::mission::flatten::flatten_mod_document_json(meta, &payload_bytes)
                .expect("flatten compiles"),
        )
        .expect("utf-8");
        assert!(
            !mod_text.contains("triggerOwner") && !mod_text.contains("connections"),
            "a connection reached the compiled mission: {mod_text}"
        );

        // 4. ── FIRE THE RULE. The same token routed instead through `entities[]`, a root
        //    `EditorPayload` DOES declare. If this does not turn the mod bytes dirty, step 3's
        //    assertion cannot detect a leak and the whole guard is decorative.
        let mut leaked = payload.clone();
        leaked["entities"] = serde_json::json!([
            {
                "id": "leak1",
                "alias": "triggerOwner",
                "position": { "x": 1.0, "z": 2.0 },
                "faction": "",
            },
            {
                "id": "leak2",
                "alias": "connections",
                "position": { "x": 3.0, "z": 4.0 },
                "faction": "",
            },
        ]);
        let leaked_text = String::from_utf8(
            crate::mission::flatten::flatten_mod_document_json(
                meta,
                &serde_json::to_vec(&leaked).expect("leaked bytes"),
            )
            .expect("leaked flatten compiles"),
        )
        .expect("utf-8");
        assert!(
            leaked_text.contains("triggerOwner") && leaked_text.contains("connections"),
            "the leak probe did not reach the mod document, so step 3 proves nothing: {leaked_text}"
        );

        // 5. Deleting the last edge CLEARS the wire — absence must be expressible, or a
        //    `CONN-DEL-001` delete silently un-deletes itself on the next load.
        doc.remove_connection("conn-1");
        let after = crate::mission::compile::compile_payload(
            &doc.small_maps_json(),
            &doc.slots_json(),
            false,
        );
        assert!(
            after
                .get("connections")
                .is_none_or(|c| c.as_array().is_some_and(Vec::is_empty)),
            "a deleted connection must not be re-emitted from the parked copy: {after}"
        );
    }

    /// **ACTION-FORM-001 geometry, pure.** The nine schema tokens produce nine DISTINCT shapes (a
    /// name that silently aliased another would be a formation the operator cannot actually pick),
    /// every offset is exactly `FORMATION_SPACING_M`-quantised, and an unknown token falls back to
    /// `column` rather than to nothing.
    #[test]
    fn formation_offsets_are_distinct_per_schema_token_and_fall_back_to_column() {
        const TOKENS: [&str; 9] = [
            "column",
            "stagger_column",
            "wedge",
            "echelon_left",
            "echelon_right",
            "vee",
            "line",
            "file",
            "diamond",
        ];
        let shapes: Vec<Vec<(f64, f64)>> = TOKENS.iter().map(|t| formation_offsets(t, 6)).collect();
        for (i, a) in shapes.iter().enumerate() {
            assert_eq!(a.len(), 6, "{}: one offset per member", TOKENS[i]);
            for (j, b) in shapes.iter().enumerate().skip(i + 1) {
                assert_ne!(
                    a, b,
                    "`{}` and `{}` are the same shape",
                    TOKENS[i], TOKENS[j]
                );
            }
        }
        // The leader is the anchor: nobody is placed on top of him.
        for (i, shape) in shapes.iter().enumerate() {
            assert!(
                !shape.contains(&(0.0, 0.0)),
                "`{}` puts a member on the leader",
                TOKENS[i]
            );
        }
        assert_eq!(
            formation_offsets("no_such_formation", 3),
            formation_offsets("column", 3),
            "an unknown token falls back to column"
        );
        assert_eq!(
            formation_offsets("column", 0),
            Vec::new(),
            "no members, no offsets"
        );
        // Column is a straight trail one spacing apart, which is the property the fallback relies on.
        assert_eq!(
            formation_offsets("column", 3),
            vec![
                (0.0, -FORMATION_SPACING_M),
                (0.0, -2.0 * FORMATION_SPACING_M),
                (0.0, -3.0 * FORMATION_SPACING_M),
            ]
        );
        // T-767 / wave 131 F3 — schema `$defs/group.formation` description must name the
        // editor consumers. Split so this line cannot satisfy the schema haystack.
        let schema = include_str!("../../../../packages/tbd-schema/schema/mission.schema.json");
        let force = format!("{}{}", "force_to_", "formation");
        let offsets = format!("{}{}", "formation_", "offsets");
        assert!(
            schema.contains(&force),
            "mission.schema.json formation description must mention force_to_formation"
        );
        assert!(
            schema.contains(&offsets),
            "mission.schema.json formation description must mention formation_offsets"
        );
    }

    /// **ACTION-FORM-001 through the document.** The leader does NOT move (he is the anchor — moving
    /// him would translate the squad and make the action a reposition nobody asked for); every member
    /// lands on its offset ROTATED by the leader's heading; and the whole re-form is ONE undo step so
    /// an operator who dislikes the result gets every unit back in one press.
    #[cfg(feature = "mission")]
    #[test]
    fn force_to_formation_anchors_the_leader_rotates_by_heading_and_is_one_undo_step() {
        let mut doc = two_slots_visible_layer();
        doc.set_origin_init(true);
        doc.add_slot(
            "s2", "sq", "L", 2, "Rifleman", None, None, 999.0, 999.0, 0.0, 0.0,
        );
        // Leader faces 90° — the formation's forward axis must follow him.
        doc.set_slot_position("s0", 1_000.0, 2_000.0, 0.0, 90.0);
        doc.set_origin_init(false);

        let depth = doc.undo_depth();
        assert_eq!(
            doc.force_to_formation("s0", "column"),
            2,
            "both members moved"
        );
        assert_eq!(
            doc.undo_depth(),
            depth + 1,
            "a re-form is ONE transaction, not one per member"
        );

        // Re-read the document on every call — a snapshot parsed once would make the post-undo
        // assertion below read the PRE-undo positions and pass no matter what undo did.
        let at = |d: &MissionDocCore, id: &str| {
            let slots: serde_json::Value =
                serde_json::from_str(&d.slots_json()).expect("slots_json");
            (
                slots[id]["position"]["x"].as_f64().expect("x"),
                slots[id]["position"]["y"].as_f64().expect("y"),
            )
        };
        assert_eq!(
            at(&doc, "s0"),
            (1_000.0, 2_000.0),
            "the LEADER is the anchor and does not move"
        );
        // Body-frame column offsets are (0, -10) and (0, -20); at heading 90° "behind" is −x.
        let (x1, y1) = at(&doc, "s1");
        assert!(
            (x1 - (1_000.0 - FORMATION_SPACING_M)).abs() < 1e-9 && (y1 - 2_000.0).abs() < 1e-9,
            "member 1 must trail along the leader's heading, got ({x1}, {y1})"
        );
        let (x2, y2) = at(&doc, "s2");
        assert!(
            (x2 - (1_000.0 - 2.0 * FORMATION_SPACING_M)).abs() < 1e-9
                && (y2 - 2_000.0).abs() < 1e-9,
            "member 2 must trail twice as far, got ({x2}, {y2})"
        );

        assert!(doc.undo(), "the re-form is on the undo stack");
        assert_eq!(
            at(&doc, "s2"),
            (999.0, 999.0),
            "one Ctrl+Z restores every re-formed unit"
        );

        // A non-leader is not a leader: firing the action off a rifleman must do NOTHING rather than
        // silently re-form the squad around him (that would be a leadership change nobody asked for).
        assert_eq!(doc.force_to_formation("s1", "wedge"), 0);
        assert_eq!(doc.force_to_formation("", "wedge"), 0);
        assert_eq!(doc.force_to_formation("not-a-slot", "wedge"), 0);
    }
}
