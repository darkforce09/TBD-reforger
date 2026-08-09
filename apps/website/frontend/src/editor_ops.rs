//! T-159.22 — the dock commands: outliner select / active layer, and palette drag-to-place.
//!
//! Peer of `mission_history` / `mission_commands`, and the same shape for the same reason: the doc /
//! engine / selection handles are `!Send` wasm-only `Rc`s that can't cross the
//! `#[cfg(target_arch = "wasm32")]` boundary into the native view shell, so the dock buttons reach
//! them through a `thread_local` [`OpsCtx`] set from `mission_editor::on_load` — exactly how the
//! Undo button reaches the undo stack.
//!
//! **Placement (T-180.1):** each `place_at` calls
//! [`map_engine_core::doc::place_character_under_side`] under [`OpsCtx::active_side`] (default
//! `BLUFOR`), which ensures `faction-{SIDE}`, mints a **new** squad, adds the slot as sole member /
//! leader, and files it under the resolved layer ([`ensure_layer`]). Layer mint stays LOCAL so it is
//! **undoable** — a boot-time layer would break the save/export gate (`smoke_save_export_editor`
//! uses the seed only). The ORBAT tree derives from squads (`build_orbat`). Seed slots still carry a
//! dangling `squadId` with no squad in the map — they list under Unfiled until placed-through.
//!
//! Consequence: the **first** place is multiple undo steps (layer + faction + squad + slot + leader
//! are separate core transactions); every later place under an existing layer/faction is fewer.
//!
//! **Borrow discipline** (the `mission_history` rule): each `pub fn` opens exactly one `OPS_CTX`
//! borrow; doc `borrow_mut`s are scoped so they drop before `mission_history::after_local_edit`
//! opens its read borrows.
#![cfg(target_arch = "wasm32")]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use leptos::prelude::{GetUntracked, RwSignal, Set};
use map_engine_core::doc::place_character_under_side;
use map_engine_core::doc::{
    apply_faction_library, EntityTransformPatch, FactionLibraryInput, FactionLibraryRole,
    FactionLibraryVehicle, MissionDocCore, APPLY_ANCHOR_X, APPLY_ANCHOR_Y, NONE_IDX,
};

use crate::asset_catalog::PlacePayload;
use crate::dto::{FactionDoc, FactionRole, FactionVehicle};
use crate::mission_doc::DocHandle;
use crate::outliner::{build_outliner_with_comments, CommentRow, LayerRow, OutlinerNode, SlotRow};
use crate::select_tool::{EngineHandle, SelectionHandle};

/// The lazily-minted default layer (React's `ensureDefaultLayer`).
const DEFAULT_LAYER_ID: &str = "layer-1";
const DEFAULT_LAYER_NAME: &str = "Layer 1";

struct OpsCtx {
    doc: DocHandle,
    engine: EngineHandle,
    selection: SelectionHandle,
    /// The drop target folder (React's `activeLayerId`). `None` ⇒ the place path resolves one.
    active_layer: RwSignal<Option<String>>,
    /// T-180.1 — active Eden side for place (`BLUFOR`/`OPFOR`/`INDFOR`). Chips write this in T-180.5.
    active_side: RwSignal<String>,
    /// T-180.5 / T-254 — Objects chip: when true, the right dock shows the Objects palette and
    /// [`begin_place_object`] / [`place_at`] mint `entitiesById` rows.
    objects_mode: RwSignal<bool>,
    /// Dock mirrors — `MissionDocCore` has no change subscription, so these are pushed from
    /// [`refresh_docks`] at every mutation site, like the OBJ/SEL readouts.
    outliner_nodes: RwSignal<Vec<OutlinerNode>>,
    /// T-168 — the ORBAT dock tree mirror (faction/squad/slot), rebuilt alongside `outliner_nodes`.
    orbat_nodes: RwSignal<Vec<OutlinerNode>>,
    selected_ids: RwSignal<Vec<String>>,
    /// T-159.26 — the Attributes modal's open slot id (`None` = closed). The dbl-click pick and the
    /// outliner activate set it; the modal component reads it reactively.
    attrs_open: RwSignal<Option<String>>,
    /// T-180.9 — Attributes tab index (`TABS[3] == "Arsenal"`). Lifted so [`open_arsenal`] can
    /// select the Arsenal tab; [`open_attributes`] leaves it alone.
    attrs_tab: RwSignal<usize>,
    /// T-159.26 — reactive doc-change tick (the modal's re-read trigger; `doc_ver` is non-reactive).
    doc_tick: RwSignal<u64>,
    /// The in-flight palette drag: `Some` between a leaf `pointerdown` and the canvas `pointerup`.
    pending: RefCell<Option<Pending>>,
    /// Monotonic minter for placed-slot ids; [`mint_id`] still proves uniqueness against the doc.
    next_id: Cell<u32>,
}

/// T-215 — which palette armed the in-flight place. The two tabs hand the map the same
/// [`PlacePayload`] but write **different entities**: a Factions leaf becomes a `slots` row through
/// `place_character_under_side`, a Vehicles leaf becomes a `vehiclesById` row through `add_vehicle`.
///
/// The discriminant lives here, on the armed value, rather than on a separate "current tab" signal:
/// the tab can change (or the dock can unmount) between the leaf's `pointerdown` and the canvas's
/// `pointerup`, and a place must commit the entity the operator actually picked up.
#[derive(Clone, Debug, PartialEq)]
enum Pending {
    Character(PlacePayload),
    Vehicle(PlacePayload),
    /// T-254 — Objects chip → `entitiesById` row.
    Object(PlacePayload),
    /// T-650 — a saved composition armed for placement (COMP-PLACE-001). Carries the composition id;
    /// the canvas release resolves the row and stamps every captured entity at the drop point as ONE
    /// undo step. Rides the same one-shot arm lifecycle as the three palette payloads above — its arm
    /// mirrors [`begin_place_object`]'s shape (a plain `arm(…)` from a palette press) — so the map's
    /// `has_pending` ghost, the release-over-chrome cancel, and Ctrl multi-place all work unchanged.
    Composition(String),
    /// T-069 (RIGHT-MODE-006) — a map marker armed from the Markers palette, carrying the
    /// `$defs/marker.icon` alias the icon row was pressed for. One-shot like the three payloads
    /// above; the canvas release writes `factionsById[faction-{SIDE}].briefing.markers[]` at the
    /// drop point. It carries the icon rather than a `PlacePayload` because a marker is not a
    /// `/registry` catalog leaf — there is no asset to resolve, only a closed vocabulary alias.
    Marker(String),
    /// T-582 — an in-progress zone draw. Unlike the three above this is **multi-click**: it is not
    /// consumed by the first canvas release, so [`place_at`] branches on it before its `take`.
    Zone(ZoneDraft),
}

/// T-582 — the in-progress zone draw.
///
/// Lives on `ctx.pending` (rather than in a signal of its own) for the same reason the palette arms
/// do: `has_pending()` is what makes `mission_editor`'s pointer handlers route a canvas release to
/// [`place_at`] instead of the select/marquee machine, and re-deriving "is a draw in flight" from a
/// second source is how the two get out of step.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneDraft {
    /// Schema `zone.type`, taken from `$defs/zone/properties/type/enum` by the dock — never typed.
    pub kind: String,
    pub shape: ZoneShape,
    /// Circle: the centre, set by the first click. `None` until then.
    pub centre: Option<(f64, f64)>,
    /// Polygon: the ring so far, one vertex per click.
    pub verts: Vec<(f64, f64)>,
    /// T-582 — RESHAPE target. `None` creates a new row (`add_*`); `Some(id)` re-shapes that
    /// existing one (`set_*_circle` / `set_*_polygon`).
    ///
    /// The two reshape mutators replace the WHOLE `shape` object, which is why a circle can become a
    /// polygon and back: `$defs/shape` is a `oneOf`, so a row carrying both keys is schema-INVALID,
    /// and a partial edit would leave exactly that. Re-shaping through them keeps every other
    /// authored field — label, faction, and the opaque `rules` — untouched, which is the whole point
    /// of offering reshape instead of delete-and-redraw.
    pub target: Option<String>,
    /// T-079 — WHICH collection this draw commits into ([`DrawTarget`]). This is the "trigger area is
    /// a SECOND CONSUMER of the zone draw tool" parameter: the whole draw state machine
    /// ([`advance_zone_draw`] / [`close_zone_polygon`]) is shared, and only the final commit branches
    /// on this to call the zone mutators vs the trigger mutators. Zones set `DrawTarget::Zone`,
    /// triggers `DrawTarget::Trigger`; a reshape (`target.is_some()`) carries the collection of the
    /// row it re-shapes.
    pub collection: DrawTarget,
}

/// One slot's editable attributes for the Attributes modal.
///
/// T-082 — read from TWO sources, not one, and the split is the whole of this ticket. `x`/`y`/`z`/
/// `rotation`/`stance`/`role`/`tag`/`squad` come from the materialized SoA, which is the render
/// projection and carries only the columns the GPU needs. `asset_id` and `description` are NOT in
/// it — `SlotSoa` has no such column and never will — so they come from the raw slot row
/// (`slots_json`). Before this ticket `read_attrs` read the SoA alone, which is why the entity TYPE
/// was unreadable in the modal even though the core could already write it: the field was missing
/// from the READ path, not from the mutator.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotAttrs {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub rotation: f64,
    pub stance: String,
    pub role: String,
    pub tag: String,
    pub squad: String,
    /// T-082 ATTR-FIELD-OBJ-TYPE — the slot's `assetId` (the entity type it spawns as). Empty when
    /// unset, which is the common case: a slot with no `assetId` compiles to its faction's default
    /// kit alias.
    pub asset_id: String,
    /// T-082 ATTR-FIELD-OBJ-ROLE-DESC — Eden's free-text "Role Description". A field of its OWN:
    /// `role` is the short label ("Rifleman") the ORBAT and the compiled document use, and having it
    /// double as the prose description is precisely the gap this ticket closes.
    pub description: String,
}

thread_local! {
    static OPS_CTX: RefCell<Option<OpsCtx>> = const { RefCell::new(None) };
    /// T-076 (RIGHT-CREW-001) — the "place vehicle with crew" toggle state. A pure placement
    /// PREFERENCE (not doc state), read once at [`place_at`] time and stamped onto the placed
    /// vehicle as its manned/unmanned intent. Kept here as a thread-local `Cell` rather than on
    /// `OpsCtx` because it is editor-session UI state with a single writer (the dock switch) and no
    /// consumer outside this module — the same reason `next_id` is a `Cell`. Defaults to `true`:
    /// Eden places vehicles crewed unless the operator turns the switch off (or holds Alt).
    static PLACE_WITH_CREW: Cell<bool> = const { Cell::new(true) };
}

/// T-076 (RIGHT-CREW-001) — set the "place vehicle with crew" toggle (the DockRight switch beside
/// the Vehicles search). Reads back through [`place_with_crew`]; the next vehicle placement stamps
/// this intent onto the row.
pub fn set_place_with_crew(with_crew: bool) {
    PLACE_WITH_CREW.with(|f| f.set(with_crew));
}

/// T-076 (RIGHT-CREW-001) — current "place vehicle with crew" toggle state (the switch's `checked`).
#[must_use]
pub fn place_with_crew() -> bool {
    PLACE_WITH_CREW.with(Cell::get)
}

/* ─────────────────────────── T-647 PLACE-003 — the empty-ground asset picker ─────────────────── */

// The picker's state struct lives in `mission_editor` (always-compiled) because the picker COMPONENT
// and its signal live there too — `editor_ops` is `#![cfg(wasm32)]`, so a native test build cannot
// see a type defined here. This module only WRITES the signal (from the wasm dblclick path), so it
// names the type through `crate::mission_editor::AssetPickerState`.
use crate::mission_editor::AssetPickerState;

thread_local! {
    /// T-647 PLACE-003 — the picker signal, installed once from `mission_editor::on_load` (the same
    /// pattern as [`crate::context_menu::set_menu_signal`] and the Attributes `attrs_open`). Kept as
    /// a standalone registered signal rather than a 14th `set_ctx` argument: it is a self-contained
    /// overlay owned by the page, read by the picker component and written only here.
    static ASSET_PICKER: RefCell<Option<RwSignal<Option<AssetPickerState>>>> =
        const { RefCell::new(None) };
}

/// T-647 PLACE-003 — register the picker signal (called once from `mission_editor::on_load`).
pub fn set_asset_picker_signal(sig: RwSignal<Option<AssetPickerState>>) {
    ASSET_PICKER.with(|s| *s.borrow_mut() = Some(sig));
}

/// T-647 PLACE-003 — open the empty-ground asset picker at a world point / screen pixel. No-op if the
/// signal was never registered (a native shell, or before `on_load`).
pub fn open_asset_picker(wx: f64, wy: f64, screen_x: f64, screen_y: f64) {
    ASSET_PICKER.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(Some(AssetPickerState {
                wx,
                wy,
                screen_x,
                screen_y,
            }));
        }
    });
}

/// T-647 PLACE-003 — close the picker (an asset was chosen, or the operator dismissed it).
pub fn close_asset_picker() {
    ASSET_PICKER.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(None);
        }
    });
}

/* ─────────────────── T-651 — the comment editor's open-id signal (same idiom) ─────────────────── */

thread_local! {
    /// T-651 — the comment editor overlay's open comment id (`None` = closed), installed once from
    /// `mission_editor::on_load`. A standalone registered signal rather than another `set_ctx`
    /// argument, exactly like [`ASSET_PICKER`]: a self-contained overlay owned by the page, read by
    /// its component and written only here.
    static COMMENT_EDITOR: RefCell<Option<RwSignal<Option<String>>>> = const { RefCell::new(None) };
}

/// T-651 — register the comment-editor signal (called once from `mission_editor::on_load`).
pub fn set_comment_editor_signal(sig: RwSignal<Option<String>>) {
    COMMENT_EDITOR.with(|s| *s.borrow_mut() = Some(sig));
}

/// T-651 — open the comment editor on `id`. This is a comment's Attributes, and it is a SEPARATE
/// surface on purpose: [`open_attributes`] reads the slot SoA, and a comment is not in it, so
/// routing a comment id there would open a modal with every field blank — the T-716 live-but-inert
/// path. No-op if the signal was never registered (native shell, or before `on_load`).
pub fn open_comment_editor(id: String) {
    COMMENT_EDITOR.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(Some(id));
        }
    });
}

/// T-651 — close the comment editor.
pub fn close_comment_editor() {
    COMMENT_EDITOR.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(None);
        }
    });
}

/// T-651 — one comment's editable fields for the editor overlay, or `None` when the id is gone
/// (deleted, or undone away while the panel was open — the overlay then closes itself rather than
/// editing a ghost).
#[must_use]
pub fn read_comment(id: &str) -> Option<CommentDetail> {
    comment_list().into_iter().find(|c| c.id == id)
}

/// Install the ops context (once, from `on_load`, after the doc is seeded).
#[allow(clippy::too_many_arguments)]
pub fn set_ctx(
    doc: DocHandle,
    engine: EngineHandle,
    selection: SelectionHandle,
    active_layer: RwSignal<Option<String>>,
    active_side: RwSignal<String>,
    objects_mode: RwSignal<bool>,
    outliner_nodes: RwSignal<Vec<OutlinerNode>>,
    orbat_nodes: RwSignal<Vec<OutlinerNode>>,
    selected_ids: RwSignal<Vec<String>>,
    attrs_open: RwSignal<Option<String>>,
    attrs_tab: RwSignal<usize>,
    doc_tick: RwSignal<u64>,
) {
    OPS_CTX.with(|c| {
        *c.borrow_mut() = Some(OpsCtx {
            doc,
            engine,
            selection,
            active_layer,
            active_side,
            objects_mode,
            outliner_nodes,
            orbat_nodes,
            selected_ids,
            attrs_open,
            attrs_tab,
            doc_tick,
            pending: RefCell::new(None),
            next_id: Cell::new(0),
        });
    });
}

/* ───────────────────────── Mission Settings (T-159.26 — environment half) ───────────────────────── */

/// The doc's terrain + environment fields — relocated to the always-compiled [`crate::dto`] so the
/// native `eden_chrome` view shell can build a default; re-exported here for wasm callers.
pub use crate::dto::MissionEnv;

/// Read terrain + environment from the doc meta (`small_maps_json` → `meta`).
pub fn read_env() -> MissionEnv {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            let root: serde_json::Value = serde_json::from_str(&core.small_maps_json()).ok()?;
            let meta = root.get("meta")?;
            let env = meta.get("environment");
            let s = |v: Option<&serde_json::Value>, k: &str, def: &str| {
                v.and_then(|e| e.get(k))
                    .and_then(|x| x.as_str())
                    .unwrap_or(def)
                    .to_string()
            };
            Some(MissionEnv {
                terrain: meta
                    .get("terrain")
                    .and_then(|t| t.as_str())
                    .unwrap_or("everon")
                    .to_string(),
                time: s(env, "time", "06:00"),
                weather: s(env, "weather", "clear"),
                show_hillshade: env
                    .and_then(|e| e.get("showHillshade"))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true),
                hillshade_opacity: env
                    .and_then(|e| e.get("hillshadeOpacity"))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.4),
                show_grid: env
                    .and_then(|e| e.get("showGrid"))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true),
            })
        })
        .unwrap_or_default()
}

/// One raw `meta.environment` key, exactly as the document holds it — `None` when it is unset.
///
/// **T-224 — why this exists next to [`read_env`].** `read_env` decodes into the fixed
/// [`MissionEnv`] struct, so it can only ever see the keys that struct names. The mission-flow
/// controls in `eden_chrome` author `briefingSeconds` / `safeStartSeconds` / `timeLimitSeconds` /
/// `jip` into the same `meta.environment` bag and have to read their own values back on every
/// dialog open, or a saved 45-minute mission reopens claiming the 90-minute default — the
/// reverted-setting symptom T-192 exists to remove, on a different value.
///
/// Widening `MissionEnv` would have been the other option and is worse: it lives in `dto.rs`, whose
/// types are pinned by the R-api golden round-trip tests against the **API** contract, and these
/// keys are not part of it. A raw read keeps the bag's growth out of the wire types.
pub fn read_env_value(key: &str) -> Option<serde_json::Value> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let root: serde_json::Value = serde_json::from_str(&core.small_maps_json()).ok()?;
        root.get("meta")?.get("environment")?.get(key).cloned()
    })
}

/// The doc's `meta.title` (empty when unset — the strip falls back to the route id). T-172 B9.
pub fn read_title() -> String {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            let root: serde_json::Value = serde_json::from_str(&core.small_maps_json()).ok()?;
            root.get("meta")?
                .get("title")
                .and_then(|t| t.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default()
}

/// Strip title commit (React's editable title → `setTitle`) — writes `meta.title` + runs the
/// shared post-edit tail (one undo step, dirty flag). T-172 B9.
pub fn set_title(title: &str) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_title(title);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
}

/// The doc's raw `slots_json` — the SZ estimator's input (T-172 B9).
pub fn slots_json() -> Option<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        d.as_ref().map(|core| core.slots_json())
    })
}

/// Merge an environment patch (React `updateEnvironment`) + run the shared tail (one undo step).
pub fn update_environment(patch_json: String) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.update_environment(&patch_json);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
}

/* ───────────────────────── keyboard actions (T-159.26 — MissionCreatorPage) ───────────────────────── */

thread_local! {
    /// The in-editor copy/paste clipboard (React `clipboardRef`) — raw slot dicts from `slots_json`.
    static CLIPBOARD: RefCell<Vec<serde_json::Value>> = const { RefCell::new(Vec::new()) };
}

/// Delete/Backspace — remove the selected entities (React `removeEntities`).
///
/// **Wave 145 F-4 — NOT "in one undoable step", which is what this line used to claim.** The slot
/// removal is one transaction, but the T-672 edge cascade opens one per deleted slot and the T-784
/// comment loop opens one per removed note, so a multi-select or mixed delete costs several Ctrl+Z
/// presses to walk back. Nothing is lost — undo restores all of it — and a single-slot or
/// single-comment delete really is one step. The full argument, and why the batch would have to be
/// minted core-side, is in the KNOWN AND ACCEPTED note on the cascade below.
///
/// **T-784 — the selection can now hold a COMMENT id, so the verb PARTITIONS it.** A comment is a
/// `commentsById` key, not a slot id: handing it to `remove_slots` removed nothing and reported
/// success, which is the T-779 class (an acknowledgement for a write that never landed) and exactly
/// the failure a comment click would otherwise have shipped. Each half goes to the mutator that owns
/// it — [`crate::editor_ops::delete_comment`]'s `remove_comment` for the notes, `remove_slots` for
/// everything else — and neither half runs when it is empty, so **Delete over a comment-only
/// selection removes that comment and touches nothing else**. A MIXED selection removes both, which
/// is what selecting both and pressing Delete means.
///
/// The membership question is asked of [`comment_details`], the same `comments_json` read the
/// Outliner rows, the map lane and the map pick are built from — not a prefix test on the id. A
/// `cmt-` prefix is [`mint_comment_id`]'s convention, not a document invariant, and a hydrated
/// mission is free to carry comment ids that were never minted here.
pub fn delete_selection() -> bool {
    let removed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let ids = ctx.selection.borrow().clone();
        if ids.is_empty() {
            return false;
        }
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            let comment_ids: std::collections::HashSet<String> =
                comment_details(core).into_iter().map(|c| c.id).collect();
            let (comments, ids): (Vec<String>, Vec<String>) =
                ids.into_iter().partition(|id| comment_ids.contains(id));
            for id in &comments {
                core.remove_comment(id);
            }
            // T-672 — take each deleted unit's connection edges with it. Without this every delete
            // manufactures `CONN-DANGLING` findings the operator then has to clean up by hand — a
            // delete that half-finishes, which is exactly the failure class the connection graph's
            // warning is about. Deliberately BEFORE `remove_slots` so the cascade reads the id set
            // while the entities still exist.
            //
            // KNOWN AND ACCEPTED: this is one transaction per deleted slot plus one for the slot
            // removal, so a multi-select delete is several undo steps rather than one. Folding the
            // edge cascade into `remove_slots_in_txn` would fix that, but `remove_slots_in_txn` is
            // shared with `remove_editor_layer` and re-shaping it is a core-side change this slice
            // does not need to make to keep the graph honest. The visible cost is extra Ctrl+Z
            // presses; the alternative cost was dangling edges.
            //
            // WAVE 145 F-4 — **the comment loop above joins that acceptance, explicitly.** T-784
            // wrote it as a plain `for` over `core.remove_comment`, and `remove_comment` opens its
            // own transaction per call, so a delete over N notes is N more undo steps on top of the
            // cascade's. It was never added to this paragraph, which left the T-672 note reading as
            // if the cascade were the only multi-step half. It is the same accepted class, for the
            // same reason (the one-txn batch would have to be minted core-side, and `store.rs` is
            // out of this slice), with the same visible cost: extra Ctrl+Z presses, never lost work
            // — undo restores every removed note. A SINGLE-comment delete is genuinely one step.
            for id in &ids {
                let _ = core.remove_connections_touching(id);
            }
            // T-784 — GUARDED, because the comment-only case is the acceptance: with nothing but
            // notes selected there is no slot half, and handing `remove_slots` an empty `Vec` is a
            // pointless transaction opened over a document this delete did not change.
            //
            // WAVE 145 F-2 — what that guard is NOT. It was justified here as preventing "an empty
            // undo step", and that justification is empirically false: yrs skips a transaction that
            // wrote nothing, so `remove_slots(vec![])` leaves `can_undo()` FALSE and mints no undo
            // step at all (probed natively against `MissionDocCore`). The guard stays — a call that
            // provably cannot change anything should not be made, and saying "I removed the slots"
            // over an empty half is the vocabulary this partition exists to avoid — but it is
            // hygiene, not the last thing standing between the operator and a dead Ctrl+Z.
            if !ids.is_empty() {
                core.remove_slots(ids);
            }
        }
        ctx.selection.borrow_mut().clear();
        true
    });
    if removed {
        crate::mission_history::after_local_edit();
    }
    removed
}

/// Spacebar — center the camera on the selection centroid (React `flyTo`, no auto-fly on click).
pub fn center_on_selection() -> bool {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        if sel.is_empty() {
            return false;
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let soa = core.materialize();
        let mut sx = 0.0f64;
        let mut sy = 0.0f64;
        let mut n = 0.0f64;
        for id in &sel {
            if let Some(row) = soa.ids.iter().position(|s| s == id) {
                sx += f64::from(soa.xs[row]);
                sy += f64::from(soa.ys[row]);
                n += 1.0;
            }
        }
        if n == 0.0 {
            return false;
        }
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_view(sx / n, sy / n, e.zoom()); // keep zoom, center on centroid
            e.on_camera_changed(); // T-172 H5 — slot sizing/cluster gate
            true
        } else {
            false
        }
    })
}

/// Ctrl/Cmd+C — snapshot the selected slot dicts to the clipboard (React copy branch).
pub fn copy_selection() -> bool {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel: std::collections::HashSet<String> =
            ctx.selection.borrow().iter().cloned().collect();
        if sel.is_empty() {
            return false;
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
            return false;
        };
        let clip: Vec<serde_json::Value> = map
            .as_object()
            .map(|o| {
                o.values()
                    .filter(|v| {
                        v.get("id")
                            .and_then(|i| i.as_str())
                            .is_some_and(|i| sel.contains(i))
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if clip.is_empty() {
            return false;
        }
        CLIPBOARD.with(|cb| *cb.borrow_mut() = clip);
        true
    })
}

/// Ctrl/Cmd+V — paste the clipboard at `(cx, cy)` (the map cursor), preserving the relative layout
/// (React `pasteSlots`; centroid → cursor). Mints ids, files under the resolved layer, keeps the
/// source squad id (inert while squads is empty), selects the paste. `true` if anything pasted.
///
/// **T-777** — and each copy keeps the elevation it was authored at, rather than landing at ground
/// level. See the `z_rows` note in the body for why that read is off the clipboard snapshot.
pub fn paste_at_cursor(cx: Option<f64>, cy: Option<f64>) -> bool {
    let placed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let clip = CLIPBOARD.with(|cb| cb.borrow().clone());
        if clip.is_empty() {
            return Vec::new();
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let layer_id = ensure_layer(ctx, core);
        let terrain = serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
            .ok()
            .and_then(|v| v.get("meta")?.get("terrain")?.as_str().map(str::to_string))
            .unwrap_or_default();
        let b = map_engine_core::mission::compile::terrain_bounds(&terrain);

        let n = clip.len();
        let mut ids = Vec::with_capacity(n);
        let (mut sx, mut sy, mut srot, mut zs) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let (mut squad_ids, mut layer_ids) = (Vec::new(), Vec::new());
        let (mut roles, mut tags, mut asset_ids, mut stances, mut loadouts) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut extras = Vec::with_capacity(n);
        let g = |v: &serde_json::Value, k: &str| {
            v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
        };
        let gp = |v: &serde_json::Value, k: &str| {
            v.get("position")
                .and_then(|p| p.get(k))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
        };
        // **T-777 — the copied slots' authored `z`, keyed by SOURCE id, resolved once for the whole
        // paste.** Built here rather than inside the loop below so the resolution is hoisted, and
        // built from the CLIPBOARD rather than from the live document, for two reasons:
        //
        // 1. `copy_selection` files the RAW `slots_json()` rows on the clipboard — the very rows
        //    [`slot_z`] is written to read — so the authored `z` is already in hand and the paste
        //    costs ZERO extra `raw_slot_rows` parses (that helper is O(document) JSON). The exact
        //    f64 survives, and a slot on a hidden layer still resolves: neither would be true off
        //    the materialized SoA, whose `zs` column is f32 and which drops T-665 hidden slots.
        // 2. `x`, `y` and `rotation` above already come from this snapshot. Reading `z` from the
        //    LIVE document instead would compose the copy's old x/y with the source's *current*
        //    elevation the moment anyone nudged the original after copying, and would resolve to
        //    nothing at all when the source has since been deleted or the clipboard is pasted into
        //    a different mission — and a failed read here is a silently flattened entity.
        //
        // `keep_z_rows` is deliberately NOT the reader for this path: its guard answers "could this
        // `update_slot_position` write terrain-follow an authored z to 0.0?", a question about that
        // mutator's Option signature which `paste_slots` does not have. Its PARTNER `slot_z` is the
        // shared reader, and that is what is reused — one z-resolution vocabulary, not a third.
        let z_rows: serde_json::Map<String, serde_json::Value> = clip
            .iter()
            .filter_map(|s| Some((s.get("id")?.as_str()?.to_string(), s.clone())))
            .collect();
        // T-220 — fields the parallel paste arrays do not carry (unknown keys + unknown
        // position sub-keys). Known keys are filtered again inside `paste_slots`.
        const PASTE_KNOWN: &[&str] = &[
            "id",
            "squadId",
            "index",
            "role",
            "tag",
            "assetId",
            "stance",
            "loadoutId",
            "loadout",
        ];
        for slot in &clip {
            ids.push(mint_id(ctx, core));
            sx.push(gp(slot, "x"));
            sy.push(gp(slot, "y"));
            srot.push(gp(slot, "rotation"));
            // **T-777 — the copy keeps the original's elevation.** This used to push a literal
            // ground value "for byte-parity with the flat-map case". The operator set that parity
            // aside on 2026-08-08 (it was a migration safety net, never a contract), and it was
            // unrecoverable regardless: nothing in this frontend re-samples terrain after a paste
            // — `terrainZ` did not survive the React deletion — so the zero was FINAL, and a
            // rooftop entity fell to the ground the moment it was duplicated. Resolved through the
            // shared [`slot_z`] against `z_rows` above; a row carrying no finite `z` still reads as
            // ground, exactly as before, so a flat-map paste is unchanged.
            //
            // **ORDER.** This push and the `ids.push` at the top of this iteration are the same
            // pass of the same walk over `clip`, so `zs[i]` is by construction the elevation of the
            // row that minted `ids[i]` — a total map, not a convention two sites happen to share.
            // Both vectors reach `paste_slots` below untouched; nothing between here and that call
            // sorts, filters, or re-orders either one.
            zs.push(slot_z(&z_rows, &g(slot, "id")).unwrap_or(0.0));
            // Keep the source squad if it still exists, else "" (empty squads map → inert).
            squad_ids.push(g(slot, "squadId"));
            layer_ids.push(layer_id.clone());
            roles.push(g(slot, "role"));
            tags.push(g(slot, "tag"));
            asset_ids.push(g(slot, "assetId"));
            let st = g(slot, "stance");
            stances.push(if st.is_empty() {
                "stand".to_string()
            } else {
                st
            });
            loadouts.push(
                slot.get("loadout")
                    .filter(|l| !l.is_null())
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default(),
            );
            let mut extra = serde_json::Map::new();
            if let Some(obj) = slot.as_object() {
                for (k, v) in obj {
                    if PASTE_KNOWN.contains(&k.as_str()) {
                        continue;
                    }
                    if k == "position" {
                        if let Some(pos) = v.as_object() {
                            let mut pos_extra = serde_json::Map::new();
                            for (pk, pv) in pos {
                                if !matches!(pk.as_str(), "x" | "y" | "z" | "rotation") {
                                    pos_extra.insert(pk.clone(), pv.clone());
                                }
                            }
                            if !pos_extra.is_empty() {
                                extra.insert(
                                    "position".into(),
                                    serde_json::Value::Object(pos_extra),
                                );
                            }
                        }
                        continue;
                    }
                    extra.insert(k.clone(), v.clone());
                }
            }
            extras.push(if extra.is_empty() {
                String::new()
            } else {
                serde_json::Value::Object(extra).to_string()
            });
        }
        core.paste_slots(
            ids.clone(),
            squad_ids,
            layer_ids,
            sx,
            sy,
            srot,
            zs,
            roles,
            tags,
            asset_ids,
            stances,
            loadouts,
            extras,
            cx,
            cy,
            b[2],
            b[3],
        );
        *ctx.selection.borrow_mut() = ids.clone();
        ids
    });
    if !placed.is_empty() {
        crate::mission_history::after_local_edit();
        true
    } else {
        false
    }
}

/* ═══════════════════════════════ T-650 — saved compositions ═══════════════════════════════ */
//
// Save (COMP-SAVE-001): capture the current selection → a self-contained composition row. Place
// (COMP-PLACE-001): the `Pending::Composition` arm above re-anchors + stamps every entity as one
// undo step. Edit (COMP-EDIT-001): rename / recategorize / delete the row. The three metadata
// fields ATTR-FIELD-COMP-{TITLE,AUTHOR,CATEGORY} are the row's own title/author/category.
//
// The capture shapes REUSE the clipboard capture (`copy_selection` / `paste_at_cursor`): a slot's
// role/tag/asset/stance/loadout come off `slots_json` exactly as the paste reads them, and a
// vehicle's heading/crew SHAPE come off `small_maps_json` exactly as `vehicle_rows` reads them. The
// only transform is absolute→relative: each entry stores `(dx, dz)` from the selection centroid, so
// a later place re-anchors the centroid at the cursor.
//
// T-781 widened the capture twice, and both halves are documented on `capture_selection_entities`:
// a selected COMMENT is captured (the composable clause of `PLACE-COMMENT-001`) and is stamped back
// into the editor-only `commentsById` root that never compiles; and every placeable entry carries
// its AUTHORED elevation through the shared `slot_z` reader, so a composition of rooftop entities
// no longer comes back on the ground.

/// One saved composition as the palette needs it: identity, metadata, and an entity count for the
/// row summary. The `entities` payload itself stays in the doc (the dock never needs to unpack it —
/// only the count and the three metadata fields are shown).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionRow {
    pub id: String,
    pub title: String,
    pub author: String,
    pub category: String,
    pub entity_count: usize,
}

/// T-650 (COMP-SAVE-001) — capture the current selection into a new saved composition, titled
/// `title` under `category`, authored by `author` (the current user's display string as-authored).
/// Returns the new composition id, or `None` when the selection is empty / captured nothing.
///
/// Each selected id is classified as a slot, vehicle, object, or comment (whichever map holds it)
/// and emitted as a RELATIVE-OFFSET entry from the selection centroid. A slot carries
/// role/tag/asset/stance and its loadout blob; a vehicle carries resourceName/heading/crewed + the
/// crew SHAPE; an object carries alias/resourceName/faction; a comment (T-781) carries its
/// title/tooltip and nothing else. Runs the shared dirty tail (one undo step for the save).
#[must_use]
pub fn save_composition(title: String, category: String, author: String) -> Option<String> {
    let new_id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let sel: Vec<String> = ctx.selection.borrow().clone();
        if sel.is_empty() {
            return None;
        }
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let entities = capture_selection_entities(core, &sel);
        if entities.is_empty() {
            return None;
        }
        let comp_id = mint_composition_id(ctx, core);
        let row = serde_json::json!({
            "id": comp_id,
            "title": title,
            "author": author,
            "category": category,
            "entities": entities,
        });
        core.add_composition(&comp_id, &row.to_string());
        Some(comp_id)
    });
    if new_id.is_some() {
        crate::mission_history::after_local_edit();
    }
    new_id
}

/// Build the relative-offset `entities` array for a selection. Slots come off `slots_json`
/// (the exact-f64 dicts the clipboard capture reads); vehicles, objects and comments come off
/// `small_maps_json`. The centroid is the mean of every captured entry's world position, in
/// selection order (a stable f64 sum), so the offsets recenter cleanly on place.
///
/// **T-781 Part A — a COMMENT is a capture source.** `PLACE-COMMENT-001` asks for composable
/// comments; before this, `commentsById` was simply not read here, so a selected comment was
/// dropped without a word and the composition came back one entity short. A comment entry carries
/// `title`/`tooltip` and no `resourceName`, because the thing it stamps is an editor-only
/// annotation that `MissionDocCore::place_composition` writes into the root `comments` map — the
/// collection `mission::flatten::EditorPayload` does not declare and serde therefore drops before
/// the mod document exists.
///
/// **T-781 Part B — every placeable entry carries its AUTHORED elevation**, resolved through the
/// shared [`slot_z`] reader against the exact-f64 rows above. Not the materialized SoA: its `zs`
/// column is f32 and it omits hidden-layer slots (T-665), so a careful operator's tucked-away
/// rooftop would read back as ground. `slot_z` is written against `{id: {position: {z}}}`, which is
/// the shape of all three source maps, so this is the one z-resolution vocabulary and not a third.
/// The elevation is written as a FIELD of the entry that produced it — never a parallel vector — so
/// no index can drift and give an entity somebody else's height.
fn capture_selection_entities(core: &MissionDocCore, sel: &[String]) -> Vec<serde_json::Value> {
    let slots = serde_json::from_str::<serde_json::Value>(&core.slots_json()).unwrap_or_default();
    let small =
        serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).unwrap_or_default();
    let vehicles = small.get("vehiclesById").cloned().unwrap_or_default();
    let entities = small.get("entitiesById").cloned().unwrap_or_default();
    let comments = small.get("commentsById").cloned().unwrap_or_default();

    // First pass: resolve each id to (kind, world position, source row) so the centroid is over the
    // SAME set the entries are built from.
    struct Captured {
        kind: &'static str,
        x: f64,
        y: f64,
        rotation: f64,
        /// T-781 Part B — the row's authored `position.z`. Zero for a comment, which has no third
        /// component to carry.
        elevation: f64,
        row: serde_json::Value,
    }
    let pos = |row: &serde_json::Value| -> (f64, f64, f64) {
        let p = row.get("position");
        (
            p.and_then(|p| p.get("x"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            p.and_then(|p| p.get("y"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            p.and_then(|p| p.get("rotation"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
        )
    };
    // T-781 Part B — the shared z reader, applied to whichever source map holds the id. Resolved in
    // the SAME iteration that reads x/y/rotation off that row, so the elevation cannot be paired
    // with a different entity's offsets.
    let elev_of = |src: &serde_json::Value, id: &str| -> f64 {
        src.as_object().and_then(|m| slot_z(m, id)).unwrap_or(0.0)
    };
    let mut captured: Vec<Captured> = Vec::new();
    for id in sel {
        if let Some(row) = slots.get(id) {
            let (x, y, r) = pos(row);
            captured.push(Captured {
                kind: "slot",
                x,
                y,
                rotation: r,
                elevation: elev_of(&slots, id),
                row: row.clone(),
            });
        } else if let Some(row) = vehicles.get(id) {
            let (x, y, r) = pos(row);
            captured.push(Captured {
                kind: "vehicle",
                x,
                y,
                rotation: r,
                elevation: elev_of(&vehicles, id),
                row: row.clone(),
            });
        } else if let Some(row) = entities.get(id) {
            let (x, y, r) = pos(row);
            captured.push(Captured {
                kind: "object",
                x,
                y,
                rotation: r,
                elevation: elev_of(&entities, id),
                row: row.clone(),
            });
        } else if let Some(row) = comments.get(id) {
            // T-781 — a comment row's position is `{x, z}`: TWO HORIZONTALS, the `$defs/marker`
            // vocabulary, and no height at all. Its `z` is therefore the second WORLD axis and
            // belongs in `y` here beside every other kind's `position.y` — reading it as an
            // elevation would file the note's northing as its altitude and place it at the origin.
            let p = row.get("position");
            let axis = |k: &str| {
                p.and_then(|p| p.get(k))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0)
            };
            captured.push(Captured {
                kind: "comment",
                x: axis("x"),
                y: axis("z"),
                // A note has no heading and no height; both are absent from the row by design.
                rotation: 0.0,
                elevation: 0.0,
                row: row.clone(),
            });
        }
    }
    if captured.is_empty() {
        return Vec::new();
    }
    let n = captured.len() as f64;
    let cx = captured.iter().map(|c| c.x).sum::<f64>() / n;
    let cy = captured.iter().map(|c| c.y).sum::<f64>() / n;

    let s = |row: &serde_json::Value, k: &str| {
        row.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    captured
        .into_iter()
        .map(|c| {
            let mut e = serde_json::Map::new();
            e.insert("kind".into(), serde_json::json!(c.kind));
            e.insert("dx".into(), serde_json::json!(c.x - cx));
            e.insert("dz".into(), serde_json::json!(c.y - cy));
            e.insert("rotation".into(), serde_json::json!(c.rotation));
            // T-781 Part B — the authored height, spelled `elevation` because `dz` is already spent
            // on the second HORIZONTAL offset in this same map. `place_composition` reads it back by
            // that name. Omitted for a comment, whose row has no such component to restore.
            if c.kind != "comment" {
                e.insert("elevation".into(), serde_json::json!(c.elevation));
            }
            match c.kind {
                "slot" => {
                    e.insert("role".into(), serde_json::json!(s(&c.row, "role")));
                    e.insert("tag".into(), serde_json::json!(s(&c.row, "tag")));
                    e.insert("assetId".into(), serde_json::json!(s(&c.row, "assetId")));
                    let stance = s(&c.row, "stance");
                    e.insert(
                        "stance".into(),
                        serde_json::json!(if stance.is_empty() {
                            "stand".to_string()
                        } else {
                            stance
                        }),
                    );
                    // The loadout blob VERBATIM (the paste-copies-loadout contract); omit when absent.
                    if let Some(l) = c.row.get("loadout").filter(|l| !l.is_null()) {
                        e.insert("loadout".into(), l.clone());
                    }
                }
                "vehicle" => {
                    e.insert(
                        "resourceName".into(),
                        serde_json::json!(s(&c.row, "resourceName")),
                    );
                    // `crewed` omit idiom: only carry `false` (absence = the with-crew default).
                    if c.row.get("crewed") == Some(&serde_json::Value::Bool(false)) {
                        e.insert("crewed".into(), serde_json::json!(false));
                    }
                    // The crew SHAPE verbatim (`{seat_id: slot_id}`), when the vehicle is crewed.
                    if let Some(crew) = c.row.get("crew").filter(|v| v.is_object()) {
                        e.insert("crew".into(), crew.clone());
                    }
                }
                "comment" => {
                    // T-781 — the two ATTR-FIELD-CMT-* text fields, verbatim and uncapped, exactly
                    // as the mutators store them (they reach no consumer that could reject them, so
                    // there is nothing to normalise FOR). The third field, position, is already the
                    // entry's `(dx, dz)`.
                    e.insert("title".into(), serde_json::json!(s(&c.row, "title")));
                    e.insert("tooltip".into(), serde_json::json!(s(&c.row, "tooltip")));
                }
                _ => {
                    // object
                    e.insert("alias".into(), serde_json::json!(s(&c.row, "alias")));
                    e.insert(
                        "resourceName".into(),
                        serde_json::json!(s(&c.row, "resourceName")),
                    );
                    e.insert("faction".into(), serde_json::json!(s(&c.row, "faction")));
                }
            }
            serde_json::Value::Object(e)
        })
        .collect()
}

/// Mint an unused composition id (`comp-{n}`), proven unique against the live compositions map.
fn mint_composition_id(ctx: &OpsCtx, core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&core.compositions_json())
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    loop {
        let id = format!("comp-{}", ctx.next_id.get());
        ctx.next_id.set(ctx.next_id.get().saturating_add(1));
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// T-650 — read every saved composition for the palette list, sorted by (category, title) so the
/// dock can group them. Off [`MissionDocCore::compositions_json`].
#[must_use]
pub fn composition_rows() -> Vec<CompositionRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.compositions_json()) else {
            return Vec::new();
        };
        let Some(obj) = map.as_object() else {
            return Vec::new();
        };
        let s = |v: &serde_json::Value, k: &str| {
            v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
        };
        let mut rows: Vec<CompositionRow> = obj
            .iter()
            .map(|(id, v)| CompositionRow {
                id: id.clone(),
                title: s(v, "title"),
                author: s(v, "author"),
                category: s(v, "category"),
                entity_count: v
                    .get("entities")
                    .and_then(|e| e.as_array())
                    .map_or(0, Vec::len),
            })
            .collect();
        rows.sort_by(|a, b| {
            a.category
                .cmp(&b.category)
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.id.cmp(&b.id))
        });
        rows
    })
}

/// T-650 — saved-composition count (backs the palette header count).
#[must_use]
pub fn composition_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| {
                ctx.doc
                    .borrow()
                    .as_ref()
                    .map(MissionDocCore::composition_count)
            })
            .unwrap_or(0)
    })
}

/// T-650 (COMP-EDIT-001 / ATTR-FIELD-COMP-TITLE) — rename a saved composition (inline edit). Blank
/// titles are allowed at the doc layer; the dock declines to write an all-whitespace title.
pub fn rename_composition(id: String, title: String) -> bool {
    edit_composition(|core| core.set_composition_title(&id, &title))
}

/// T-650 (COMP-EDIT-001 / ATTR-FIELD-COMP-CATEGORY) — recategorize a saved composition (inline).
pub fn recategorize_composition(id: String, category: String) -> bool {
    edit_composition(|core| core.set_composition_category(&id, &category))
}

/// T-650 (ATTR-FIELD-COMP-AUTHOR) — set a saved composition's author display string (inline).
pub fn set_composition_author(id: String, author: String) -> bool {
    edit_composition(|core| core.set_composition_author(&id, &author))
}

/// T-650 (COMP-EDIT-001) — delete a saved composition (inline). Clears the place arm if it was armed
/// on the row being deleted, so a release cannot commit a composition that no longer exists.
pub fn delete_composition(id: String) -> bool {
    let did = edit_composition(|core| core.remove_composition(&id));
    if did {
        OPS_CTX.with(|c| {
            if let Some(ctx) = c.borrow().as_ref() {
                let clear =
                    matches!(&*ctx.pending.borrow(), Some(Pending::Composition(p)) if *p == id);
                if clear {
                    *ctx.pending.borrow_mut() = None;
                }
            }
        });
    }
    did
}

/// Shared edit tail for the composition mutators: run `f` against the core, then the dirty tail
/// (one undo step). Returns `false` when there is no doc.
fn edit_composition(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/* ───────────────────────── Attributes modal (T-159.26 / .23 spec) ───────────────────────── */

/// T-649 (ATTR-MULTI-001) — the shared open path for [`open_attributes`] / [`open_arsenal`].
///
/// **This inverts the old A1 contract.** Until T-649 both entry points opened with
/// `if ctx.selection.borrow().len() > 1 { return; }` — a multi-selection SUPPRESSED the modal
/// entirely. That made two context-menu rows dishonest: `context_menu.rs:277` / `:281` register
/// "Edit Loadout..." and "Attributes..." with `MenuEntry::on(..)` unconditionally, so at
/// `selection.len() > 1` the rows rendered ENABLED and clicking them did nothing at all (the
/// T-716 live-but-inert rows). A multi-selection now OPENS the modal in multi-edit mode instead,
/// which is what makes those rows honest.
///
/// Selection handling is the whole difference between the two modes and is why this is one
/// function rather than a copied guard:
///   * `id` is **inside** a multi-selection ⇒ leave the selection ALONE. Replacing it with `[id]`
///     (what the single path does) would silently collapse the very set the operator is about to
///     multi-edit, and the modal reads that set back through [`attrs_multi_ids`].
///   * otherwise (single selection, or a right-click that retargeted to an entity outside the
///     selection) ⇒ replace with `[id]`, so modal, SEL readout, and map tint agree — the original
///     behaviour, unchanged.
fn open_attrs_modal(id: String, arsenal_tab: bool) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let keep_selection = {
            let sel = ctx.selection.borrow();
            sel.len() > 1 && sel.contains(&id)
        };
        if !keep_selection {
            *ctx.selection.borrow_mut() = vec![id.clone()];
            let ids = ctx.selection.borrow().clone();
            let mut eng = ctx.engine.borrow_mut();
            if let Some(e) = eng.as_mut() {
                e.set_selection(ids);
            }
        }
        if arsenal_tab {
            ctx.attrs_tab.set(3);
        }
        ctx.attrs_open.set(Some(id));
    });
    crate::mission_history::refresh_selection();
}

/// Open Attributes for `id` (the dbl-click / outliner-activate contract). A multi-selection opens
/// the modal in MULTI-EDIT mode over the whole selection — see [`open_attrs_modal`] for the
/// inversion of the old suppress-on-multi rule. Leaves the Attributes tab index alone (default
/// Identity until the user changes it).
pub fn open_attributes(id: String) {
    open_attrs_modal(id, false);
}

/// T-180.9 — Open Attributes on the Arsenal tab (`TABS[3]`) for `id`. Same selection handling as
/// [`open_attributes`].
///
/// T-649 honesty note: inverting the guard here is what stops the "Edit Loadout..." row being
/// inert on a multi-selection — the modal now opens. The Arsenal tab BODY
/// (`arsenal.rs::ArsenalTab`) still edits ONE slot (the clicked `id`); loadout multi-apply lives in
/// `arsenal.rs`, which is not this slice's to touch. `attributes.rs` renders a banner on the
/// Arsenal tab under a multi-selection saying exactly that, so the modal never implies it is
/// writing all of them.
pub fn open_arsenal(id: String) {
    open_attrs_modal(id, true);
}

/// T-649 SEL-ALL-001 — Ctrl/Cmd+A: replace the selection with everything **on screen**.
///
/// Eden scopes Select All to the viewport, not to the whole mission, so this is a viewport-rect
/// query over [`crate::select_tool::view_ids_with_vehicles`] — the marquee's own primitive with its
/// corners pinned to the canvas — and not a `soa.ids` dump. `viewport_w`/`viewport_h` are the
/// container's CSS size at keypress; the camera is snapshotted from the live engine view the same
/// way a pointer-down freezes one, so Ctrl+A and a full-canvas marquee drag agree by construction.
///
/// `vehicle_points()` is resolved BEFORE the `OPS_CTX` borrow opens (it opens its own), keeping the
/// module's one-borrow-per-`pub fn` discipline. Returns whether it acted, so the keydown arm can
/// `prevent_default` — the browser's own Select All would otherwise blue-wash the editor chrome.
pub fn select_all_in_view(viewport_w: f64, viewport_h: f64) -> bool {
    if !(viewport_w > 0.0 && viewport_h > 0.0) {
        return false;
    }
    let points = vehicle_points();
    let acted = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let cam = {
            let eng = ctx.engine.borrow();
            let Some(e) = eng.as_ref() else {
                return false;
            };
            crate::select_tool::frozen_camera(
                viewport_w,
                viewport_h,
                e.target_x(),
                e.target_y(),
                e.zoom(),
            )
        };
        let ids = {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            crate::select_tool::view_ids_with_vehicles(&cam, &core.materialize(), &points)
        };
        // The engine tint lane is slots-only (the vehicle lane draws its own selection) — the same
        // split the `LG::Marquee` commit makes in `mission_editor.rs`.
        let slot_ids: Vec<String> = ids
            .iter()
            .filter(|i| !points.iter().any(|(v, _, _)| v == *i))
            .cloned()
            .collect();
        *ctx.selection.borrow_mut() = ids;
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_selection(slot_ids);
        }
        true
    });
    if acted {
        // Selection change, not a doc edit — the SEL readout only (T-159.21), never a history step.
        crate::mission_history::refresh_selection();
    }
    acted
}

/// Close the modal (Esc / backdrop / close button).
pub fn close_attributes() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.attrs_open.set(None);
        }
    });
}

/// T-082 — every slot row of the doc, keyed by id, straight off `slots_json()`.
///
/// The raw rows, NOT the SoA: `assetId` and `description` (and every other authored key) live only
/// here. Parsed once per call and handed to the readers below, because `slots_json` is O(all slots)
/// JSON and the modal must not pay it per field. Both callers already pay one `materialize()` of
/// the same order, and both run on a modal render — never the frame loop.
fn raw_slot_rows(core: &MissionDocCore) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::from_str::<serde_json::Value>(&core.slots_json()) {
        Ok(serde_json::Value::Object(m)) => m,
        _ => serde_json::Map::new(),
    }
}

/// T-082 — one string key off a raw slot row; empty when absent or not a string (the `add_slot`
/// omit idiom means "absent" is the canonical unset, so it must read back as empty, not as a hole).
fn row_str(rows: &serde_json::Map<String, serde_json::Value>, id: &str, key: &str) -> String {
    rows.get(id)
        .and_then(|r| r.get(key))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// **wave-127 F-2** — one slot's CURRENT `z`, exact, straight off the raw row. `None` when the slot
/// is absent or carries no finite numeric `z`.
///
/// Read from the raw row rather than the materialized SoA for two reasons, both of which would
/// corrupt the value this exists to preserve: the SoA's `zs` column is **f32**, so round-tripping an
/// authored `z` through it would rewrite it as a slightly different number on every X tweak; and a
/// slot filed under a HIDDEN layer is omitted from the SoA entirely (T-665), so the read would fail
/// on precisely the slots a careful operator has tucked away — and a failed read is a zeroed z.
pub(crate) fn slot_z(rows: &serde_json::Map<String, serde_json::Value>, id: &str) -> Option<f64> {
    rows.get(id)?
        .get("position")?
        .get("z")?
        .as_f64()
        .filter(|v| v.is_finite())
}

/// **wave-127 F-2** — the slot rows an Attributes position commit needs in order to KEEP an authored
/// `z`; `None` when this commit cannot zero one, and the callers then skip the read entirely.
///
/// `update_slot_position` terrain-follows on any x/y write — `z = None` with `x` or `y` set stores
/// `pz = 0.0`. In the JS oracle that is harmless because the caller re-samples the DEM and writes the
/// real elevation straight back. **In this frontend nothing re-samples after an Attributes commit**:
/// `terrainZ` did not survive the React deletion (the only mention left in this module is a comment
/// on [`place_at`]), so the document simply keeps the literal `0.0`. An operator who authored a
/// rooftop `z` and later nudged X by a metre in the Transform tab lost that `z` — silently, and
/// inside the same undo step as the X edit.
///
/// The fix is here, at the CALLER, not in the mutator: `MissionDocCore::update_slot_position` claims
/// byte-parity with `ydoc.updateSlotPosition` and keeps it. Its callers read the current `z` and pass
/// it back in, which makes the terrain-follow a no-op for the paths that have no sampler behind them.
///
/// **wave-127 F-5 — the placement helpers are one of those paths.** [`commit_positions`] (Align /
/// Distribute / the placement patterns) writes x/y with `z = None` for every slot it moves, so it
/// zeroed an authored z exactly the way the Attributes tab did — while preserving it for vehicles in
/// the same selection. It now resolves the z through this pair too.
///
/// **wave-127 F-6 — and so does the marquee/handle DRAG.** It commits through
/// `move_entities_and_vehicles` in `mission_editor` (the `LG::Move` arm), which used to pass
/// `vec![0.0; n]` and so flattened every dragged slot inside one txn. That caller lives outside this
/// module, which is why this pair is `pub(crate)`: the drag reads the rows ONCE and maps them over
/// its `slot_ids` in order. One z-resolution vocabulary for all three paths, not three.
///
/// Reading the rows is O(document) JSON, so it is done once per COMMIT — once per BATCH for
/// `commit_positions`, which moves many entities — and only for a commit that could actually zero a
/// `z`: an explicit z write, or a rotation-only edit, never reaches it.
pub(crate) fn keep_z_rows(
    core: &MissionDocCore,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    (z.is_none() && (x.is_some() || y.is_some())).then(|| raw_slot_rows(core))
}

/// Read one slot's editable attributes for the modal's field values.
/// `None` when the slot no longer exists (undone away while open → the modal closes).
///
/// T-082 — the SoA supplies the transform/identity columns; the raw row (`raw_slot_rows`) supplies
/// `assetId` and `description`, which the SoA does not carry. See [`SlotAttrs`] for why that split
/// is the ticket rather than an implementation detail.
///
/// **T-744 (wave-113 F-2)** — existence is RAW membership, not SoA membership.
/// `MissionDocCore::materialize()` drops T-665 layer-hidden and T-701 `editorHidden` slots before
/// any column is pushed. Gating `Option` on `soa.ids.iter().position(…)?` made Hide close the
/// Attributes modal through the same `None` → `close_attributes()` path as "slot was undone away".
/// Transform/identity columns still prefer the SoA when the slot is visible; when materialize has
/// filtered it, those columns fall back to the raw row so the open modal keeps real field values
/// (a zeroed fallback would corrupt the next Transform commit).
pub fn read_attrs(id: &str) -> Option<SlotAttrs> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let rows = raw_slot_rows(core);
        // T-744 — Option gate is RAW existence. Hide must not look like undo-away.
        if !rows.contains_key(id) {
            return None;
        }
        let soa = core.materialize();
        if let Some(row) = soa.ids.iter().position(|s| s == id) {
            let dict = |idx: u32, dict: &[String]| {
                if idx == NONE_IDX {
                    String::new()
                } else {
                    dict.get(idx as usize).cloned().unwrap_or_default()
                }
            };
            let stance = match soa.stance.get(row).copied().unwrap_or(0) {
                map_engine_core::doc::STANCE_CROUCH => "crouch",
                map_engine_core::doc::STANCE_PRONE => "prone",
                _ => "stand",
            };
            Some(SlotAttrs {
                id: id.to_string(),
                x: f64::from(soa.xs[row]),
                y: f64::from(soa.ys[row]),
                z: f64::from(soa.zs[row]),
                rotation: f64::from(soa.rotations[row]),
                stance: stance.to_string(),
                role: dict(soa.role_idx[row], &soa.roles),
                tag: dict(soa.tag_idx[row], &soa.tags),
                squad: dict(soa.squad_idx[row], &soa.squads),
                asset_id: row_str(&rows, id, "assetId"),
                description: row_str(&rows, id, "description"),
            })
        } else {
            // Hidden (layer or editorHidden) but still in the doc — keep the modal open.
            Some(slot_attrs_from_raw(&rows, id))
        }
    })
}

/// T-744 — Attributes snapshot from a raw `slots_json` row. Used when `materialize()` has filtered
/// the slot (hide) so the modal can stay open without inventing zeros for Transform fields.
fn slot_attrs_from_raw(rows: &serde_json::Map<String, serde_json::Value>, id: &str) -> SlotAttrs {
    let pos = rows.get(id).and_then(|r| r.get("position"));
    let num = |key: &str| -> f64 {
        pos.and_then(|p| p.get(key))
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0)
    };
    // Prefer the exact-z helper (finite filter) so a malformed z does not display as NaN-coerced 0.
    let z = slot_z(rows, id).unwrap_or_else(|| num("z"));
    let stance_raw = row_str(rows, id, "stance");
    let stance = match stance_raw.as_str() {
        "crouch" => "crouch",
        "prone" => "prone",
        _ => "stand",
    };
    SlotAttrs {
        id: id.to_string(),
        x: num("x"),
        y: num("y"),
        z,
        rotation: num("rotation"),
        stance: stance.to_string(),
        role: row_str(rows, id, "role"),
        tag: row_str(rows, id, "tag"),
        squad: row_str(rows, id, "squadId"),
        asset_id: row_str(rows, id, "assetId"),
        description: row_str(rows, id, "description"),
    }
}

/// T-082 (wave-102 F-7) — how many of `ids` are transform-locked.
///
/// The modal needs the COUNT, not a bool, because a multi-selection can straddle the lock: all
/// locked ⇒ the Transform fields are disabled outright; some locked ⇒ the fields stay live (the
/// unlocked members really will move) and the modal says how many will not. Reporting either case
/// as the other is the F-7 lie in a new costume.
///
/// Asks the CORE (`slot_layer_is_locked`), never a re-derived layer walk here: the whole value of
/// the affordance is that it cannot disagree with the mutator that refuses the write.
#[must_use]
pub fn attrs_locked_count(ids: &[String]) -> usize {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        ids.iter()
            .filter(|id| core.slot_layer_is_locked(id))
            .count()
    })
}

/// Attributes Transform commit — `update_slot_position` (x/y clamp to terrain bounds, rotation
/// normalizes, manual z sticks) + the shared post-change tail (A4: one commit = one undo step).
///
/// T-082 (wave-102 F-7) — a slot the core will REFUSE (transform-locked layer) no longer fires the
/// tail. `did` used to be "the ops context and the document both exist", which is not the same
/// question as "did anything change": a refused write still bumped `doc_ver`, marked the mission
/// DIRTY and armed a persist for an edit that never happened. The UI half of F-7 is the disabled
/// affordance the modal draws from [`attrs_locked_count`]; this is the state half.
///
/// **T-775 — "did the number change?" is answered by the FIELD, not here, and that is deliberate.**
/// This function has no equality skip: `did` means "the core did not refuse the slot", so every call
/// that reaches it rewrites `position` and fires `after_local_edit()`. That made a focus/blur on an
/// untouched coordinate a real edit — a dirty mission, an armed persist and an undo step for
/// nothing. The guard belongs at `attributes::number_field`'s `commit`, which is the only place that
/// knows the settled value the operator started from.
///
/// It is not merely convenient to guard there, it is the only correct place: this layer cannot see
/// what the operator started from — it is handed a number, not an edit. "Same x" and "same document"
/// were also different questions here for a second reason that no longer holds: an x-only call used
/// to terrain-follow the slot's z to `0.0`, so it changed the document even when `x` was unchanged.
///
/// **wave-127 F-2 — an x/y edit no longer discards an authored z.** The comment that used to sit here
/// said the zeroed z was "DEM re-sampled JS-side"; that is FALSE for this path — nothing follows an
/// Attributes commit to re-sample, so the `0.0` was final and an authored rooftop z died under a 1 m
/// X nudge. [`keep_z_rows`] explains the fix and why it lives at this caller instead of in the core
/// mutator.
pub fn attrs_update_position(
    id: &str,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        // T-649 — was an inline copy of `terrain_bounds_of` (T-650 added the identical helper
        // below); both this and the multi commit now resolve the clamp through the one function so
        // they cannot drift apart.
        if core.slot_layer_is_locked(id) {
            return false; // the core would skip this write; do not report it as an edit
        }
        // wave-127 F-2 — an x/y edit carries the slot's CURRENT z back in, so the core's
        // terrain-follow cannot flatten an authored one. See `keep_z_rows`.
        let z = z.or_else(|| keep_z_rows(core, x, y, z).and_then(|rows| slot_z(&rows, id)));
        let b = terrain_bounds_of(core);
        core.update_slot_position(id, x, y, z, rotation, b[2], b[3]);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
}

/// T-649 ATTR-MULTI-001 — the Transform commit applied to EVERY id in `ids`.
///
/// Field-by-field, exactly like the single-slot [`attrs_update_position`]: a `None` argument is a
/// field the operator did not opt in (its checkbox is unticked), and the slot mutator leaves those
/// columns untouched — so ticking "Rotation" and typing a heading can never also stamp one slot's
/// X onto the rest of the selection.
///
/// **Undo (T-732):** one LOCAL txn via [`MissionDocCore::update_entity_transforms`] = **one** undo
/// step for the whole stamp (N Ctrl+Z for N slots was the pre-T-732 defect). Still fires **one**
/// history/persist tail (`after_local_edit` once, below).
pub fn attrs_update_position_multi(
    ids: &[String],
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
) {
    if ids.is_empty() || (x.is_none() && y.is_none() && z.is_none() && rotation.is_none()) {
        return;
    }
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let b = terrain_bounds_of(core);
        // wave-127 F-2 — read ONCE for the whole stamp, then hand each member its own z back. An X
        // stamp across a selection used to flatten every member's authored z in one undo step.
        let rows = keep_z_rows(core, x, y, z);
        // T-082 (F-7) — `update_entity_transforms` returns how many patches wrote; locked / unknown
        // ids are skipped inside the one txn, so an all-locked selection yields 0 and skips the tail.
        let mut patches: Vec<EntityTransformPatch> = Vec::with_capacity(ids.len());
        for id in ids {
            if core.slot_layer_is_locked(id) {
                continue;
            }
            let z = z.or_else(|| rows.as_ref().and_then(|r| slot_z(r, id)));
            patches.push(EntityTransformPatch {
                id: id.clone(),
                is_slot: true,
                x,
                y,
                z,
                rotation,
            });
        }
        core.update_entity_transforms(&patches, b[2], b[3]) > 0
    });
    if did {
        crate::mission_history::after_local_edit();
    }
}

/// T-648 XFORM-SHIFT-001 — rotate the whole selection to FACE the cursor `(cx, cy)` (world metres),
/// each entity about its OWN position, quantised to the rotation ladder rung `rung`
/// ([`crate::mission_editor::transform`]). This is the commit end of the Shift+drag gesture and the
/// widget rotate ring.
///
/// Returns whether anything rotated (nothing selected, or every entity sitting exactly under the
/// cursor, is a no-op — [`crate::mission_editor::transform::bearing_to_face`] returns `None` for a
/// degenerate aim and that entity is left untouched).
///
/// **Undo (T-732):** commits through [`MissionDocCore::rotate_entities`] — one LOCAL txn — so a
/// multi-selection rotate is **one** Ctrl+Z, matching the single-entity case. The whole gesture still
/// fires **one** history/persist tail (`after_local_edit` once below).
pub fn rotate_selection_to_face(cx: f64, cy: f64, rung: usize) -> bool {
    if !cx.is_finite() || !cy.is_finite() {
        return false;
    }
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        if sel.is_empty() {
            return false;
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let soa = core.materialize();
        let veh_root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok();
        let terrain = veh_root
            .as_ref()
            .and_then(|v| v.get("meta")?.get("terrain")?.as_str().map(str::to_string))
            .unwrap_or_default();
        let tb = map_engine_core::mission::compile::terrain_bounds(&terrain);
        let mut items: Vec<(String, bool, f64)> = Vec::new();
        for id in &sel {
            if let Some(row) = soa.ids.iter().position(|s| s == id) {
                let (sx, sy) = (f64::from(soa.xs[row]), f64::from(soa.ys[row]));
                if let Some(bearing) =
                    crate::mission_editor::transform::bearing_to_face(sx, sy, cx, cy)
                {
                    let deg = crate::mission_editor::transform::snap_rotate(bearing, rung);
                    items.push((id.clone(), true, deg));
                }
                continue;
            }
            let Some(pos) = veh_root
                .as_ref()
                .and_then(|r| r.get("vehiclesById")?.get(id)?.get("position").cloned())
            else {
                continue;
            };
            let (Some(vx), Some(vy)) = (
                pos.get("x").and_then(serde_json::Value::as_f64),
                pos.get("y").and_then(serde_json::Value::as_f64),
            ) else {
                continue;
            };
            if let Some(bearing) = crate::mission_editor::transform::bearing_to_face(vx, vy, cx, cy)
            {
                let deg = crate::mission_editor::transform::snap_rotate(bearing, rung);
                items.push((id.clone(), false, deg));
            }
        }
        core.rotate_entities(&items, tb[2], tb[3]) > 0
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/* ═══════════════════════════════ T-645 — Placement helpers ═══════════════════════════════════════ */
//
// The wasm wiring for the Placement Tools. Every entry point here:
//   1. reads the LIVE selection's positions (slots off the materialized SoA, vehicles off
//      `small_maps_json` — the exact two sources `rotate_selection_to_face` reads),
//   2. computes target positions/yaws with the DOM-free pure math in `crate::place_helpers`
//      (natively golden-tested),
//   3. CONFIRMS via `confirm_with_message` when the op moves MORE THAN 10 entities, and
//   4. commits through [`MissionDocCore::update_entity_transforms`] / [`MissionDocCore::rotate_entities`]
//      (T-732 — one LOCAL txn = one undo step), then fires ONE `after_local_edit` history/persist tail.
//
// ── UNDO (T-732) ─────────────────────────────────────────────────────────────────────────────────
// Pattern / align / space / orient / Shift-rotate all share the atomic batch API. A 50-entity
// circular apply is ONE Ctrl+Z. The >10 confirm says so out loud (aligned with T-693's merge toast).

/// One selected entity resolved to its kind + current world position, for the placement math.
struct SelPos {
    id: String,
    /// `true` = slot; `false` = vehicle (both commit via the T-732 atomic batch APIs).
    is_slot: bool,
    x: f64,
    y: f64,
    /// The VEHICLE z a reposition preserves, read exact off `vehiclesById`.
    ///
    /// wave-127 F-5 — for a SLOT this is the f32 SoA column, so [`commit_positions`] must NOT commit
    /// it: widening `f32` back to `f64` would rewrite an authored z as a slightly different number on
    /// every align/space/pattern. The slot's z is resolved from the raw row ([`slot_z`]) at commit
    /// time instead. It is still carried here because the placement math and the callers read it.
    z: f64,
}

/// Resolve the live selection into `(SelPos list, terrain [x0,y0,w,h])`. Slots come off the
/// materialized SoA (widening the `f32` columns to f64 — the `Math.fround` store boundary); vehicles
/// come off `small_maps_json` `vehiclesById`. Ids in the selection that are neither (objects, or a
/// stale id) are dropped — placement acts on the transformable slot+vehicle set, the same scope as
/// `rotate_selection_to_face`. Returns an empty list when nothing resolves.
fn resolve_selection_positions(core: &MissionDocCore, sel: &[String]) -> (Vec<SelPos>, [f64; 4]) {
    let soa = core.materialize();
    let veh_root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok();
    let tb = terrain_bounds_of(core);
    let mut out = Vec::with_capacity(sel.len());
    for id in sel {
        if let Some(row) = soa.ids.iter().position(|s| s == id) {
            out.push(SelPos {
                id: id.clone(),
                is_slot: true,
                x: f64::from(soa.xs[row]),
                y: f64::from(soa.ys[row]),
                z: f64::from(soa.zs[row]),
            });
            continue;
        }
        if let Some(pos) = veh_root
            .as_ref()
            .and_then(|r| r.get("vehiclesById")?.get(id)?.get("position").cloned())
        {
            let (Some(vx), Some(vy)) = (
                pos.get("x").and_then(serde_json::Value::as_f64),
                pos.get("y").and_then(serde_json::Value::as_f64),
            ) else {
                continue;
            };
            out.push(SelPos {
                id: id.clone(),
                is_slot: false,
                x: vx,
                y: vy,
                z: pos
                    .get("z")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
            });
        }
    }
    (out, tb)
}

/// Ask the operator to confirm a bulk rearrangement when it moves more than the destructive
/// threshold (`place_helpers::needs_confirm`). Returns `true` to proceed. Below the threshold there
/// is no prompt (returns `true`). The wasm build shows a real `window().confirm(...)`; a native build
/// (no `window`) proceeds — the confirm is a UI guard, not a correctness gate, and the native path is
/// test-only. `verb` names the op in the prompt ("apply the Circular pattern to").
///
/// T-732 — the prompt states the honest one-step undo (atomic batch API), matching T-693's merge
/// toast form. Loadout bulk ops still N-step and use [`confirm_bulk_n_step`] instead.
#[cfg(target_arch = "wasm32")]
fn confirm_bulk(n: usize, verb: &str) -> bool {
    if !crate::place_helpers::needs_confirm(n) {
        return true;
    }
    let msg = format!("This will {verb} {n} entities. Continue? (Ctrl+Z undoes the whole op.)");
    web_sys::window()
        .and_then(|w| w.confirm_with_message(&msg).ok())
        .unwrap_or(false)
}

/// Confirm for bulk ops that are still N undo steps (loadout apply/remove — no atomic batch yet).
#[cfg(target_arch = "wasm32")]
fn confirm_bulk_n_step(n: usize, verb: &str) -> bool {
    if !crate::place_helpers::needs_confirm(n) {
        return true;
    }
    let msg = format!("This will {verb} {n} entities. Continue?");
    web_sys::window()
        .and_then(|w| w.confirm_with_message(&msg).ok())
        .unwrap_or(false)
}

/// Commit a set of target positions (index-aligned with `entities`) through
/// [`MissionDocCore::update_entity_transforms`] — **one** LOCAL txn = **one** undo step (T-732).
/// Slots: x/y clamped to `[0,w]×[0,h]`, authored z carried through. Vehicles: z + existing heading
/// preserved (a move does not re-orient). Returns whether anything committed.
///
/// **wave-127 F-5 — a placement command no longer flattens an authored slot z.** Every slot write
/// here is an x/y write; the sticky z is resolved via [`slot_z`] / [`keep_z_rows`] and passed in so
/// the mutator cannot terrain-follow to `pz = 0.0`.
///
/// The rows are read ONCE for the whole batch, not per entity: this commits `k` entities and
/// [`raw_slot_rows`] is an O(document) JSON parse. `keep_z_rows` is asked with the FIRST moved slot's
/// write shape (x and y set, z absent).
fn commit_positions(
    core: &MissionDocCore,
    entities: &[SelPos],
    targets: &[crate::place_helpers::Pt],
    tb: [f64; 4],
) -> bool {
    let z_rows = entities
        .iter()
        .zip(targets.iter())
        .find(|(e, t)| e.is_slot && (e.x != t.x || e.y != t.y))
        .and_then(|(_, t)| keep_z_rows(core, Some(t.x), Some(t.y), None));
    let mut patches: Vec<EntityTransformPatch> = Vec::new();
    for (e, t) in entities.iter().zip(targets.iter()) {
        if e.x == t.x && e.y == t.y {
            continue;
        }
        if e.is_slot {
            let z = z_rows.as_ref().and_then(|rows| slot_z(rows, &e.id));
            patches.push(EntityTransformPatch {
                id: e.id.clone(),
                is_slot: true,
                x: Some(t.x),
                y: Some(t.y),
                z,
                rotation: None,
            });
        } else {
            let heading = vehicle_heading_of(core, &e.id).unwrap_or(0.0);
            patches.push(EntityTransformPatch {
                id: e.id.clone(),
                is_slot: false,
                x: Some(t.x.clamp(0.0, tb[2])),
                y: Some(t.y.clamp(0.0, tb[3])),
                z: Some(e.z),
                rotation: Some(heading),
            });
        }
    }
    core.update_entity_transforms(&patches, tb[2], tb[3]) > 0
}

/// Read a vehicle's current heading (rotation) off `small_maps_json`; `None` if absent/unplaced.
fn vehicle_heading_of(core: &MissionDocCore, id: &str) -> Option<f64> {
    let root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok()?;
    root.get("vehiclesById")?
        .get(id)?
        .get("position")?
        .get("rotation")?
        .as_f64()
}

/// T-645 (PLACE-PATTERN-001) — apply a placement PATTERN to the live selection, LIVE (in place). The
/// pattern re-arranges the selection's positions; each entity keeps its identity and rotation. `kind`
/// is the pattern selector; `Fill Area` seeds its deterministic scatter from the selection ids
/// (`place_helpers::seed_from_ids`), so the same selection scatters the same way (reproducible).
///
/// Confirms when moving > 10 entities. Returns whether anything moved (a selection of `< 2`, or a
/// pattern that lands every entity where it already is, moves nothing). **Undo: `k` steps for `k`
/// Undo (T-732): one step for the whole selection.**
pub fn apply_pattern_to_selection(kind: crate::place_helpers::PatternKind) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let (entities, tb) = resolve_selection_positions(core, &sel);
        if entities.len() < 2 {
            return false;
        }
        let src: Vec<crate::place_helpers::Pt> = entities
            .iter()
            .map(|e| crate::place_helpers::Pt::new(e.x, e.y))
            .collect();
        let targets = match kind {
            crate::place_helpers::PatternKind::Circular => {
                crate::place_helpers::pattern_circular(&src)
            }
            crate::place_helpers::PatternKind::Line => crate::place_helpers::pattern_line(&src),
            crate::place_helpers::PatternKind::Grid => crate::place_helpers::pattern_grid(&src),
            crate::place_helpers::PatternKind::FillArea => {
                let ids: Vec<String> = entities.iter().map(|e| e.id.clone()).collect();
                let seed = crate::place_helpers::seed_from_ids(&ids);
                crate::place_helpers::pattern_fill_area(&src, seed)
            }
        };
        #[cfg(target_arch = "wasm32")]
        if !confirm_bulk(
            entities.len(),
            &format!("apply the {} pattern to", kind.label()),
        ) {
            return false;
        }
        commit_positions(core, &entities, &targets, tb)
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-645 (PLACE-ALIGN-001) — align the live selection to one of the six edges/centres
/// (`place_helpers::AlignEdge`). Confirms when moving > 10. Returns whether anything moved. Undo:
/// one step for the whole selection (T-732).
pub fn align_selection(edge: crate::place_helpers::AlignEdge) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let (entities, tb) = resolve_selection_positions(core, &sel);
        if entities.len() < 2 {
            return false;
        }
        let src: Vec<crate::place_helpers::Pt> = entities
            .iter()
            .map(|e| crate::place_helpers::Pt::new(e.x, e.y))
            .collect();
        let targets = crate::place_helpers::align_edge(&src, edge);
        #[cfg(target_arch = "wasm32")]
        if !confirm_bulk(entities.len(), "align") {
            return false;
        }
        commit_positions(core, &entities, &targets, tb)
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-645 (PLACE-SPACE-001) — space the live selection equally along one of the three axes
/// (`place_helpers::SpaceAxis`). Confirms when moving > 10. Returns whether anything moved. Undo:
/// one step for the whole selection (T-732).
pub fn space_selection(axis: crate::place_helpers::SpaceAxis) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let (entities, tb) = resolve_selection_positions(core, &sel);
        if entities.len() < 3 {
            return false; // space-equally needs at least 3 (2 are already "spaced")
        }
        let src: Vec<crate::place_helpers::Pt> = entities
            .iter()
            .map(|e| crate::place_helpers::Pt::new(e.x, e.y))
            .collect();
        let targets = crate::place_helpers::space_equally(&src, axis);
        #[cfg(target_arch = "wasm32")]
        if !confirm_bulk(entities.len(), "space") {
            return false;
        }
        commit_positions(core, &entities, &targets, tb)
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-645 (PLACE-ORIENT-001) — orient the live selection under one of the six commands
/// (`place_helpers::Orient`): N/E/S/W set an absolute yaw; face-centre/face-away turn each entity
/// toward/away from the selection centroid. Rotates IN PLACE (no move) via
/// [`MissionDocCore::rotate_entities`] (T-732 — one LOCAL txn). An entity sitting exactly on the
/// centroid declines a FACE command (`orient_yaw` → `None`) and is left unchanged; cardinals always
/// apply.
///
/// Confirms when re-orienting > 10 entities. Returns whether anything rotated.
/// **Undo (T-732): one step for the whole selection.**
pub fn orient_selection(cmd: crate::place_helpers::Orient) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let (entities, tb) = resolve_selection_positions(core, &sel);
        if entities.is_empty() {
            return false;
        }
        #[cfg(target_arch = "wasm32")]
        if !confirm_bulk(entities.len(), "re-orient") {
            return false;
        }
        let pivot = crate::place_helpers::centroid(
            &entities
                .iter()
                .map(|e| crate::place_helpers::Pt::new(e.x, e.y))
                .collect::<Vec<_>>(),
        );
        let mut items: Vec<(String, bool, f64)> = Vec::new();
        for e in &entities {
            let Some(deg) = crate::place_helpers::orient_yaw(
                cmd,
                crate::place_helpers::Pt::new(e.x, e.y),
                pivot,
            ) else {
                continue; // degenerate face (entity on the centroid) → leave unchanged
            };
            items.push((e.id.clone(), e.is_slot, deg));
        }
        core.rotate_entities(&items, tb[2], tb[3]) > 0
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// Read a slot's embedded `loadout` JSON (Arsenal picks) from `slots_json`. `None` when unset.
pub fn read_loadout(id: &str) -> Option<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let map: serde_json::Value = serde_json::from_str(&core.slots_json()).ok()?;
        let lo = map.get(id)?.get("loadout")?;
        if lo.is_null() {
            return None;
        }
        Some(lo.to_string())
    })
}

thread_local! {
    /// T-068.15.2 — per-character default cargo (registry `character_default_cargo`
    /// edges, aggregated). Filled by the editor's compat fetch; consumed by the
    /// seed hooks (place / apply-kit / Arsenal open).
    static CARGO_DEFAULTS: RefCell<HashMap<String, Vec<crate::arsenal_rules::CargoRow>>> =
        RefCell::new(HashMap::new());
}

/// Install the character → default-cargo map (from the `/registry/compat` fetch).
pub fn set_cargo_defaults(map: HashMap<String, Vec<crate::arsenal_rules::CargoRow>>) {
    CARGO_DEFAULTS.with(|c| *c.borrow_mut() = map);
}

/// Seed one slot's cargo inside an already-open doc borrow (shared by the place /
/// apply-kit hooks — the caller owns the history tail). Seeds only when the
/// character has defaults and the loadout carries no `cargo` key.
fn seed_cargo_in_core(
    core: &MissionDocCore,
    id: &str,
    asset_id: &str,
    loadout: Option<&str>,
) -> bool {
    let defaults = CARGO_DEFAULTS.with(|c| c.borrow().get(asset_id).cloned());
    let Some(defaults) = defaults else {
        return false;
    };
    match crate::arsenal_rules::seed_cargo(loadout, &defaults) {
        // T-779 — the SINK's answer, not a hardcoded `true`. `update_slot_loadout` returns `false`
        // for an id the document does not hold (T-770), and "a seed was computed" is a different
        // claim from "the document took it". Every current caller discards this bool, which is
        // precisely why it had to stop lying: the next reader would inherit a flag that was only
        // ever right by accident.
        Some(json) => core.update_slot_loadout(id, Some(json)),
        None => false,
    }
}

/// Arsenal-open seed (pre-.15.2 slots): own ctx scope + history tail. Returns the
/// seeded loadout JSON so the caller can render it without a re-read.
pub fn seed_slot_cargo(id: &str) -> Option<String> {
    let seeded = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let map: serde_json::Value = serde_json::from_str(&core.slots_json()).ok()?;
        let slot = map.get(id)?;
        let asset_id = slot.get("assetId")?.as_str().filter(|s| !s.is_empty())?;
        let loadout = slot
            .get("loadout")
            .filter(|l| !l.is_null())
            .map(|l| l.to_string());
        let defaults = CARGO_DEFAULTS.with(|c| c.borrow().get(asset_id).cloned())?;
        let json = crate::arsenal_rules::seed_cargo(loadout.as_deref(), &defaults)?;
        // T-779 — `Some(json)` only if the DOCUMENT took the write. The `map.get(id)?` above makes
        // a refusal hard to reach today, but the tail below (`after_local_edit`) mints an undo step
        // and dirties the mission off this `Option` alone, so it must carry the sink's answer
        // rather than the fact that we got this far.
        core.update_slot_loadout(id, Some(json.clone()))
            .then_some(json)
    });
    if seeded.is_some() {
        crate::mission_history::after_local_edit();
    }
    seeded
}

/// Set/clear a slot's `loadout` (Arsenal commit) + the shared tail (one undo step). `None`/empty
/// clears the key.
///
/// **Returns whether the DOCUMENT took the write** (T-779). Until this slice `did` was a hardcoded
/// `true` sitting directly under `core.update_slot_loadout(…);`, which threw away the `bool` T-770
/// had just added to the mutator for exactly this purpose. Two things were wrong with that:
///
/// 1. The tail fired whenever [`OPS_CTX`] and the doc merely EXISTED. A write against a stale or
///    deleted slot id — the Arsenal holds the id it opened with, and that entity can be deleted or
///    undone out from under the modal — still dirtied the mission and minted an undo step over a
///    document that had not changed. Ctrl+Z then had a step that restored nothing.
/// 2. Nothing built on this path could tell a real write from a no-op, which is the whole defect
///    T-770 was filed to close. `arsenal::commit_writes` (the multi-write sibling) counts the sink
///    and gets this right; the single-write path did not.
///
/// The `bool` is returned rather than swallowed because the operator has to be able to learn about
/// a refusal: `ArsenalTab`'s persistence line would otherwise render its green "no unsaved changes"
/// verdict over a pick that never landed. See `arsenal.rs`'s `persist` closure.
///
/// The gate itself lives in [`crate::arsenal::commit_one_write`] and not in an `if` here, because
/// this module is `cfg(target_arch = "wasm32")` from line one and a native test cannot reach it —
/// the same reason T-770 put the batch loop in `arsenal::commit_writes`. There the refusal→no-tail
/// arithmetic is *driven* by `arsenal::tests::t779`; here it could only be read.
pub fn set_loadout(id: &str, loadout_json: Option<String>) -> bool {
    crate::arsenal::commit_one_write(
        || {
            OPS_CTX.with(|c| {
                let guard = c.borrow();
                let Some(ctx) = guard.as_ref() else {
                    return false;
                };
                let d = ctx.doc.borrow();
                let Some(core) = d.as_ref() else {
                    return false;
                };
                core.update_slot_loadout(id, loadout_json)
            })
        },
        || {
            crate::mission_history::after_local_edit();
        },
    )
}

/* ═════ T-699 (3DEN-LOAD-001 / -002 / -010) — the loadout BUFFER: Copy · Apply · Remove Everything ═════ */

thread_local! {
    /// The loadout buffer. Deliberately a **snapshot of bytes**, not a list of source ids: T-687's
    /// loadout INHERITANCE was cancelled by the operator, and a buffer that stored ids would have to
    /// re-read the sources at Apply time, which is inheritance with extra steps. Copy takes the
    /// bytes; the sources can then be edited, or deleted, and the buffer neither knows nor cares.
    ///
    /// Separate from `CLIPBOARD` on purpose. That one holds whole slot dicts for Ctrl+C/Ctrl+V and
    /// a copied *entity* is a different thing from a copied *kit*; sharing one cell would mean
    /// Ctrl+C silently destroying a loadout buffer the author was in the middle of using.
    static LOADOUT_BUFFER: RefCell<Vec<crate::arsenal::BufferedLoadout>> =
        const { RefCell::new(Vec::new()) };

    /// The Apply seed stream. Fixed start + a fixed step (see `next_apply_seed`), so the Nth Apply
    /// of a session always draws the Nth assignment: pressing Apply twice re-rolls, and a bug report
    /// that names a press replays exactly. No clock, no JS RNG — see `arsenal::buffer_draw` for the
    /// full argument.
    static APPLY_SEED: std::cell::Cell<u64> = const { std::cell::Cell::new(0x2545_F491_4F6C_DD1D) };
}

/// Advance the session's Apply seed and return the value this Apply draws with. The step is the
/// SplitMix64 gamma, so consecutive Applies land in unrelated parts of the stream rather than in
/// adjacent ones. Advances on every ATTEMPT, including a refused one — burning a seed costs nothing
/// and the alternative (advance only on success) makes "the third Apply" ambiguous in exactly the
/// bug reports the fixed stream exists to make replayable.
fn next_apply_seed() -> u64 {
    APPLY_SEED.with(|s| {
        let now = s.get();
        s.set(now.wrapping_add(0x9E37_79B9_7F4A_7C15));
        now
    })
}

/// The selection, filtered to slot ids, read in one doc borrow. Vehicles and objects are dropped:
/// `loadout` is a slot column, so counting a vehicle would report "applied to 5" over 4 documents.
fn selection_slot_targets() -> Vec<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        selected_slot_ids(core, &sel)
    })
}

/// **Copy** (3DEN-LOAD-001) — buffer the loadout of EVERY selected entity, not just one. Returns how
/// many were buffered.
///
/// An entity with no `loadout` key buffers as a `None` — a bare kit is a real thing to copy, and
/// dropping those would silently make a 5-entity Copy a 3-entity buffer. A Copy that resolves to
/// **nothing** (empty selection, or a selection of vehicles only) leaves the previous buffer intact
/// rather than clearing it: an accidental click must not destroy work the author is mid-way through
/// using, and "0 copied" is already said in the receipt.
pub fn copy_loadouts_from_selection() -> usize {
    let buffered = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
            return Vec::new();
        };
        selected_slot_ids(core, &sel)
            .into_iter()
            .map(|id| {
                let loadout_json = map
                    .get(&id)
                    .and_then(|s| s.get("loadout"))
                    .filter(|l| !l.is_null())
                    .map(ToString::to_string);
                crate::arsenal::BufferedLoadout {
                    source_id: id,
                    loadout_json,
                }
            })
            .collect()
    });
    let n = buffered.len();
    if n > 0 {
        LOADOUT_BUFFER.with(|b| *b.borrow_mut() = buffered);
    }
    n
}

/// What is in the buffer right now (for the panel's label and its receipt).
pub fn loadout_buffer() -> Vec<crate::arsenal::BufferedLoadout> {
    LOADOUT_BUFFER.with(|b| b.borrow().clone())
}

/// How many loadouts are buffered — the affordance the Apply button is enabled on.
pub fn loadout_buffer_len() -> usize {
    LOADOUT_BUFFER.with(|b| b.borrow().len())
}

/// **Apply** (3DEN-LOAD-002) — write the buffer onto the selection, drawing one buffered loadout per
/// entity at random. `Ok((planned, committed))` is how many writes the plan held and how many the
/// document actually took; `Err` is the refusal list, and a refusal writes **nothing at all**. Both
/// numbers are returned rather than just the second so the caller's receipt can say when they
/// disagree instead of quietly reporting the optimistic one.
///
/// The gate is `arsenal::plan_apply`, which is T-686's import gate over the whole buffer — the same
/// three rule passes, run before a die is rolled so the verdict cannot depend on the draw. Planning
/// is complete before the write borrow opens, so there is no path on which half a plan reaches the
/// document because the other half was refused.
///
/// ⚠️ **N ENTITIES IS N UNDO STEPS, and this is where that is true.** `update_slot_loadout` opens one
/// Yrs transaction per call and the store's `capture_timeout_millis = 0` makes every transaction its
/// own undo step, so the loop below costs one Ctrl+Z per entity. The core has no atomic multi-entity
/// loadout write — that is **T-732**, the same wall wave 111's T-645 hit and the same one
/// `commit_positions` documents a few hundred lines up — and `store.rs` is outside this slice's
/// `owns`. The single `after_local_edit()` below is the shared post-change tail (re-materialize,
/// dock rebuild, persist), NOT an undo boundary, and conflating the two is exactly the fake
/// atomicity this comment refuses. The author is told the real number: `arsenal::apply_receipt`
/// builds its line from the returned commit count.
pub fn apply_loadout_buffer_to_selection(
    items: &[crate::dto::RegistryItem],
    feed: &crate::arsenal_rules::CompatFeed,
) -> Result<(usize, usize), Vec<crate::arsenal_rules::RowError>> {
    let buffer = loadout_buffer();
    let targets = selection_slot_targets();
    if buffer.is_empty() || targets.is_empty() {
        return Ok((0, 0));
    }
    if !confirm_bulk_n_step(targets.len(), "overwrite the loadout of") {
        return Ok((0, 0));
    }
    let writes = crate::arsenal::plan_apply(&targets, &buffer, next_apply_seed(), items, feed)?;
    Ok((writes.len(), commit_loadout_writes(&writes)))
}

/// **Remove Everything** (3DEN-LOAD-010) — strip every selected entity's loadout. Returns
/// `(planned, committed)`, for the same reason Apply does.
///
/// This is the ONE strip verb this slice builds. The nine per-category variants 3den also offers
/// (NVGs, vests, goggles, headgear, weapons, …) are marked `maybe` upstream and are deliberately out
/// of scope — see `arsenal::stripped_loadout`, which also explains why a strip writes an explicit
/// empty document rather than clearing the field (clearing it would let the cargo seed put the
/// author's magazines back).
///
/// Same N-steps-for-N-entities reality as Apply; see there and T-732.
pub fn remove_all_loadouts_from_selection() -> (usize, usize) {
    let targets = selection_slot_targets();
    if targets.is_empty() {
        return (0, 0);
    }
    if !confirm_bulk_n_step(targets.len(), "remove every item from") {
        return (0, 0);
    }
    let writes = crate::arsenal::plan_remove(&targets);
    (writes.len(), commit_loadout_writes(&writes))
}

/// The shared write path for both verbs: one doc borrow, one `update_slot_loadout` per planned
/// write, one shared post-change tail. Returns the number of writes the document actually took —
/// `arsenal::commit_writes` counts the sink, and both receipts are built from that count rather than
/// from the plan length, so a drop cannot be reported as a success.
fn commit_loadout_writes(writes: &[crate::arsenal::LoadoutWrite]) -> usize {
    if writes.is_empty() {
        return 0;
    }
    let commits = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        crate::arsenal::commit_writes(writes, |id, json| core.update_slot_loadout(id, json))
    });
    if commits > 0 {
        crate::mission_history::after_local_edit();
    }
    commits
}

/// Attributes Identity/stance commit — `update_slot(role/tag/stance)` + the shared tail.
///
/// T-082 — `asset_id` (ATTR-FIELD-OBJ-TYPE) and `description` (ATTR-FIELD-OBJ-ROLE-DESC) ride the
/// SAME commit seam under the same `None`-means-not-opted-in discipline, but land through a second
/// core mutator (`update_slot_object`) because they are not `update_slot` columns. Each is a no-op
/// when nothing in its half is `Some`, so a role keystroke opens exactly one transaction and a type
/// keystroke opens exactly one — the modal's one-commit-one-undo-step contract is unchanged.
/// (`update_slot_role_character` is deliberately NOT the writer here; see its counterpart's note on
/// `MissionDocCore::update_slot_object` for why routing a type edit through it would wipe `tag`.)
///
/// NOT gated on the transform lock, and that is the core's rule rather than an omission: T-665 locks
/// TRANSFORM only, so identity/type/description edits are legal on a locked slot.
///
/// **T-745 (wave-113 F-3)** — `did` used to mean "ops context + document both exist", so an
/// all-`None` call (or a call against a nonexistent id) still ran `after_local_edit` and armed a
/// false save. Mirror [`attrs_update_slot_multi`]: only fire the tail when something actually
/// changed. Existence is RAW (`raw_slot_rows`), not SoA — a hidden slot is still real (T-744).
pub fn attrs_update_slot(
    id: &str,
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
    asset_id: Option<String>,
    description: Option<String>,
) {
    // T-745 — nothing opted in ⇒ no write, no history/persist tail. Same shape as the multi peer
    // (T-082 widened that guard by the object fields; the single-target path lacked it).
    if role.is_none()
        && tag.is_none()
        && stance.is_none()
        && asset_id.is_none()
        && description.is_none()
    {
        return;
    }
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        // T-745 — a nonexistent id is a core no-op; do not bump doc_ver / dirty / persist.
        // RAW map, not SoA: Hide filters the SoA (T-744) but the row is still in the document.
        if !raw_slot_rows(core).contains_key(id) {
            return false;
        }
        if role.is_some() || tag.is_some() || stance.is_some() {
            core.update_slot(id, role, tag, stance);
        }
        core.update_slot_object(id, asset_id, description);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
}

/* ─────────── T-649 ATTR-MULTI-001 / ATTR-MULTI-CHK-001 — multi-selection Attributes ─────────── */

/// T-649 — the Identity/stance commit applied to EVERY id in `ids`. Peer of
/// [`attrs_update_position_multi`]; same `None`-means-not-opted-in field discipline (`update_slot`
/// leaves a `None` column alone) and the same one-tail / N-undo-steps honesty note.
pub fn attrs_update_slot_multi(
    ids: &[String],
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
    asset_id: Option<String>,
    description: Option<String>,
) {
    // Nothing opted in ⇒ no writes at all. T-082 widened this guard by the two new fields: a commit
    // that opts into NEITHER half must stay a no-op, not become N transactions of `None`.
    if ids.is_empty()
        || (role.is_none()
            && tag.is_none()
            && stance.is_none()
            && asset_id.is_none()
            && description.is_none())
    {
        return;
    }
    let slot_half = role.is_some() || tag.is_some() || stance.is_some();
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        for id in ids {
            if slot_half {
                core.update_slot(id, role.clone(), tag.clone(), stance.clone());
            }
            // T-082 — the type / role-description half of the same fan-out, same per-field
            // `Option` discipline: `None` leaves that key alone on every target.
            core.update_slot_object(id, asset_id.clone(), description.clone());
        }
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
}

/// T-649 — the slot ids the Attributes modal is editing when it opened over a MULTI-selection.
///
/// An **empty** return means single-edit, and the modal renders exactly as it always has (no
/// checkboxes anywhere). It is non-empty only when both:
///   * the live selection still contains `open_id` — `open_attrs_modal` already collapses a
///     right-click that retargeted outside the selection, so this re-check is what keeps the modal
///     honest if a dock edits the selection while it is open; and
///   * at least two of the selected ids are real slot rows.
///
/// Vehicles are filtered out on purpose: every field in this modal is a slot-SoA column
/// (x/y/z/rotation/stance/role/tag) and `vehiclesById` rows have none of them, so counting a
/// vehicle would show "N selected" while a Role write silently missed it. T-741: the modal header
/// must name this slot subset (and say vehicles are excluded when [`attrs_selection_len`] is
/// wider) — never report the full selection as the write set.
#[must_use]
pub fn attrs_multi_ids(open_id: &str) -> Vec<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let sel = ctx.selection.borrow().clone();
        if sel.len() < 2 || !sel.iter().any(|s| s == open_id) {
            return Vec::new();
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let soa = core.materialize();
        let ids: Vec<String> = sel
            .into_iter()
            .filter(|s| soa.ids.iter().any(|r| r == s))
            .collect();
        if ids.len() < 2 {
            Vec::new()
        } else {
            ids
        }
    })
}

/// T-741 — live selection length (slots + vehicles + …). Attributes multi-edit compares this to
/// [`attrs_multi_ids`]'s slot subset so the header can say when vehicles were excluded from the
/// write set (wave-112 NIT-4 / Ctrl+A via `view_ids_with_vehicles`).
#[must_use]
pub fn attrs_selection_len() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .map(|ctx| ctx.selection.borrow().len())
            .unwrap_or(0)
    })
}

/// T-649 ATTR-MULTI-CHK-001 — which Attributes fields DISAGREE across a multi-selection.
///
/// Eden's multi-edit rule has two halves. This is the first: a field whose value is identical on
/// every selected entity can show that value; a field whose values differ has no value to show, so
/// the modal blanks it and disables it until its per-field checkbox opts it in. `attributes.rs`
/// owns the second half (the checkbox + the disable).
///
/// A single `materialize()` feeds every comparison, so the flags are a consistent snapshot of one
/// doc state rather than seven independent reads. Floats compare by **bits**, not by `==`: the
/// question is "is this literally the same stored value", and bit compare answers it exactly
/// without an epsilon that would call two genuinely different headings equal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttrDiff {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub rotation: bool,
    pub stance: bool,
    pub role: bool,
    pub tag: bool,
    /// T-082 ATTR-FIELD-OBJ-TYPE — compared off the RAW rows, not the SoA (it has no such column).
    pub asset_id: bool,
    /// T-082 ATTR-FIELD-OBJ-ROLE-DESC — same, and for the same reason.
    pub description: bool,
}

impl AttrDiff {
    /// True when at least one field disagrees — the modal's "Multiple values" hint.
    #[must_use]
    pub fn any(self) -> bool {
        self.x
            || self.y
            || self.z
            || self.rotation
            || self.stance
            || self.role
            || self.tag
            || self.asset_id
            || self.description
    }
}

/// T-649 — [`AttrDiff`] for `ids`. Fewer than two resolvable rows ⇒ all-false (nothing can differ).
#[must_use]
pub fn read_attrs_diff(ids: &[String]) -> AttrDiff {
    if ids.len() < 2 {
        return AttrDiff::default();
    }
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return AttrDiff::default();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return AttrDiff::default();
        };
        let soa = core.materialize();
        // T-082 — carry the ID alongside the SoA row index. The two new fields are compared off the
        // raw slot rows (the SoA has no `assetId` / `description` column) and those are keyed by id,
        // so the SoA index alone is no longer enough to name a member of the selection. The MEMBER
        // SET is still exactly the set that resolves in the SoA, so which entities are compared is
        // unchanged — only how many columns are compared over them.
        let rows: Vec<(&String, usize)> = ids
            .iter()
            .filter_map(|id| Some((id, soa.ids.iter().position(|s| s == id)?)))
            .collect();
        let Some((&(first_id, first), rest)) = rows.split_first() else {
            return AttrDiff::default();
        };
        let raw = raw_slot_rows(core);
        // Resolve dict-coded columns to their STRINGS before comparing: `materialize()` gives no
        // guarantee that two rows carrying the same role text share an index, so an index compare
        // could report a difference the operator cannot see in the field.
        let text = |idx: u32, dict: &[String]| {
            if idx == NONE_IDX {
                String::new()
            } else {
                dict.get(idx as usize).cloned().unwrap_or_default()
            }
        };
        let mut d = AttrDiff::default();
        for &(id, r) in rest {
            d.x |= soa.xs[r].to_bits() != soa.xs[first].to_bits();
            d.y |= soa.ys[r].to_bits() != soa.ys[first].to_bits();
            d.z |= soa.zs[r].to_bits() != soa.zs[first].to_bits();
            d.rotation |= soa.rotations[r].to_bits() != soa.rotations[first].to_bits();
            d.stance |= soa.stance.get(r).copied().unwrap_or(0)
                != soa.stance.get(first).copied().unwrap_or(0);
            d.role |= text(soa.role_idx[r], &soa.roles) != text(soa.role_idx[first], &soa.roles);
            d.tag |= text(soa.tag_idx[r], &soa.tags) != text(soa.tag_idx[first], &soa.tags);
            // T-082 — absent reads back as `""` (`row_str`), so "one slot has no type and the other
            // has one" is a DIFFERENCE, which is what the operator sees in the field.
            d.asset_id |= row_str(&raw, id, "assetId") != row_str(&raw, first_id, "assetId");
            d.description |=
                row_str(&raw, id, "description") != row_str(&raw, first_id, "description");
        }
        d
    })
}

/// Read the doc's `editorLayers` as rows for the tree. There is **no** public `editor_layers`
/// accessor on the core, and `materialize()`'s `layers` dict holds layer *ids* only — the names /
/// `parentId` / `entityIds` live in `small_maps_json()`'s `editorLayersById` (`store.rs:153`).
///
/// Sorted by id so the tree order can't depend on `serde_json`'s map type (`preserve_order` or not).
fn layer_rows(core: &MissionDocCore) -> Vec<LayerRow> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return Vec::new();
    };
    let Some(map) = root.get("editorLayersById").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut rows: Vec<LayerRow> = map
        .values()
        .filter_map(|v| {
            let o = v.as_object()?;
            Some(LayerRow {
                id: o.get("id")?.as_str()?.to_string(),
                name: o
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default()
                    .to_string(),
                // `parentId` is `null` at the root (never absent) — `add_editor_layer` writes
                // `Any::Null` (`store.rs:803`).
                parent_id: o
                    .get("parentId")
                    .and_then(|p| p.as_str())
                    .map(str::to_string),
                entity_ids: o
                    .get("entityIds")
                    .and_then(|e| e.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
                // T-665 — `hidden`/`locked` are written only when true (store setters remove the key
                // on false), so an absent key is the canonical `false`.
                hidden: o
                    .get("hidden")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                locked: o
                    .get("locked")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows
}

/// T-168 — read `factionsById` from `small_maps_json()` into ORBAT faction rows.
fn faction_rows(core: &MissionDocCore) -> Vec<crate::outliner::FactionRow> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return Vec::new();
    };
    let Some(map) = root.get("factionsById").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    map.values()
        .filter_map(|v| {
            let o = v.as_object()?;
            Some(crate::outliner::FactionRow {
                id: o.get("id")?.as_str()?.to_string(),
                key: o
                    .get("key")
                    .and_then(|k| k.as_str())
                    .unwrap_or_default()
                    .to_string(),
                name: o
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default()
                    .to_string(),
                squad_ids: str_array(o.get("squadIds")),
            })
        })
        .collect()
}

/// T-168 — read `squadsById` from `small_maps_json()` into ORBAT squad rows.
fn squad_rows(core: &MissionDocCore) -> Vec<crate::outliner::SquadRow> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return Vec::new();
    };
    let Some(map) = root.get("squadsById").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    map.values()
        .filter_map(|v| {
            let o = v.as_object()?;
            Some(crate::outliner::SquadRow {
                id: o.get("id")?.as_str()?.to_string(),
                name: o
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or_default()
                    .to_string(),
                faction_id: o
                    .get("factionId")
                    .and_then(|f| f.as_str())
                    .unwrap_or_default()
                    .to_string(),
                slot_ids: str_array(o.get("slotIds")),
                leader_slot_id: o
                    .get("leaderSlotId")
                    .and_then(|l| l.as_str())
                    .unwrap_or_default()
                    .to_string(),
                vehicle_ids: str_array(o.get("vehicleIds")),
            })
        })
        .collect()
}

/// A JSON string array → `Vec<String>` (skipping non-strings). Shared by the ORBAT row readers.
fn str_array(v: Option<&serde_json::Value>) -> Vec<String> {
    v.and_then(|e| e.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Adapt the materialized SoA into the tree's slot rows (id + resolved role).
fn slot_rows(core: &MissionDocCore) -> Vec<SlotRow> {
    let soa = core.materialize();
    (0..soa.ids.len())
        .map(|i| {
            let idx = soa.role_idx[i];
            let role = if idx == NONE_IDX {
                String::new()
            } else {
                soa.roles.get(idx as usize).cloned().unwrap_or_default()
            };
            SlotRow {
                id: soa.ids[i].clone(),
                role,
            }
        })
        .collect()
}

/// Rebuild the dock mirrors from the live doc + selection. Called from
/// `mission_history::refresh_signals`, i.e. from **every** mutation site (place, drag-move, undo,
/// redo, click-select, the IDB restore swap) — so the tree can never show a stale slot set.
pub fn refresh_docks() {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let (nodes, orbat) = {
            let d = ctx.doc.borrow();
            match d.as_ref() {
                Some(core) => {
                    let slots = slot_rows(core);
                    (
                        // T-651 — the layer tree carries editor-only comment rows alongside slots.
                        // The ORBAT tree below does NOT: it is squad-scoped, and an annotation
                        // belongs to no squad.
                        build_outliner_with_comments(
                            &layer_rows(core),
                            &slots,
                            &comment_rows(core),
                        ),
                        crate::outliner::build_orbat(
                            &faction_rows(core),
                            &squad_rows(core),
                            &slots,
                        ),
                    )
                }
                None => (Vec::new(), Vec::new()),
            }
        };
        ctx.outliner_nodes.set(nodes);
        ctx.orbat_nodes.set(orbat);
        mirror_selection(ctx);
        ctx.doc_tick
            .set(ctx.doc_tick.get_untracked().wrapping_add(1));
    });
}

/// T-780 [wave 142 F-1] — push the entity selection to the dock mirror, **reconciling the map's edge
/// selection first**.
///
/// This is the only place `selected_ids` is written, and both mirrors ([`refresh_docks`] on every
/// document change, [`refresh_selection_mirrors`] on every selection-only change) go through it — so
/// no route can put an entity selection on screen without [`reconcile_connection_selection`] having
/// run against it. That is what turns "the two selections are mutually exclusive" from a statement
/// about ONE pick's ordering into a property of every selection write in the editor, including the
/// three that do not know the map exists (the Outliner row, the click-to-select router, a place).
fn mirror_selection(ctx: &OpsCtx) {
    reconcile_connection_selection(ctx);
    ctx.selected_ids.set(ctx.selection.borrow().clone());
}

/// Selection-only dock mirror: push `selected_ids` (the trees' fine-grained `is_sel` source)
/// without rebuilding the node trees. Pairs with `mission_history::refresh_selection` (T-172 B8).
pub fn refresh_selection_mirrors() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            mirror_selection(ctx);
        }
    });
}

/// Outliner slot row → select it (replacing the selection), mirroring React: "selecting a slot
/// selects it globally (no auto camera move)" (`EditorLayersSection.tsx:5`). Runs the same
/// selection-only tail a map click does — no doc edit, so no rebind / persist / undo step.
pub fn select_slot(id: String) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        *ctx.selection.borrow_mut() = vec![id];
        let ids = ctx.selection.borrow().clone();
        // NAMED, not a `borrow_mut()` temporary in the `if let`: a temporary would live to the end
        // of the closure and so drop AFTER `guard` — the borrow it reads through. A binding declared
        // after `guard` drops before it (reverse declaration order).
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_selection(ids); // tint lane
        }
    });
    crate::mission_history::refresh_selection(); // SEL + dock highlight only — no tree rebuild
}

/// Outliner folder row → make it the drop target (React's `setActiveLayer`).
pub fn set_active_layer(id: Option<String>) {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.active_layer.set(id);
        }
    });
}

/// T-665 — flip a layer's `hidden` flag (the outliner eye toggle), then the shared post-change tail
/// (one commit = one undo step; `after_local_edit` re-materializes, so a now-hidden layer's slots
/// vanish from the map and a re-shown layer's return, and rebuilds the dock so the glyph updates).
pub fn set_layer_hidden(id: &str, hidden: bool) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_editor_layer_hidden(id, hidden);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
}

/// T-665 — flip a layer's `locked` flag (the outliner lock toggle) + the shared tail. Its slots
/// (and its subtree's) then refuse a move at the store level; the tree rebuild re-marks the rows.
pub fn set_layer_locked(id: &str, locked: bool) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_editor_layer_locked(id, locked);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
}

/* ═══════════════════════ T-701 — per-entity editor visibility override ═══════════════════════ */
//
// 3den E9 "Enable Visibility" — an editor-LOCAL `editorHidden` flag on the slot row that NEVER
// compiles (it rides `editor.slots`, the editor-only block reloaded verbatim, and is structurally
// stripped from the MOD wire by `flatten`'s `SlotIn`/`ModSlot` — see
// `store::set_slot_editor_hidden`). Distinct from the T-665 LAYER eye (per-LAYER, on a folder row):
// this is PER-ENTITY, so a maker can declutter a single dense-area entity without hiding its layer.
// Enforcement is at `store::materialize`, where effective-hidden = `layer-hidden OR entity-hidden`.
//
// These are thin wrappers onto the SHIPPED, TESTED store mutators (`set_slots_editor_hidden` — the
// per-entity ONE-TXN batch, so hiding the whole selection is ONE undo step; `clear_all_editor_hidden`
// — the reveal-all, one txn), riding the SAME post-change tail as the layer eye
// (`after_local_edit` → `refresh_docks`): the eye and the H-key flip visibility with the SAME
// semantics (one commit = one undo step; re-materialize drops/returns the affected slots), so there
// is NO new inconsistency between the two affordances.
//
// PENDING PRODUCT DECISION (T-715, amended wave 104), INHERITED not widened: a hidden slot vanishes
// from the dock trees (the `slot_rows` feed reads `materialize`, which now also drops entity-hidden
// slots) and selection consumers can act on a hidden entity sight-unseen. Entity-hidden JOINS
// layer-hidden in exactly that same open lane — this slice does not fix it and must not widen it.
//
// UI RESIDUE (a wiring line for T-733's family — NOT this slice's `owns`): the context-menu Hide/Show
// row (`context_menu.rs` + the T-664 enum), the H-key on the selection (`mission_editor` keydown),
// and the dock hidden-glyph render (`eden_tree`) all live in files OUTSIDE these two owned modules.
// What ships visibly here: this ops-level API + the `slot_hidden_rows` accessor a dock CAN render
// once wired. The keyboard/menu/glyph are the residue.

/// Filter a selection to the ids that are SLOTS (present in `slotsById`), keeping HIDDEN ones —
/// unlike a `materialize()`-based resolve, this reads `slots_json` so an entity that is currently
/// editor-hidden is still resolvable (you must be able to SHOW what you hid). Non-slot ids (vehicles,
/// objects, or a stale id) are dropped; the visibility flag lives on the slot row, matching the
/// store's slot-only `materialize` filter (vehicles/entities have no SoA filter this slice).
fn selected_slot_ids(core: &MissionDocCore, sel: &[String]) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return Vec::new();
    };
    let Some(map) = root.as_object() else {
        return Vec::new();
    };
    sel.iter()
        .filter(|id| map.contains_key(id.as_str()))
        .cloned()
        .collect()
}

/// Read whether the current selection is ALL-hidden (used to decide the toggle direction and to drive
/// a menu row's checked state). Returns `None` when the selection resolves to no slots (nothing to
/// toggle). `Some(true)` ⇒ every selected slot is hidden (so a toggle SHOWS); `Some(false)` ⇒ at
/// least one is visible (a toggle HIDES, matching Eden's "any visible → hide all" bias).
fn selection_all_hidden(core: &MissionDocCore, ids: &[String]) -> Option<bool> {
    if ids.is_empty() {
        return None;
    }
    let root = serde_json::from_str::<serde_json::Value>(&core.slots_json()).ok()?;
    let map = root.as_object()?;
    Some(ids.iter().all(|id| {
        map.get(id)
            .and_then(|s| s.get("editorHidden"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }))
}

/// T-701 (3den E9) — set `editorHidden` on the whole live SELECTION in ONE undo step, then the shared
/// post-change tail (re-materialize drops/returns the affected slots, dock rebuild re-marks the rows).
/// `hidden = true` hides, `false` shows. Rides `store::set_slots_editor_hidden` — the per-entity
/// one-txn batch — so the H-key over a multi-selection is one Eden action, NOT one step per slot
/// (contrast the T-732 position lane, which lacks such a batch). Returns whether anything was flipped
/// (an empty selection, or a selection with no slots, is a no-op). Shared by the H-key affordance and
/// a context-menu Hide/Show row once those are wired (T-733's family).
fn set_selection_hidden(hidden: bool) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let ids = selected_slot_ids(core, &sel);
        if ids.is_empty() {
            return false;
        }
        core.set_slots_editor_hidden(&ids, hidden);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-701 — HIDE the live selection (declutter). One undo step; returns whether anything was hidden.
///
/// `#[allow(dead_code)]`: the canonical ops API. Its live caller — an H-key on the selection — lives
/// in `mission_editor`'s keydown, and a context-menu Hide row in `context_menu.rs`; both are OUTSIDE
/// this slice's `owns` (the H-key + menu are the stated T-733-family residue). Shipped tested here.
#[allow(dead_code)]
pub fn hide_selection() -> bool {
    set_selection_hidden(true)
}

/// T-701 — SHOW (un-hide) the live selection. One undo step; returns whether anything was shown.
///
/// `#[allow(dead_code)]` — same residue note as [`hide_selection`] (the context-menu Show row / the
/// H-key toggle live outside `owns`).
#[allow(dead_code)]
pub fn show_selection() -> bool {
    set_selection_hidden(false)
}

/// T-701 — TOGGLE `editorHidden` on the live selection (the H-key affordance's natural verb): if every
/// selected slot is already hidden, SHOW them; otherwise HIDE them all (Eden's "any visible → hide
/// all" bias, so a mixed selection collapses to hidden). One undo step. Returns whether anything
/// flipped (no-op on an empty / slot-less selection).
///
/// `#[allow(dead_code)]`: this is the verb the H-key affordance calls (T-648 keydown idiom in
/// `mission_editor`, out of `owns`). Shipped + covered by the store batch/undo tests.
#[allow(dead_code)]
pub fn toggle_hidden() -> bool {
    // Decide the direction under a read borrow, then delegate to the one-txn setter. Reading the
    // direction and committing under separate borrows is fine: this is single-threaded editor state
    // and nothing mutates the selection between the two.
    let dir = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let ids = selected_slot_ids(core, &sel);
        // all-hidden → show (flip to visible); else → hide.
        selection_all_hidden(core, &ids).map(|all_hidden| !all_hidden)
    });
    match dir {
        Some(hidden) => set_selection_hidden(hidden),
        None => false,
    }
}

/// T-701 — SHOW ALL: clear `editorHidden` on EVERY slot in the doc in ONE undo step (the reveal-all
/// command, so a maker who hid several entities across the mission un-hides them all at once). Rides
/// `store::clear_all_editor_hidden` (one txn) + the shared tail. Returns the number of entities
/// un-hidden (0 ⇒ nothing was hidden, and no visible change).
///
/// `#[allow(dead_code)]`: a menu/command entry point (a "Show All" item) is the residue; the reveal-
/// all txn + undo are proven at the store (`show_all_clears_every_flag_in_one_txn`).
#[allow(dead_code)]
pub fn show_all_hidden() -> usize {
    let cleared = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        core.clear_all_editor_hidden()
    });
    if cleared > 0 {
        crate::mission_history::after_local_edit();
    }
    cleared
}

/// T-701 — dock-facing accessor: every slot's `(id, editorHidden)` read straight off `slots_json`
/// (`slotsById`), so it lists HIDDEN slots too — the twin of [`layer_rows`]'s `hidden` field but
/// per-ENTITY. `slot_rows` (fed from `materialize`) deliberately DROPS hidden slots (they leave the
/// tree, the T-715 inherited lane), so a dock that wants to render a hidden entity dimmed-but-present
/// (an "eye-off" glyph, the Eden affordance) reads THIS instead. Sorted by id for deterministic order
/// (mirrors `layer_rows`). Pure over the doc — the render wiring (glyph, click-to-show) is the
/// T-733-family residue, but the DATA a dock needs ships here.
///
/// `#[allow(dead_code)]`: the consuming dock render (an eye-off glyph on the slot row) is outside
/// `owns` (`eden_tree`); this accessor is the shippable-visible datum it will read once wired.
#[allow(dead_code)]
#[must_use]
pub fn slot_hidden_rows(core: &MissionDocCore) -> Vec<(String, bool)> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return Vec::new();
    };
    let Some(map) = root.as_object() else {
        return Vec::new();
    };
    let mut rows: Vec<(String, bool)> = map
        .iter()
        .map(|(id, v)| {
            let hidden = v
                .get("editorHidden")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            (id.clone(), hidden)
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

/* ═══════════════════════════ T-666 — Outliner layer authoring ═══════════════════════════ */
//
// LAYER-CREATE-001 / LAYER-DEL-001 / SEL-LAYER-CHILDREN-001 / SEL-LAYER-DESC-001 (the ops half;
// SEL-GROUP-ICON-001 is a pure render rule in `eden_tree`). These are thin wrappers onto the
// SHIPPED, TESTED core layer mutators (`add_editor_layer` / `rename_editor_layer` /
// `remove_editor_layer` (subtree + reseed) / `reparent_editor_layer` (cycle-guarded) /
// `move_slot_to_layer`) — verified at filing to have ZERO UI callers. Each wrapper opens exactly
// one `OPS_CTX` borrow, scopes the doc write so it drops before the tail, and rides
// `mission_history::after_local_edit()` — which is what calls `refresh_docks()` (via
// `refresh_signals`), so "call core + refresh_docks" is one tail, exactly like `set_layer_hidden`.
// The core mutators each commit a SINGLE transaction, so one authoring action = one undo step.

thread_local! {
    /// Monotonic minter for created-layer ids; uniqueness is still PROVEN against the live doc in
    /// [`mint_layer_id`] (undo frees ids; an IDB restore can bring back a doc that already used one).
    static NEXT_LAYER_ID: Cell<u32> = const { Cell::new(0) };
    /// LAYER-CREATE-001 — the id of the layer just created, so the tree can arm its inline rename
    /// immediately (spec: "inline-rename armed immediately"). The dock reads+clears this reactively.
    static RENAME_ARMED: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Armed drag between a tree-row `pointerdown` and a folder/root-dropzone `pointerup`
    /// (pointer-drag — the same idiom as [`PENDING_REFILE`], not HTML5 DnD, since the Leptos
    /// frontend has no HTML5-DnD lane). A folder row arms `Folder` (drop reparents); a slot row in
    /// the layer tree arms `Slot` (drop refiles). One latch so a folder's `pointerup` dispatches
    /// either without a second source of truth to get out of step.
    static PENDING_LAYER_DRAG: RefCell<Option<LayerDrag>> = const { RefCell::new(None) };
}

/// T-666 — what a tree pointer-drag is carrying (see [`PENDING_LAYER_DRAG`]).
#[derive(Clone)]
enum LayerDrag {
    /// A folder being reparented.
    Folder(String),
    /// A slot being refiled into a folder.
    Slot(String),
    /// T-651 — an editor-only COMMENT being refiled into a folder. A separate variant rather than
    /// reusing [`LayerDrag::Slot`] because the completion calls a different mutator: a comment id is
    /// a `commentsById` key, and `move_slot_to_layer` would happen to work today (it only shuffles
    /// `entityIds`) but would silently become wrong the moment slot refiling grows a squad or
    /// selection side effect. The variant makes the two intents unmistakable at the drop site.
    Comment(String),
}

/// Mint an unused `layer-{n}` id, proven unique against the doc's live layer set.
fn mint_layer_id(core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        layer_rows(core).into_iter().map(|l| l.id).collect();
    loop {
        let id = format!("layer-{}", NEXT_LAYER_ID.with(Cell::get));
        NEXT_LAYER_ID.with(|c| c.set(c.get().saturating_add(1)));
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// Auto-name a new layer "New Layer N" where N is the smallest positive integer not already used by
/// an existing "New Layer …" name (so creating three in a row reads 1/2/3, and a delete-then-create
/// reuses the gap). Names need not be unique in the doc; this is only a friendly default.
fn mint_layer_name(core: &MissionDocCore) -> String {
    let used: std::collections::HashSet<u32> = layer_rows(core)
        .iter()
        .filter_map(|l| {
            l.name
                .strip_prefix("New Layer ")?
                .trim()
                .parse::<u32>()
                .ok()
        })
        .collect();
    let mut n = 1u32;
    while used.contains(&n) {
        n += 1;
    }
    format!("New Layer {n}")
}

/// LAYER-CREATE-001 — create a folder as a **child of the selected/active folder** (or a root when
/// none is active), auto-named "New Layer N", and arm its inline rename. Returns the new id.
///
/// Rides the shipped `add_editor_layer`; one transaction ⇒ one undo step. The parent is the active
/// layer if it still exists (a stale pointer — folder deleted/undone-away — falls back to root,
/// mirroring `ensure_layer`'s staleness handling).
pub fn create_layer() -> Option<String> {
    let created = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let id = {
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            let rows = layer_rows(core);
            // Parent = the active folder if it still exists, else root (None).
            let parent = ctx
                .active_layer
                .get_untracked()
                .filter(|a| rows.iter().any(|l| &l.id == a));
            let id = mint_layer_id(core);
            let name = mint_layer_name(core);
            core.add_editor_layer(&id, &name, parent);
            id
        };
        // Make the new folder the active drop target + arm its inline rename.
        ctx.active_layer.set(Some(id.clone()));
        RENAME_ARMED.with(|r| *r.borrow_mut() = Some(id.clone()));
        Some(id)
    });
    if created.is_some() {
        crate::mission_history::after_local_edit();
    }
    created
}

/// LAYER-CREATE-001 — take the id of the just-created layer whose inline rename should open, if any.
/// Consumed once (cleared on read) so the dock arms the input exactly once per creation.
#[must_use]
pub fn take_rename_armed() -> Option<String> {
    RENAME_ARMED.with(|r| r.borrow_mut().take())
}

/// Rename an Outliner folder (inline-rename commit). Rides the shipped `rename_editor_layer`; one
/// transaction ⇒ one undo step. A blank name after trim is rejected (a folder must keep a label).
pub fn rename_layer(id: &str, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.rename_editor_layer(id, name);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// LAYER-DEL-001 — delete a folder with the SHIPPED subtree semantics: `remove_editor_layer`
/// deletes the folder AND its whole subtree (child folders + every slot filed in any of them), keeps
/// ≥1 layer (reseeding a default if the subtree was every layer). One transaction ⇒ one undo step.
///
/// This is destructive (the whole subtree), which is why the dock guards it behind a confirm whose
/// text says so. The reseed id is minted here so a "delete the only layer" reseed can't collide.
pub fn delete_layer(id: &str) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            let reseed = mint_layer_id(core);
            core.remove_editor_layer(id, &reseed);
        }
        // If the active drop target was inside the removed subtree it is now dangling; the next
        // place's `ensure_layer` re-resolves, but clear it eagerly so the header reads honestly.
        if ctx.active_layer.get_untracked().as_deref() == Some(id) {
            ctx.active_layer.set(None);
        }
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// Reparent a folder (drag-in-tree / root-dropzone). Rides the cycle-guarded `reparent_editor_layer`
/// (a drop into the folder's own subtree is a no-op at the core), so this wrapper does not re-check
/// cycles. `new_parent = None` moves it to the root. One transaction ⇒ one undo step.
pub fn reparent_layer(id: &str, new_parent: Option<String>) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.reparent_editor_layer(id, new_parent);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// Refile a slot into a different folder (drag a slot row onto a folder). Rides the shipped
/// `move_slot_to_layer` (detach from every folder holding it, append to the target); squad is
/// unchanged (workflow-only). One transaction ⇒ one undo step.
pub fn refile_slot_to_layer(slot_id: &str, layer_id: &str) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            core.move_slot_to_layer(slot_id, layer_id);
        }
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/* ═══════════════════ T-651 — editor comments / annotations (PLACE-COMMENT-001) ═══════════════════ */
//
// A comment is an EDITOR-ONLY VIRTUAL ENTITY. It shows in the Outliner, files into a layer, drags
// and copies like any other row — and it NEVER COMPILES. That last part is not enforced here and
// deliberately so: the exclusion is structural in `map-engine-core`
// (`mission::flatten::EditorPayload` declares no `comments` key, so serde drops the array before the
// mod document exists), proven by `comments_never_reach_the_mod_document`. Nothing in this file
// filters anything, which is why nothing in this file can forget to.
//
// CORPUS HONESTY: this feature is evidenced by ONE community across TWO eras — FNF v3's 28 in-map
// comments (including a seven-paragraph tutorial) and the TWO that survived v4's total rewrite. WOG
// and OFCRA have no comment equivalent. It is not a four-way convergence and must not be sold as
// one; the template seed is sized to the surviving evidence (two), not the peak (28).

/// T-651 — one comment as the docks need it. Mirrors [`crate::outliner::CommentRow`] plus the world
/// position the tree does not need but a drag/copy caller does.
#[derive(Clone, Debug, PartialEq)]
pub struct CommentDetail {
    pub id: String,
    pub title: String,
    pub tooltip: String,
    pub x: f64,
    pub z: f64,
}

/// T-651 — read `commentsById` into tree rows, sorted by id so the Unfiled bucket's order cannot
/// depend on `serde_json`'s map type (the `layer_rows` rule).
fn comment_rows(core: &MissionDocCore) -> Vec<CommentRow> {
    comment_details(core)
        .into_iter()
        .map(|d| CommentRow {
            id: d.id,
            title: d.title,
            tooltip: d.tooltip,
        })
        .collect()
}

/// T-651 — every comment with its position, off the narrow [`MissionDocCore::comments_json`] getter.
#[must_use]
pub fn comment_details(core: &MissionDocCore) -> Vec<CommentDetail> {
    let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.comments_json()) else {
        return Vec::new();
    };
    let Some(obj) = map.as_object() else {
        return Vec::new();
    };
    let mut rows: Vec<CommentDetail> = obj
        .iter()
        .map(|(id, v)| {
            let pos = |k: &str| {
                v.get("position")
                    .and_then(|p| p.get(k))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0)
            };
            let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
            CommentDetail {
                id: id.clone(),
                title: s("title"),
                tooltip: s("tooltip"),
                x: pos("x"),
                z: pos("z"),
            }
        })
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows
}

/// T-651 — every comment in the live doc (dock/read helper).
#[must_use]
pub fn comment_list() -> Vec<CommentDetail> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        d.as_ref().map(comment_details).unwrap_or_default()
    })
}

/// T-651 — placed-comment count.
#[must_use]
pub fn comment_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.doc.borrow().as_ref().map(MissionDocCore::comment_count))
            .unwrap_or(0)
    })
}

/// Mint an unused `cmt-{n}` id, proven unique against the live comments map.
///
/// A separate counter namespace from [`mint_id`]'s `n{…}` on purpose: comment ids share the
/// `editorLayers[].entityIds` array with slot ids, and `build_outliner_with_comments` resolves a
/// filed id by looking it up in the slot map first and the comment map second. Disjoint prefixes
/// make that lookup unambiguous by construction rather than by luck.
fn mint_comment_id(core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&core.comments_json())
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    let mut n = existing.len() + 1;
    loop {
        let id = format!("cmt-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n += 1;
    }
}

/// T-651 (`PLACE-COMMENT-001`) — **place a comment at world `(x, z)`**, the RMB-on-empty →
/// "Place Comment" gesture. Files it under the resolved active layer ([`ensure_layer`], the same
/// resolution a unit place uses) so it lands where the operator is working rather than in Unfiled.
/// Returns the new comment id.
///
/// The default title/tooltip are placeholders an operator overwrites; they are non-empty so the new
/// row is visible and clickable the instant it appears (the `SLOT_FALLBACK_LABEL` reasoning).
///
/// This does NOT touch the selection. A comment id is not a slot id, and pushing it into the
/// selection lane would put a non-slot into `set_selection` (engine tint), `delete_selection` and
/// the SEL readout — three surfaces that all index the slot SoA and would find nothing.
pub fn place_comment(x: f64, z: f64) -> Option<String> {
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let id = mint_comment_id(core);
        let layer_id = ensure_layer(ctx, core);
        core.add_comment(&id, "Comment", "", x, z);
        core.move_comment_to_layer(&id, &layer_id);
        Some(id)
    })?;
    crate::mission_history::after_local_edit();
    Some(id)
}

/// T-651 (ATTR-FIELD-CMT-TITLE) — retitle a comment (inline edit).
pub fn rename_comment(id: String, title: String) -> bool {
    edit_comment(|core| core.set_comment_title(&id, &title))
}

/// T-651 (ATTR-FIELD-CMT-TOOLTIP) — rewrite a comment's tooltip body (inline edit).
pub fn set_comment_tooltip(id: String, tooltip: String) -> bool {
    edit_comment(|core| core.set_comment_tooltip(&id, &tooltip))
}

/// T-651 (ATTR-FIELD-CMT-POSITION) — **DRAG commit**: move a comment to world `(x, z)`. One core
/// transaction ⇒ one undo step, so a drag is one Ctrl+Z exactly like a slot move.
pub fn move_comment(id: String, x: f64, z: f64) -> bool {
    edit_comment(|core| core.set_comment_position(&id, x, z))
}

/// T-651 — **COPY**: duplicate a comment `offset` metres to the south-east, keeping title and
/// tooltip. Returns the new id, or `None` when the source is gone. One undo step.
///
/// The offset exists so the copy is not perfectly stacked on its source: two comments at identical
/// coordinates are one indistinguishable row in the tree and one unclickable glyph on any future
/// map render.
pub fn duplicate_comment(id: &str, offset: f64) -> Option<String> {
    let new_id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let new_id = mint_comment_id(core);
        if !core.duplicate_comment(id, &new_id, offset, -offset) {
            return None;
        }
        // Land the copy in the same folder as its source, else the resolved active layer. A copy
        // that silently jumped to another folder would be a refile the operator never asked for.
        let layer_id = layer_rows(core)
            .into_iter()
            .find(|l| l.entity_ids.iter().any(|e| e == id))
            .map_or_else(|| ensure_layer(ctx, core), |l| l.id);
        core.move_comment_to_layer(&new_id, &layer_id);
        Some(new_id)
    })?;
    crate::mission_history::after_local_edit();
    Some(new_id)
}

/// T-651 — delete a comment (also unfiles it from its folder — see `remove_comment`).
pub fn delete_comment(id: String) -> bool {
    edit_comment(|core| core.remove_comment(&id))
}

/// T-651 — **LAYERS**: file a comment into `layer_id` (the outliner drag drop). Mirrors
/// [`refile_slot_to_layer`]; one transaction ⇒ one undo step.
pub fn refile_comment_to_layer(comment_id: &str, layer_id: &str) -> bool {
    edit_comment(|core| core.move_comment_to_layer(comment_id, layer_id))
}

/// Shared edit tail for the comment mutators: run `f` against the core, then the dirty tail (one
/// undo step). Returns `false` when there is no doc. The [`edit_composition`] idiom.
fn edit_comment(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-651 — seed the NEW-MISSION TEMPLATE's comments into the freshly-minted doc, under the `INIT`
/// origin so the template is not an undo step (the boot/seed contract). No-op on any doc that
/// already carries a comment, so a restore or a server hydrate can never be given a second copy.
///
/// Called once from `mission_editor`'s boot, BEFORE the IndexedDB restore and the server hydrate —
/// both of which replace the document wholesale, which is exactly right: a restored or downloaded
/// mission is not a new mission and gets whatever comments it was saved with.
pub fn seed_new_mission_template(doc: &DocHandle) -> usize {
    let borrowed = doc.borrow();
    let Some(core) = borrowed.as_ref() else {
        return 0;
    };
    core.set_origin_init(true);
    let ids = core.seed_template_comments();
    core.set_origin_init(false);
    ids.len()
}

/* ═══════ T-672 — the editor-only CONNECTION GRAPH (CONN-START/SYNC/DEL-001, ACTION-FORM-001) ════ */
//
// A connection is an EDITOR-ONLY relation between two placed things. Like a comment it NEVER
// COMPILES, and — as with comments — that is not enforced HERE and deliberately so: the exclusion is
// structural in `map-engine-core` (`mission::flatten::EditorPayload` declares no `connections` key,
// and `mission.schema.json` declares no relation collection for one to land in), proven by
// `connections_never_reach_the_mod_document`. Nothing in this file filters anything, which is why
// nothing in this file can forget to.
//
// ── THE SHAPE OF THE CONNECT GESTURE, AND WHY IT IS NOT A DRAG ───────────────────────────────────
// The ticket's code hint proposed a third pointer mode: drag from entity to entity, arming in
// `mission_editor`'s `onpointerdown` and completing in `onpointerup`. That is NOT what ships here,
// and the reason is recorded rather than left to be rediscovered.
//
// The armed-pointerup path in `mission_editor.rs` is KNOWN DEFECTIVE and is filed as T-723: it has
// no `ev.button()` filter (so a middle- or right-button release fires the armed branch), it returns
// without taking `left`, stranding an `LG::Pending` that a later bare pointermove promotes into a
// phantom drag, and there is no Esc disarm at all. The invariant comment sitting beside it claiming
// "`left`/`pan_px` are both None here" is FALSE and was refuted four waves ago. Building a THIRD
// arming mode on that machine would inherit all three defects and add a fourth arming source to the
// thing that already cannot decide who owns a release.
//
// So the connect CREATE flow starts as two context-menu acts (T-672): right-click the source →
// `Connect ▸` → a kind, which arms; right-click the target → `Connect ▸ Complete` (or `Cancel`).
// T-768 wires the Eden LMB-target half on top of T-723's fixed armed-pointerup machine: after arming,
// a sub-threshold LMB pick on an entity calls [`complete_connect`] — the SAME mutator the RMB
// Complete row uses. No new LeftGesture variant, no new Pending place-arm: only a second CALLER.
// Esc / pointercancel / RMB Cancel / panel Cancel disarm. CONN-DEL line-select still needs a
// connections render lane (`mission_history` rebind + `draw_order`) — panel Delete remains the
// disclosed substitute.
//
// ── SEE + CHECK COME FIRST ───────────────────────────────────────────────────────────────────────
// [`connection_list`] and [`connection_findings`] are the halves the ticket's warning is about, and
// they are what the Connections panel in `mission_editor` renders. A connection has no map glyph in
// this slice (the `LaneRole::SquadLinks` trace is in the slice notes), so that panel is the ONLY
// surface on which an operator can observe or audit the graph. It is not a nice-to-have attached to
// the edge verbs; the edge verbs are attached to it.

/// T-672 — one connection as the panel renders it: the doc row plus a resolved label per endpoint.
///
/// Labels are best-effort (`"SL (s0)"` for a slot whose role is known, the bare id otherwise) and are
/// display-only — every VERB here takes the `id`, so a label that cannot be resolved degrades the
/// row's readability and nothing else. An unresolvable endpoint is exactly the `CONN-DANGLING` case
/// the findings list flags by id, which is why the label does not try to hide it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionListRow {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub from_label: String,
    pub to_label: String,
}

/// T-672 — one validation finding for the panel (`code` / `connection_id` / `detail`), mirroring
/// `map-engine-core`'s `ConnectionFinding` across the JSON getter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionFindingRow {
    pub code: String,
    pub connection_id: String,
    pub detail: String,
}

thread_local! {
    /// T-672 — the Connections panel's open flag, installed once from `mission_editor::on_load`.
    /// The [`COMMENT_EDITOR`] idiom: a self-contained overlay owned by the page, read by its
    /// component and written only here.
    static CONNECTIONS_PANEL: RefCell<Option<RwSignal<bool>>> = const { RefCell::new(None) };
}

/// T-672 — register the Connections-panel signal (called once from `mission_editor::on_load`).
pub fn set_connections_panel_signal(sig: RwSignal<bool>) {
    CONNECTIONS_PANEL.with(|s| *s.borrow_mut() = Some(sig));
}

/// T-672 — open the Connections panel (the SEE + CHECK surface). No-op if the signal was never
/// registered (native shell, or before `on_load`).
pub fn open_connections_panel() {
    CONNECTIONS_PANEL.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(true);
        }
    });
}

/// T-672 — close the Connections panel.
pub fn close_connections_panel() {
    CONNECTIONS_PANEL.with(|s| {
        if let Some(sig) = *s.borrow() {
            sig.set(false);
        }
    });
}

thread_local! {
    /// T-780 [wave 142 F-1] — the connection edge SELECTED on the map (`None` = none). The signal is
    /// owned by `mission_editor` (it is page-local overlay state, not document content) and handed
    /// here through the [`CONNECTIONS_PANEL`] idiom.
    ///
    /// T-780 claimed the edge selection and the entity selection were "mutually exclusive by
    /// construction". They were not. The construction was the MAP pick's own ordering — an edge is
    /// only picked on a miss, and a miss clears the entity selection — and it says nothing about the
    /// other routes into a selection: the Outliner row ([`select_slot`]), the marquee commit, the
    /// click-to-select router, a place. Through any of those both selections were live at once, and
    /// Delete over a highlighted slot removed an amber line somewhere else instead.
    ///
    /// So the claim is MADE TRUE here rather than asserted in a comment: the selection is registered
    /// with the module that owns every entity-selection write, and [`reconcile_connection_selection`]
    /// runs inside [`mirror_selection`] — the one function through which an entity selection reaches
    /// the UI at all.
    static CONNECTION_SELECTION: RefCell<Option<RwSignal<Option<String>>>> =
        const { RefCell::new(None) };
}

/// T-780 [wave 142 F-1] — register the map's connection selection (once, from
/// `mission_editor::on_load`). No registration ⇒ every verb below is inert, which is the native
/// shell's case and the pre-mount case.
pub fn set_connection_selection_signal(sig: RwSignal<Option<String>>) {
    CONNECTION_SELECTION.with(|s| *s.borrow_mut() = Some(sig));
}

/// T-780 [wave 142 F-1] — **the reconcile.** Drop the map's edge selection when it can no longer be
/// the thing Delete removes, for either of the two reasons it stops being that:
///
/// * an ENTITY selection is live — the operator is looking at a highlighted slot, so the amber line
///   is a promise about a keypress that now belongs to the slot. This is the exclusivity, enforced.
/// * the id no longer names an edge in the DOCUMENT — an undo of the connect, a panel-side delete,
///   the T-672 endpoint cascade, an IDB restore. A stale id is not merely an inert tint: it is what
///   let Delete report a removal over a document that never changed, which is the T-779 defect
///   (success over a write that did not happen) rebuilt on a different surface.
///
/// Read through `try_get_untracked`, which answers `None` on a DISPOSED signal (a route-leave that
/// outran a refresh), and NOTHING is written in that case — the T-778 rule: a write onto a signal
/// whose owner is gone is not a reconcile, it is a lie with no reader.
fn reconcile_connection_selection(ctx: &OpsCtx) {
    CONNECTION_SELECTION.with(|s| {
        let Some(sig) = *s.borrow() else {
            return;
        };
        let Some(Some(id)) = sig.try_get_untracked() else {
            return;
        };
        // Both borrows are released before the write: the set re-runs the lane Effect, which reads
        // the same doc handle.
        let entity_selected = !ctx.selection.borrow().is_empty();
        let still_there = {
            let d = ctx.doc.borrow();
            d.as_ref()
                .is_some_and(|core| connection_id_in_doc(core, &id))
        };
        if entity_selected || !still_there {
            sig.set(None);
        }
    });
}

/// T-780 [wave 142 F-2] — does the live document actually hold an edge with this id?
///
/// Asked over `connection_rows_json`, the SAME stable listing the panel renders and the map lane is
/// built from, so "present" means one thing to the verb, to the reconcile and to the operator's
/// eyes. The core's `remove_connection` returns unit and so cannot answer "was it there?"; this
/// takes the answer BEFORE the write instead of inferring it from a count afterwards.
fn connection_id_in_doc(core: &MissionDocCore, id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.connection_rows_json()) else {
        return false;
    };
    rows.as_array().is_some_and(|a| {
        a.iter()
            .any(|r| r.get("id").and_then(serde_json::Value::as_str) == Some(id))
    })
}

/// T-780 [wave 142 F-1] — the same question [`delete_connection`] gates on, asked from the map's
/// Delete arm so a stale selection FALLS THROUGH to the entity delete instead of being handed to a
/// verb that can only answer `false`. One question, one implementation, two callers.
#[must_use]
pub fn connection_exists(id: &str) -> bool {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        d.as_ref()
            .is_some_and(|core| connection_id_in_doc(core, id))
    })
}

thread_local! {
    /// T-672 / T-768 — the armed half of the connect: `Some((kind, from_id))`.
    ///
    /// A plain `thread_local` and NOT a variant of place-arm `Pending`: place-arm is consumed by the
    /// palette stamp path. Connect stays separate so RMB Complete/Cancel and the LMB pick CALLER
    /// (T-768) both read the same cell without riding the place-arm enum. `mission_editor`'s
    /// LG::Pending click path and Esc/pointercancel consult this cell; they do not promote it into
    /// LeftGesture state.
    static PENDING_CONNECT: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

/// T-672 (`CONN-START-001`, act 1) — arm a connect of `kind` FROM `from_id`. Returns `false`
/// (arming nothing) for an unknown kind or an empty source, so a menu row that somehow dispatched
/// with a bad payload cannot leave the editor half-armed.
///
/// Re-arming REPLACES any previous arm rather than stacking: the operator changed their mind about
/// the source or the kind, and a queue of pending connects is not a thing anyone asked for.
pub fn arm_connect(kind: &str, from_id: &str) -> bool {
    // The AUTHORITY on this vocabulary is `MissionDocCore::add_connection`, which refuses an unknown
    // kind into the document. This check is about the ARM (a UI state, not a document one): see
    // `context_menu::ConnKind::parse`'s note for why the editor keeps its own copy and why the two
    // cannot diverge dangerously.
    if from_id.is_empty() || crate::context_menu::ConnKind::parse(kind).is_none() {
        return false;
    }
    PENDING_CONNECT.with(|p| {
        *p.borrow_mut() = Some((kind.to_string(), from_id.to_string()));
    });
    true
}

/// T-672 — the armed connect, if any: `(kind, from_id)`. Read by `context_menu::open` so the menu it
/// paints shows `Complete` / `Cancel` instead of the three kind rows — the arm is captured AT OPEN,
/// the same rule `MenuTarget::world` follows, so a state change while the menu is up cannot make a
/// visible row mean something else by the time it is clicked.
#[must_use]
pub fn pending_connect() -> Option<(String, String)> {
    PENDING_CONNECT.with(|p| p.borrow().clone())
}

/// T-672 — drop an armed connect without writing anything.
pub fn cancel_connect() {
    PENDING_CONNECT.with(|p| *p.borrow_mut() = None);
}

/// T-672 (`CONN-START-001` / `CONN-SYNC-001`, act 2) — complete the armed connect onto `to_id`.
/// `false` when nothing was armed or the core refused the edge (self-link, duplicate, empty
/// endpoint — see `MissionDocCore::add_connection`).
///
/// **The arm is consumed on ATTEMPT, not on success.** A refused edge that left the connect armed
/// would leave the operator in a mode they thought they had exited, and their very next right-click
/// would silently mean "connect" instead of "open a menu" — a stranded arm is the T-723 defect
/// shape, and this feature is not going to reproduce it one module over.
pub fn complete_connect(to_id: &str) -> bool {
    let Some((kind, from_id)) = PENDING_CONNECT.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    if to_id.is_empty() {
        return false;
    }
    let drawn = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let id = mint_connection_id(core);
        core.add_connection(&id, &kind, &from_id, to_id)
    });
    if drawn {
        crate::mission_history::after_local_edit();
    }
    drawn
}

/// T-672 (`CONN-DEL-001`) — delete one edge by id. This is the verb the panel's per-row button
/// calls, and it is the whole of `CONN-DEL-001` in this slice: Eden deletes a connection by selecting
/// its LINE and pressing Del, and there is no line to select here (no render lane — see the slice
/// notes), so the addressable row IS the selection. One core transaction ⇒ one Ctrl+Z.
///
/// **The returned `bool` means "this connection was there and is now gone"** [wave 142 F-2]. It used
/// to mean something weaker and wrong: the guard was `connection_count() == 0`, a COUNT rather than
/// an id-presence check, so an id the document did not hold returned `true` whenever any OTHER edge
/// existed — `after_local_edit` then dirtied a mission that never changed. That is the same class
/// T-779 removed from `set_loadout` in the same wave (a verb inventing an acknowledgement for a
/// write that did not land), and T-780's map selection made it reachable: an undo, or a panel-side
/// delete, leaves the map holding an id the graph no longer has.
///
/// `MissionDocCore::remove_connection` returns unit and cannot report what it removed, so the answer
/// is taken from the document BEFORE the write ([`connection_id_in_doc`]) instead of guessed after
/// it. If the core mutator ever grows a `bool` like `add_connection` has, this gate becomes its
/// return value and the pre-read goes away.
pub fn delete_connection(id: &str) -> bool {
    let removed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        if !connection_id_in_doc(core, id) {
            return false;
        }
        core.remove_connection(id);
        true
    });
    if removed {
        crate::mission_history::after_local_edit();
    }
    removed
}

/// T-672 (SEE) — every connection in the live doc, in `map-engine-core`'s stable listing order, with
/// endpoint labels resolved off the slot rows.
#[must_use]
pub fn connection_list() -> Vec<ConnectionListRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let labels: std::collections::HashMap<String, String> = slot_rows(core)
            .into_iter()
            .map(|r| {
                let label = if r.role.is_empty() {
                    r.id.clone()
                } else {
                    format!("{} ({})", r.role, r.id)
                };
                (r.id, label)
            })
            .collect();
        let label_of = |id: &str| labels.get(id).cloned().unwrap_or_else(|| id.to_string());
        let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.connection_rows_json())
        else {
            return Vec::new();
        };
        rows.as_array()
            .map(|a| {
                a.iter()
                    .map(|r| {
                        let s =
                            |k: &str| r.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let (from, to) = (s("from"), s("to"));
                        ConnectionListRow {
                            from_label: label_of(&from),
                            to_label: label_of(&to),
                            id: s("id"),
                            kind: s("kind"),
                            from,
                            to,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// T-672 (CHECK) — every validation finding over the live graph. The panel renders these beside the
/// rows; `connection_id` joins them to [`connection_list`].
#[must_use]
pub fn connection_findings() -> Vec<ConnectionFindingRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.connection_findings_json())
        else {
            return Vec::new();
        };
        rows.as_array()
            .map(|a| {
                a.iter()
                    .map(|f| {
                        let s =
                            |k: &str| f.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        ConnectionFindingRow {
                            code: s("code"),
                            connection_id: s("connectionId"),
                            detail: s("detail"),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// T-672 (`ACTION-FORM-001` / `CTX-FORMATION-001`) — snap the squad led by `leader_slot_id` onto its
/// formation positions. Returns the number of units moved; 0 (and no undo step) when the id leads no
/// squad, so firing the row on a rifleman is inert rather than a silent leadership change.
///
/// The geometry and the single-transaction guarantee live in `MissionDocCore::force_to_formation`;
/// this is the thin editor-side call plus the history tick, so the action is one Ctrl+Z.
pub fn force_to_formation(leader_slot_id: &str, formation: &str) -> usize {
    let moved = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        core.force_to_formation(leader_slot_id, formation)
    });
    if moved > 0 {
        crate::mission_history::after_local_edit();
    }
    moved
}

/// T-672 — mint an unused `conn-{n}` id, proven unique against the live connections map. A separate
/// counter namespace from [`mint_id`] / [`mint_comment_id`] for the same reason theirs are separate:
/// disjoint prefixes make "what kind of thing is this id" answerable by construction.
fn mint_connection_id(core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&core.connections_json())
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    let mut n = existing.len() + 1;
    loop {
        let id = format!("conn-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n += 1;
    }
}

// ── Pointer-drag reparent/refile in the tree (the T-037-era TreeView-DnD role, on the current
//    pointer idiom — mirrors ORBAT's `begin_refile`/`complete_refile_onto_squad`). A folder row
//    arms on `pointerdown`; dropping onto another folder reparents, onto the header root-dropzone
//    reparents to root, and a slot row reuses the same latch to refile.

/// Arm a folder for a pointer-drag reparent (folder-row `pointerdown`).
pub fn begin_layer_drag(layer_id: String) {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Folder(layer_id)));
}

/// Arm a slot for a pointer-drag refile into a folder (slot-row `pointerdown`, layer tree only).
pub fn begin_layer_slot_drag(slot_id: String) {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Slot(slot_id)));
}

/// T-651 — arm a COMMENT for a pointer-drag refile into a folder (comment-row `pointerdown`). This
/// is "comments support drag" in the tree: the same latch, the same folder-row `pointerup`
/// completion, a different mutator ([`refile_comment_to_layer`]).
pub fn begin_layer_comment_drag(comment_id: String) {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Comment(comment_id)));
}

/// Drop an armed drag ANYWHERE that isn't a valid target (clear without mutating).
pub fn cancel_layer_drag() {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = None);
}

/// Complete an armed drag onto `dest_folder_id`: a FOLDER drag reparents under it (no-op self/subtree
/// drop — the core rejects it); a SLOT drag refiles into it. `false` when nothing was armed.
pub fn complete_layer_drop_onto_folder(dest_folder_id: String) -> bool {
    let Some(drag) = PENDING_LAYER_DRAG.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    match drag {
        LayerDrag::Folder(id) => {
            if id == dest_folder_id {
                return false;
            }
            reparent_layer(&id, Some(dest_folder_id))
        }
        LayerDrag::Slot(slot_id) => refile_slot_to_layer(&slot_id, &dest_folder_id),
        // T-651 — a comment files into a folder exactly like a slot (same `entityIds` array).
        LayerDrag::Comment(comment_id) => refile_comment_to_layer(&comment_id, &dest_folder_id),
    }
}

/// Complete an armed FOLDER drag by reparenting it to the ROOT (the header dropzone). A slot drag is
/// dropped (a slot must live in some folder; "refile to no folder" is not a thing the doc models —
/// it would just leave the slot unfiled, and the root dropzone is a folder-reparent affordance).
/// `false` when nothing was armed or a slot was armed.
pub fn complete_layer_drop_onto_root() -> bool {
    let drag = PENDING_LAYER_DRAG.with(|p| p.borrow_mut().take());
    match drag {
        Some(LayerDrag::Folder(id)) => reparent_layer(&id, None),
        _ => false,
    }
}

// ── Folder-click selection (SEL-LAYER-CHILDREN-001 / SEL-LAYER-DESC-001). Reads the UNFILTERED
//    doc (`layer_rows` → `entity_ids`), NOT `materialize()`: see `eden_tree`'s module note — a
//    folder-click must select what the DOC says the layer contains, so a hidden layer still
//    selects its slots (the T-715 lane). No doc edit ⇒ selection-only tail, no undo step.

/// SEL-LAYER-CHILDREN-001 — select a folder's DIRECT slot children (replacing the selection).
pub fn select_layer_children(layer_id: &str) {
    let ids = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        Some(crate::eden_tree::layer_direct_slot_children(
            &layer_rows(core),
            layer_id,
        ))
    });
    if let Some(ids) = ids {
        set_slot_selection(ids);
    }
}

/// SEL-LAYER-DESC-001 — select every slot in a folder's whole subtree (replacing the selection).
pub fn select_layer_descendants(layer_id: &str) {
    let ids = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        Some(crate::eden_tree::layer_descendant_slots(
            &layer_rows(core),
            layer_id,
        ))
    });
    if let Some(ids) = ids {
        set_slot_selection(ids);
    }
}

/// Replace the slot selection with `ids` and run the selection-only tail a map/outliner click takes
/// (engine tint + SEL + dock highlight; no doc edit ⇒ no rebind/persist/undo). Shared by the two
/// folder-click selectors above.
fn set_slot_selection(ids: Vec<String>) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        *ctx.selection.borrow_mut() = ids;
        let ids = ctx.selection.borrow().clone();
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_selection(ids);
        }
    });
    crate::mission_history::refresh_selection();
}

/// Palette leaf `pointerdown` → arm a place. Consumed by [`place_at`] on a canvas release, or
/// dropped by [`cancel_pending`] on a release over chrome.
///
/// T-180.5 — no-op while the Objects chip is active (stub catalog; place must not panic).
pub fn begin_place(payload: PlacePayload) {
    arm(Pending::Character(payload));
}

/// T-215 — Vehicles-palette leaf `pointerdown` → arm a **vehicle** place. Same lifecycle as
/// [`begin_place`] (consumed by [`place_at`], dropped by [`cancel_pending`]), so the map's
/// `has_pending` ghost and the release-over-chrome cancel need no vehicle-specific branch.
pub fn begin_place_vehicle(payload: PlacePayload) {
    arm(Pending::Vehicle(payload));
}

/// T-254 — Objects-palette leaf → arm an **entity** place (`entitiesById` / schema `entities[]`).
pub fn begin_place_object(payload: PlacePayload) {
    arm(Pending::Object(payload));
}

/// T-650 (COMP-PLACE-001) — Composition-palette row press → arm a **composition** place. Same
/// one-shot lifecycle as [`begin_place_object`] (consumed by [`place_at`] on a canvas release,
/// dropped by [`cancel_pending`] on a release over chrome); the canvas release re-anchors every
/// captured entity at the drop point and writes them as one undo step ([`place_composition_at`]).
pub fn begin_place_composition(composition_id: String) {
    arm(Pending::Composition(composition_id));
}

/// T-791 — the composition id currently armed for placement, or `None`. Backs the compositions
/// panel's LIVE armed-state hint ("click the map to place… · Esc to cancel"), the mirror of
/// [`armed_marker_icon`]. Reading it under the `doc_tick` heartbeat is what makes the hint appear on
/// [`begin_place_composition`] and vanish on [`cancel_pending`] (Esc / RMB / release over chrome),
/// since both bump that tick.
#[must_use]
pub fn armed_composition_id() -> Option<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Composition(id)) => Some(id.clone()),
            _ => None,
        }
    })
}

/// Arm a place. Objects mode only accepts [`Pending::Object`]; side modes reject Object so a
/// leftover Objects arm cannot commit after the chip switches away.
///
/// T-791 — the arm bumps the dock tick on the way out (via [`bump_doc_tick`], called OUTSIDE the
/// `OPS_CTX` borrow so the nested borrow inside it cannot re-enter and panic). That is what lets a
/// palette/composition surface show a LIVE armed-state hint that appears the instant a row is armed:
/// the compositions panel reads [`armed_composition_id`] off this tick, exactly as the Markers panel
/// reads [`armed_marker_icon`]. Arming writes no document row, so this is the light `bump_doc_tick`
/// (dock re-read only), not `refresh_docks`/`after_local_edit` (tree rebuild + persist).
fn arm(pending: Pending) {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let objects = ctx.objects_mode.get_untracked();
            let ok = match &pending {
                Pending::Object(_) => objects,
                Pending::Character(_) | Pending::Vehicle(_) => !objects,
                // T-650 — a composition arms from its OWN tab, independent of the Objects chip (like
                // a zone, it is neither a BLUFOR thing nor an Objects thing). Always accepted; the
                // Composition tab is the only surface that produces this arm.
                Pending::Composition(_) => true,
                // T-069 — a marker arms from its OWN tab for the same reason a composition does: a
                // marker is neither a BLUFOR thing nor an Objects thing. Its SIDE comes from the
                // active chip at DROP time (`ensure_side_faction` in `place_at_impl`), not from the
                // Objects flag, so the Objects chip has nothing to say about whether it may arm.
                Pending::Marker(_) => true,
                // T-582 — zones are not a palette arm and do not route through here
                // ([`begin_zone_draw`] sets `pending` itself, because a zone is authorable under
                // either chip: a play area is not a BLUFOR thing or an Objects thing). Rejecting
                // rather than accepting keeps this function's contract "palette arms only".
                Pending::Zone(_) => false,
            };
            if !ok {
                *ctx.pending.borrow_mut() = None;
                return;
            }
            *ctx.pending.borrow_mut() = Some(pending);
        }
    });
    // T-791 — reflect the new armed state in the dock (armed-state hint). Outside the borrow above.
    bump_doc_tick();
}

/// T-650 — how many entities are currently selected (backs the "Save composition…" affordance,
/// which is shown only when a selection exists). The selection is app-side slot/vehicle ids.
#[must_use]
pub fn selection_len() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .map_or(0, |ctx| ctx.selection.borrow().len())
    })
}

/// Is a palette drag in flight? The `pointerup` handler asks before doing any work.
#[must_use]
pub fn has_pending() -> bool {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|ctx| ctx.pending.borrow().is_some())
    })
}

/// Drop the armed place (release over chrome, or pointercancel).
///
/// **T-582 — a zone draw survives this.** The palette arms are one-shot, so a release over a dock
/// means "I changed my mind" and dropping them is right. A zone draw is multi-click, and the chrome
/// host stops `pointerdown` only — so every click on the Zones panel's own Close / Undo vertex /
/// Cancel buttons ALSO bubbles a `pointerup` to the map container and lands here. Dropping the draft
/// on those would make the Close control destroy the ring it is meant to commit. A zone draw is
/// therefore ended only by finishing it or by [`cancel_zone_draw`], which the Cancel button calls
/// explicitly.
///
/// T-791 — when the arm is actually cleared (Esc / RMB / release over chrome), bump the dock tick so
/// the armed-state hint disappears — the "hint gone" half of the composition-place acceptance. A zone
/// draw survives (the early return), so its hint must NOT be torn down: the bump is gated on a real
/// clear and runs OUTSIDE the `OPS_CTX` borrow (`bump_doc_tick` borrows it again).
pub fn cancel_pending() {
    let cleared = OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let mut p = ctx.pending.borrow_mut();
            if matches!(*p, Some(Pending::Zone(_))) {
                return false;
            }
            let had = p.is_some();
            *p = None;
            return had;
        }
        false
    });
    if cleared {
        bump_doc_tick();
    }
}

/// Every slot id the document actually holds — the slot half of both minters' uniqueness universe,
/// read off the EXACT [`MissionDocCore::slots_json`] row map.
///
/// **Never `materialize()`.** The SoA is a VIEW: it drops slots on a hidden layer (T-665) and slots
/// carrying their own `editorHidden` flag (T-701) before any column is pushed, so a hidden slot's id
/// is INVISIBLE to a materialize-sourced uniqueness proof. Combined with [`OpsCtx::next_id`] resetting
/// to 0 on every editor mount, that is a silent-destruction path: restore a document from IDB whose
/// `n0` sits on a hidden layer, place anything, and the mint hands back `n0` — and the doc writes are
/// upserts (`add_slot` / `place_composition` insert a fresh row under the id), so the tucked-away slot
/// is overwritten inside the placement's undo step, with no error and no warning. Hidden slots are
/// precisely the work a careful author put out of sight, i.e. the work least likely to be noticed
/// missing.
///
/// This is the wave-127 rule for the id namespace: read `slots_json` (exact, complete), never the
/// SoA/materialized view, which is lossy about hidden slots — the same reason
/// [`capture_selection_entities`] and the shared `slot_z` reader were pointed at `slots_json`.
/// Cost is one O(all slots) parse per placement gesture, the same price `paste_at_cursor` and the
/// Attributes readers already pay; a place is a rare gesture, not the render hot path.
fn live_slot_ids(core: &MissionDocCore) -> std::collections::HashSet<String> {
    serde_json::from_str::<serde_json::Value>(&core.slots_json())
        .ok()
        .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
        .unwrap_or_default()
}

/// Mint an unused slot id. The counter keeps this O(1) amortized, but uniqueness is **proven**
/// against the live doc rather than assumed: undo frees ids, and an IDB restore can bring back a
/// document that already used `n0`.
///
/// The proof runs over [`live_slot_ids`] — the raw `slots_json` keys, hidden rows included — because
/// the materialized SoA cannot see a hidden slot and would let this hand back an id that is already
/// taken. See that helper for the full argument.
fn mint_id(ctx: &OpsCtx, core: &MissionDocCore) -> String {
    let existing = live_slot_ids(core);
    loop {
        let id = format!("n{}", ctx.next_id.get());
        ctx.next_id.set(ctx.next_id.get().saturating_add(1));
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// T-650 — mint `count` unused ids at once, proven unique against slots AND vehicles AND entities
/// (a composition place mixes all three, so uniqueness against the slot SoA alone — [`mint_id`] —
/// is not enough). The union is read once; each minted id is also added to it so a run of ids inside
/// one call cannot collide with itself.
///
/// **T-781 — `commentsById` joined that union** the moment a composition could carry a comment. The
/// disjoint `cmt-`/`n` prefixes ([`mint_comment_id`]) keep hand-placed notes out of the way, but a
/// composition's comment is minted HERE and so gets an `n{k}` id like everything else; without this
/// key a second placement could re-mint an id an earlier composed note already holds and upsert it
/// away. Uniqueness is proven against the live doc rather than assumed, exactly as [`mint_id`]
/// argues: undo frees ids and an IDB restore can bring back a document that already used them.
///
/// **The slot half comes off [`live_slot_ids`], not `materialize()`** — a hidden slot is absent from
/// the SoA, so the old source let a placement re-mint (and upsert away) an id a hidden row already
/// held. The three small-map halves below need no such treatment: `small_maps_json` dumps those root
/// maps verbatim, with no visibility filter anywhere in it.
fn mint_ids(ctx: &OpsCtx, core: &MissionDocCore, count: usize) -> Vec<String> {
    let mut existing = live_slot_ids(core);
    if let Ok(small) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) {
        for key in ["vehiclesById", "entitiesById", "commentsById"] {
            if let Some(obj) = small.get(key).and_then(|v| v.as_object()) {
                existing.extend(obj.keys().cloned());
            }
        }
    }
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        let id = format!("n{}", ctx.next_id.get());
        ctx.next_id.set(ctx.next_id.get().saturating_add(1));
        if existing.insert(id.clone()) {
            out.push(id);
        }
    }
    out
}

/// T-650 — the terrain bounds `[x0, y0, width, height]` for the live mission (the `paste_at_cursor`
/// idiom: read `meta.terrain` off `small_maps_json`, resolve bounds via the compile helper).
fn terrain_bounds_of(core: &MissionDocCore) -> [f64; 4] {
    let terrain = serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
        .ok()
        .and_then(|v| v.get("meta")?.get("terrain")?.as_str().map(str::to_string))
        .unwrap_or_default();
    map_engine_core::mission::compile::terrain_bounds(&terrain)
}

/// T-650 — the `entities` array (as a JSON string) of composition `id`, or `None` when the id is
/// absent. Read off the narrow [`MissionDocCore::compositions_json`] getter.
fn composition_entities_json(core: &MissionDocCore, id: &str) -> Option<String> {
    let map = serde_json::from_str::<serde_json::Value>(&core.compositions_json()).ok()?;
    let entities = map.get(id)?.get("entities")?;
    Some(entities.to_string())
}

/// T-650 — how many entities an `entities` JSON array carries (0 for a non-array).
fn composition_entity_count(entities_json: &str) -> usize {
    serde_json::from_str::<serde_json::Value>(entities_json)
        .ok()
        .and_then(|v| v.as_array().map(Vec::len))
        .unwrap_or(0)
}

/// T-650 — does `id` name a live slot? (Used to keep only slot ids in the post-place selection —
/// SEL/tint run over the slot SoA.) Reads the materialized SoA once per call; a placement is a rare
/// gesture, not the render hot path.
fn slot_attrs_exists(core: &MissionDocCore, id: &str) -> bool {
    core.materialize().ids.iter().any(|s| s == id)
}

/// Resolve the drop target: the active layer if it still exists, else any existing layer (the
/// lexicographically first, so the choice is deterministic), else mint the default one. Mirrors
/// React's `activeLayerId ?? ensureDefaultLayer(md)`.
fn ensure_layer(ctx: &OpsCtx, core: &MissionDocCore) -> String {
    let rows = layer_rows(core);
    if let Some(active) = ctx.active_layer.get_untracked() {
        if rows.iter().any(|l| l.id == active) {
            return active;
        }
        ctx.active_layer.set(None); // stale pointer (folder deleted / undone away)
    }
    if let Some(first) = rows.first() {
        return first.id.clone();
    }
    core.add_editor_layer(DEFAULT_LAYER_ID, DEFAULT_LAYER_NAME, None);
    DEFAULT_LAYER_ID.to_string()
}

/// T-169 smoke hook — bulk-add `n` slots (each a new squad under BLUFOR), then refresh the docks,
/// so the virtual-outliner gate can push a tree past [`crate::outliner::VIRTUAL_SLOT_THRESHOLD`]
/// without 50 palette drags. Not on any UI path (the `__missionDoc` bridge exposes it for the gate).
pub fn debug_seed_slots(n: u32) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        let layer_id = ensure_layer(ctx, core);
        for _ in 0..n {
            let id = mint_id(ctx, core);
            let _ = place_character_under_side(
                core, "BLUFOR", &id, &layer_id, "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
            );
        }
    });
    crate::mission_history::after_local_edit();
}

/* ───────────────────────── T-180.7 — ORBAT Manager mutators ───────────────────────── */

/// Slot fields the ORBAT Manager inspector / `format_slot_line` need (from `slots_json`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OrbatSlotDetail {
    pub id: String,
    pub role: String,
    pub tag: String,
    pub callsign: String,
    pub rank: String,
    pub index: u32,
    pub squad_id: String,
    pub summary: String,
    pub primary: String,
    pub launcher: String,
}

/// Snapshot of squads/factions/slot details for the ORBAT Manager (one doc read).
#[derive(Clone, Debug, Default)]
pub struct OrbatManagerSnapshot {
    pub factions: Vec<crate::outliner::FactionRow>,
    pub squads: Vec<crate::outliner::SquadRow>,
    pub slots: Vec<OrbatSlotDetail>,
}

/// Read live ORBAT rows + per-slot loadout/identity for the Stitch manager (G7 live data).
pub fn orbat_manager_snapshot() -> OrbatManagerSnapshot {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return OrbatManagerSnapshot::default();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return OrbatManagerSnapshot::default();
        };
        OrbatManagerSnapshot {
            factions: faction_rows(core),
            squads: squad_rows(core),
            slots: slot_details(core),
        }
    })
}

/// T-659 — the live input the header's slot census reads: `(factions, squads, slot squad ids)`.
///
/// Reuses [`orbat_manager_snapshot`] (one doc read of the same rows the ORBAT Manager sees) rather
/// than re-deriving from the document — so the header badge and the ORBAT dock can never disagree
/// about who is on which side. The third element is one `squadId` per slot (empty string when the
/// slot carries none); its length is the total slot count, which is what makes the pure
/// [`crate::eden_top_strip::census_from_rows`] buckets provably sum to the total. Vehicles are read
/// by the sibling [`vehicle_rows`] and are deliberately NOT folded in here: the header is a *slot*
/// (people) census, and mixing crewed vehicles into the same per-side integers would misreport the
/// roster the community naming convention is built on.
#[must_use]
pub fn census_input() -> (
    Vec<crate::outliner::FactionRow>,
    Vec<crate::outliner::SquadRow>,
    Vec<String>,
) {
    let snap = orbat_manager_snapshot();
    let slot_squad_ids = snap.slots.iter().map(|s| s.squad_id.clone()).collect();
    (snap.factions, snap.squads, slot_squad_ids)
}

fn slot_details(core: &MissionDocCore) -> Vec<OrbatSlotDetail> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return Vec::new();
    };
    let Some(map) = root.as_object() else {
        return Vec::new();
    };
    map.values()
        .filter_map(|v| {
            let o = v.as_object()?;
            let lo = o.get("loadout").and_then(|l| l.as_object());
            Some(OrbatSlotDetail {
                id: o.get("id")?.as_str()?.to_string(),
                role: o
                    .get("role")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),
                tag: o
                    .get("tag")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default()
                    .to_string(),
                callsign: o
                    .get("callsign")
                    .and_then(|c| c.as_str())
                    .unwrap_or_default()
                    .to_string(),
                rank: o
                    .get("rank")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),
                index: o
                    .get("index")
                    .and_then(|i| i.as_u64().or_else(|| i.as_i64().map(|n| n as u64)))
                    .unwrap_or(0) as u32,
                squad_id: o
                    .get("squadId")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                summary: lo
                    .and_then(|m| m.get("summary"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                primary: lo
                    .and_then(|m| m.get("primary"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                launcher: lo
                    .and_then(|m| m.get("launcher"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

fn ensure_side_faction(core: &MissionDocCore, side: &str) -> String {
    let faction_id = format!("faction-{side}");
    let factions = faction_rows(core);
    if !factions.iter().any(|f| f.id == faction_id) {
        core.add_faction(&faction_id, side, side);
    }
    faction_id
}

fn mint_squad_id_for_side(core: &MissionDocCore, side: &str) -> String {
    let existing: std::collections::HashSet<String> =
        squad_rows(core).into_iter().map(|s| s.id).collect();
    let mut n: u32 = 1;
    loop {
        let id = format!("squad-{side}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

/// G5 — add an empty squad under `side` (`BLUFOR`/`OPFOR`/`INDFOR`).
pub fn orbat_add_squad(side: String) -> Option<String> {
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return None;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return None;
        };
        if !matches!(side.as_str(), "BLUFOR" | "OPFOR" | "INDFOR") {
            return None;
        }
        let faction_id = ensure_side_faction(core, &side);
        let squad_id = mint_squad_id_for_side(core, &side);
        let ordinal = faction_rows(core)
            .iter()
            .find(|f| f.id == faction_id)
            .map(|f| f.squad_ids.len())
            .unwrap_or(0);
        let name = format!("Squad {}", ordinal + 1);
        core.add_squad(&squad_id, &faction_id, &name, None);
        Some(squad_id)
    });
    if id.is_some() {
        crate::mission_history::after_local_edit();
    }
    id
}

/// T-188 — along-front spacing between the slots of one squad, matching
/// `apply_faction_library`'s `APPLY_ANCHOR_X + 15.0 * i` formation so a hand-built squad and an
/// applied kit lay out identically.
const ORBAT_SLOT_SPACING_X: f64 = 15.0;

/// G6 — add a role (slot) into an existing squad; default role Rifleman. Not `place_character_under_side`.
///
/// **T-188 — placement.** This used to hand `add_slot` `0.0, 0.0, 0.0, 0.0`, so every role added
/// from the ORBAT dock materialised at world origin — the terrain's south-west corner, nowhere near
/// its squad, and stacked on every other Add Role. It now anchors the way the sibling
/// [`orbat_add_vehicle`] already does, off the squad's own geometry: see [`next_slot_xy`] for the
/// lane, and [`squad_anchor_xy`] for the empty-squad fallback.
pub fn orbat_add_slot(squad_id: String, role: String) -> Option<String> {
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return None;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return None;
        };
        let sq = squad_rows(core).into_iter().find(|s| s.id == squad_id)?;
        let index = sq.slot_ids.len() as u32;
        let layer_id = ensure_layer(ctx, core);
        let slot_id = mint_id(ctx, core);
        let role = if role.trim().is_empty() {
            "Rifleman".to_string()
        } else {
            role
        };
        let (x, y) = next_slot_xy(core, &sq);
        let asset_id = asset_id_for_role(core, &sq, &role);
        core.add_slot(
            &slot_id,
            &squad_id,
            &layer_id,
            index,
            &role,
            None,
            asset_id.clone(),
            x,
            y,
            0.0,
            0.0,
        );
        // T-068.15.2 — a slot that carries a character carries its default cargo too; same borrow
        // scope ⇒ one undo step with the add, exactly like the place / apply-kit hooks.
        if let Some(a) = &asset_id {
            seed_cargo_in_core(core, &slot_id, a, None);
        }
        if sq.leader_slot_id.is_empty() {
            core.set_leader(&squad_id, &slot_id);
        }
        Some(slot_id)
    });
    if id.is_some() {
        crate::mission_history::after_local_edit();
    }
    id
}

/// T-188 — best-effort `assetId` for a role added from the ORBAT dock.
///
/// Add Role has no character picker (the palette drag is the only place a character is chosen), so
/// the slot is minted with no `assetId` of its own. Derive one from the doc instead: the resource
/// name an existing slot with the **same role** already carries, nearest first — the squad itself,
/// then sibling squads under the same faction. Deterministic: both tiers walk ordered doc arrays
/// (`faction.squadIds` / `squad.slotIds`), never the unordered `slotsById` map, so two identical
/// missions pick the same character.
///
/// Deliberately role-keyed, not "any squad-mate": borrowing the squad leader's prefab for a
/// Rifleman would swap the character out for the wrong one, which is worse than leaving it unset.
///
/// **Scope is the faction, and stops there (wave-1 fix).** A third tier used to sweep the whole
/// mission (`obj.keys()`), matching on `role` alone with no faction predicate anywhere. That is a
/// cross-faction character leak, and it is reachable with stock data: `faction-library.sample.json`
/// defines only `Rifleman` / `Squad Leader`, both on `Character_USSR_Rifleman.et`, and both ORBAT
/// "Add Role" buttons hardcode `"Rifleman"` — apply the Soviet template, add a BLUFOR squad, hit
/// Add Role, and tiers 1–2 come up empty while tier 3 hands the BLUFOR slot a USSR body. It reaches
/// the game intact: `kit-aliases.json` maps that resource to `kit:sov_rifleman` and
/// `mission::flatten` resolves the kit per-`assetId`.
///
/// Returning `None` is the **correct** outcome, not a gap. `flatten` falls back to
/// `KitAliases::faction_default(faction_key)` for a slot with no `assetId`, i.e. the slot compiles
/// to its own faction's default kit — so tier 3 traded a faction-correct default for a possibly
/// faction-wrong pick. Tiers 1–2 already cover every character the faction owns; anything beyond
/// them is by definition another faction's.
fn asset_id_for_role(
    core: &MissionDocCore,
    sq: &crate::outliner::SquadRow,
    role: &str,
) -> Option<String> {
    let root = serde_json::from_str::<serde_json::Value>(&core.slots_json()).ok()?;
    let obj = root.as_object()?;

    let squads = squad_rows(core);
    let siblings = faction_rows(core)
        .into_iter()
        .find(|f| f.id == sq.faction_id)
        .map(|f| f.squad_ids)
        .unwrap_or_default();

    let mut candidates: Vec<&String> = sq.slot_ids.iter().collect();
    for sid in &siblings {
        if let Some(s) = squads.iter().find(|s| s.id == *sid) {
            candidates.extend(s.slot_ids.iter());
        }
    }

    candidates.into_iter().find_map(|id| {
        let slot = obj.get(id)?;
        if slot.get("role").and_then(serde_json::Value::as_str) != Some(role) {
            return None;
        }
        slot.get("assetId")
            .and_then(serde_json::Value::as_str)
            .filter(|a| !a.is_empty())
            .map(ToString::to_string)
    })
}

/// G2 — Make SL via core `set_leader` (does not overwrite MED/ENG tag).
pub fn orbat_set_leader(squad_id: String, slot_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_leader(&squad_id, &slot_id);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// Remove a slot (cascade detach); GC empty squad; promote leader when needed.
pub fn orbat_remove_slot(slot_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let detail = slot_details(core)
            .into_iter()
            .find(|s| s.id == slot_id)
            .unwrap_or_default();
        let squad_id = detail.squad_id.clone();
        if squad_id.is_empty() {
            return false;
        }
        let sq = squad_rows(core).into_iter().find(|s| s.id == squad_id);
        let was_leader = sq.as_ref().is_some_and(|s| s.leader_slot_id == slot_id);
        let remaining: Vec<String> = sq
            .map(|s| s.slot_ids.into_iter().filter(|id| id != &slot_id).collect())
            .unwrap_or_default();
        core.remove_slots(vec![slot_id]);
        if remaining.is_empty() {
            core.remove_squad(&squad_id);
        } else if was_leader {
            if let Some(next) = remaining.first() {
                core.set_leader(&squad_id, next);
            }
        }
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// Remove a squad and its slots.
pub fn orbat_remove_squad(squad_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.remove_squad(&squad_id);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// Rename a squad.
pub fn orbat_rename_squad(squad_id: String, name: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.rename_squad(&squad_id, &name);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-180.8 — REPLACE-apply a Faction Library doc onto `side` (H-L2 / H-L7b).
///
/// **T-308 — returns the refusal instead of swallowing it.** This used to be `-> bool` over
/// `apply_faction_library(...).is_ok()`, so the one thing the operator needed — *which* squads
/// block the apply and how to clear them — was discarded one line after it was computed, and the
/// dialog printed "Apply failed." on top of a confirm the operator had already accepted. The
/// `Err` string is `ApplyFactionError`'s own `Display`; the other two arms cover the cases where
/// there is no document to apply onto at all (previously also a bare `false`).
pub fn orbat_apply_faction(side: String, doc: FactionDoc) -> Result<(), String> {
    let res = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Err("No mission editor is open.".to_string());
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Err("No mission document is loaded.".to_string());
        };
        let layer_id = ensure_layer(ctx, core);
        let input = FactionLibraryInput {
            name: doc.name,
            roles: doc
                .roles
                .into_iter()
                .map(|r| FactionLibraryRole {
                    role: r.role,
                    tag: r.tag,
                    character: r.character,
                    loadout: r.loadout,
                })
                .collect(),
            vehicles: doc
                .vehicles
                .into_iter()
                .map(|v| FactionLibraryVehicle {
                    vehicle: v.vehicle,
                    label: v.label,
                })
                .collect(),
        };
        apply_faction_library(core, &side, &layer_id, &input).map_err(|e| e.to_string())?;
        // T-068.15.2 — seed default cargo after a kit apply. Cargo-key-absent
        // slots only, so user edits and library-carried cargo[] are preserved;
        // same borrow scope ⇒ one undo step with the apply.
        if let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) {
            if let Some(obj) = map.as_object() {
                for (sid, slot) in obj {
                    let Some(rn) = slot
                        .get("assetId")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                    else {
                        continue;
                    };
                    let lo = slot
                        .get("loadout")
                        .filter(|l| !l.is_null())
                        .map(|l| l.to_string());
                    seed_cargo_in_core(core, sid, rn, lo.as_deref());
                }
            }
        }
        Ok(())
    });
    if res.is_ok() {
        crate::mission_history::after_local_edit();
    }
    res
}

/// T-180.8 — `add_vehicle` + `attach_vehicle` with map position (H-L7 / H8).
pub fn orbat_add_vehicle(squad_id: String, resource_name: String) -> Option<String> {
    if resource_name.trim().is_empty() {
        return None;
    }
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return None;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return None;
        };
        let sq = squad_rows(core).into_iter().find(|s| s.id == squad_id)?;
        let n = sq.vehicle_ids.len();
        let vehicle_id = mint_id(ctx, core);
        // Place near squad leader / first slot, else Everon center.
        let (x, y) = squad_anchor_xy(core, &sq).unwrap_or((APPLY_ANCHOR_X, APPLY_ANCHOR_Y));
        let x = x + 30.0 + 20.0 * n as f64;
        let y = y - 30.0;
        core.add_vehicle(
            &vehicle_id,
            &resource_name,
            Some(x),
            Some(y),
            Some(0.0),
            Some(0.0),
        );
        core.attach_vehicle(&squad_id, &vehicle_id);
        Some(vehicle_id)
    });
    if id.is_some() {
        crate::mission_history::after_local_edit();
    }
    id
}

/// T-215 — the vehicle half of [`place_at`]: a `vehiclesById` row at the operator's map point,
/// owned by the active Eden side. Returns `false` on an unknown side (the same refusal
/// `place_character_under_side` gives), leaving the doc untouched.
///
/// **Why this does not attach the vehicle to a squad.** The obvious symmetry with the character path
/// would be `attach_vehicle` onto the side's current squad — and it is wrong.
/// `place_orbat::is_open_for_placement` treats a squad holding *any* vehicle as authored, so the
/// attach would close the side's current squad and the very next character placement would mint a
/// fresh one. Placing a truck would silently split the fireteam being built around it, which is the
/// one-squad-per-click defect T-321 exists to prevent. The side is recorded on the vehicle itself
/// (`factionId`) instead; `attach_vehicle` remains the way to say "this squad's vehicle", from the
/// ORBAT Manager where that is what the operator means.
///
/// `z`/`rotation` are `0.0` for the same reason the character path uses them: the flat-map commit
/// has no DEM sample yet. Heading is authored afterwards, not guessed at drop.
fn place_vehicle_in_core(
    core: &MissionDocCore,
    side: &str,
    vehicle_id: &str,
    resource_name: &str,
    x: f64,
    y: f64,
    with_crew: bool,
) -> bool {
    if !matches!(side, "BLUFOR" | "OPFOR" | "INDFOR") || resource_name.trim().is_empty() {
        return false;
    }
    let faction_id = ensure_side_faction(core, side);
    // T-732 — add + factionId + crewed stamp in ONE LOCAL txn (was three Ctrl+Z for unmanned).
    core.place_vehicle_with_crew_stamp(
        vehicle_id,
        resource_name,
        x,
        y,
        0.0,
        0.0,
        &faction_id,
        with_crew,
    );
    true
}

/// T-254 — Objects half of [`place_at`]: an `entitiesById` row at the map point.
///
/// `alias` is derived from the ResourceName + display label (`derive_object_alias`); `faction`
/// is the schema factionKey slug (`blufor`/`opfor`/`indfor`) from the active Eden side.
fn place_object_in_core(
    core: &MissionDocCore,
    side: &str,
    entity_id: &str,
    payload: &PlacePayload,
    x: f64,
    y: f64,
) -> bool {
    if !matches!(side, "BLUFOR" | "OPFOR" | "INDFOR") || payload.asset_id.trim().is_empty() {
        return false;
    }
    let alias = crate::asset_catalog::derive_object_alias(&payload.asset_id, &payload.role);
    if alias.is_empty() {
        return false;
    }
    // Ensure the side's faction row exists (same as vehicles) so the mission graph stays coherent,
    // but write the schema factionKey on the entity itself — not `faction-{SIDE}`.
    let _ = ensure_side_faction(core, side);
    let faction = side.to_lowercase();
    core.add_entity(entity_id, &alias, &payload.asset_id, x, y, 0.0, 0.0);
    core.set_entity_faction(entity_id, &faction);
    true
}

/// T-215 — one authored cargo row on a vehicle: `{item, qty}`, `mission.schema.json`
/// `$defs/entityInventory`. No `container` key — for an entity the container *is* the entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VehicleCargoRow {
    pub item: String,
    pub qty: i64,
}

/// T-215 — one vehicle as the docks need it: identity, where it is, and what it carries.
#[derive(Clone, Debug, PartialEq)]
pub struct VehicleRow {
    pub id: String,
    pub resource_name: String,
    /// `None` for a vehicle that has never been given a map position — the state every
    /// ORBAT-added vehicle was in before this ticket, and still is when added from the ORBAT
    /// Manager without a drop.
    pub xy: Option<(f64, f64)>,
    /// Authored heading (degrees). `None` when unplaced; `Some(0.0)` is a real authored zero.
    pub rotation: Option<f64>,
    /// Elevation when placed; `None` when unplaced.
    pub z: Option<f64>,
    /// `faction-{SIDE}` when the vehicle was map-placed; empty when it only has a squad.
    pub faction_id: String,
    /// Empty when the vehicle is not attached to a squad (every map-placed vehicle).
    pub squad_id: String,
    pub cargo: Vec<VehicleCargoRow>,
    /// T-076 — the vehicle's crew map: `seat_id → slot_id`. Empty when nobody is boarded. Read off
    /// the same `crew` object the core writes ([`MissionDocCore::assign_crew_seat`]); the panel joins
    /// each generic seat (driver/gunner/commander/cargoN) against this to show who occupies it.
    pub crew: std::collections::HashMap<String, String>,
}

/// Read every `vehiclesById` row for the docks. Off `small_maps_json`, like [`squad_rows`] —
/// vehicles are deliberately off the slot SoA, so there is no columnar reader to use instead.
#[must_use]
pub fn vehicle_rows() -> Vec<VehicleRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
            return Vec::new();
        };
        let Some(map) = root.get("vehiclesById").and_then(|v| v.as_object()) else {
            return Vec::new();
        };
        let mut rows: Vec<VehicleRow> = map
            .iter()
            .map(|(id, v)| {
                let s = |k: &str| {
                    v.get(k)
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string()
                };
                let pos = v.get("position");
                VehicleRow {
                    id: id.clone(),
                    resource_name: s("resourceName"),
                    xy: pos.and_then(|p| Some((p.get("x")?.as_f64()?, p.get("y")?.as_f64()?))),
                    rotation: pos.and_then(|p| p.get("rotation")?.as_f64()),
                    z: pos.and_then(|p| p.get("z")?.as_f64()),
                    faction_id: s("factionId"),
                    squad_id: s("squadId"),
                    cargo: v
                        .get("cargo")
                        .and_then(|c| c.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|r| {
                                    Some(VehicleCargoRow {
                                        item: r.get("item")?.as_str()?.to_string(),
                                        qty: r.get("qty")?.as_i64()?,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    // T-076 — join key for the seat list: `{seat_id: slot_id}` off the vehicle row.
                    crew: v
                        .get("crew")
                        .and_then(|c| c.as_object())
                        .map(|o| {
                            o.iter()
                                .filter_map(|(seat, slot)| {
                                    Some((seat.clone(), slot.as_str()?.to_string()))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            })
            .collect();
        // `small_maps_json` is a serde_json object (key-sorted), but sort explicitly so the dock
        // order is a property of this reader rather than of a JSON implementation detail.
        rows.sort_by(|a, b| a.id.cmp(&b.id));
        rows
    })
}

/// T-215 — replace a vehicle's authored cargo, then the shared dirty tail (one undo step).
/// Rows the schema cannot represent are dropped by the core mutator; an empty list clears the key.
pub fn set_vehicle_cargo(vehicle_id: String, rows: Vec<VehicleCargoRow>) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let pairs: Vec<(String, i64)> = rows.into_iter().map(|r| (r.item, r.qty)).collect();
        core.set_vehicle_cargo(&vehicle_id, &pairs);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-425 — placed vehicle positions for map pick/marquee (`id`, world x, world y).
#[must_use]
pub fn vehicle_points() -> Vec<(String, f64, f64)> {
    vehicle_rows()
        .into_iter()
        .filter_map(|v| v.xy.map(|(x, y)| (v.id, x, y)))
        .collect()
}

/// T-425 — author a vehicle's heading (degrees). Preserves x/y/z. No-op if missing/unplaced.
pub fn set_vehicle_heading(vehicle_id: String, heading_deg: f64) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
            return false;
        };
        let Some(v) = root.get("vehiclesById").and_then(|m| m.get(&vehicle_id)) else {
            return false;
        };
        let Some(pos) = v.get("position") else {
            return false;
        };
        let (Some(x), Some(y)) = (
            pos.get("x").and_then(|n| n.as_f64()),
            pos.get("y").and_then(|n| n.as_f64()),
        ) else {
            return false;
        };
        let z = pos.get("z").and_then(|n| n.as_f64()).unwrap_or(0.0);
        core.set_vehicle_position(&vehicle_id, x, y, z, heading_deg);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-425 — drag-release commit for placed vehicles (shared world delta). Rotation preserved.
pub fn move_vehicles(ids: Vec<String>, dx: f64, dy: f64) -> bool {
    if ids.is_empty() || (dx == 0.0 && dy == 0.0) {
        return false;
    }
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.move_vehicles(&ids, dx, dy);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-425 — true when `id` is a `vehiclesById` key (used by the select/drag gesture to route
/// commits away from the slot SoA `move_entities` path).
#[must_use]
pub fn is_vehicle_id(id: &str) -> bool {
    vehicle_rows().iter().any(|v| v.id == id)
}

/// T-215 — delete a placed vehicle (the map palette can create them, so something must be able to
/// remove them). Core [`MissionDocCore::remove_vehicle`] also detaches it from any squad.
pub fn remove_vehicle(vehicle_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.remove_vehicle(&vehicle_id);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-076 — one placed slot as the crew seat picker needs it: the doc id plus a human-readable label
/// (its resolved role, or the raw id when a slot has no role yet). Every placed character is a
/// candidate crew member, so the picker's options are exactly [`slot_rows`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedSlotChoice {
    pub id: String,
    pub label: String,
}

/// T-076 — the placed slots a crew seat can be assigned to (board target list for the seat picker).
/// Sorted by label then id so the dropdown order is a property of this reader, not of SoA iteration.
#[must_use]
pub fn placed_slot_choices() -> Vec<PlacedSlotChoice> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let mut rows: Vec<PlacedSlotChoice> = slot_rows(core)
            .into_iter()
            .map(|s| {
                let label = if s.role.is_empty() {
                    s.id.clone()
                } else {
                    format!("{} ({})", s.role, s.id)
                };
                PlacedSlotChoice { id: s.id, label }
            })
            .collect();
        rows.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
        rows
    })
}

/// T-076 — board a placed slot into a vehicle seat (`vehicle.crew[seat_id] = slot_id`), then the
/// shared dirty tail (one undo step). The **one-seat-per-slot** rule is enforced by the core mutator
/// [`MissionDocCore::assign_crew_seat`], which vacates `slot_id` from any other seat of any vehicle
/// before writing — so this reader-agnostic op cannot author a soldier into two vehicles at once.
pub fn assign_crew_seat(vehicle_id: String, seat_id: String, slot_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.assign_crew_seat(&vehicle_id, &seat_id, &slot_id);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-076 — unboard: clear one vehicle seat. Core [`MissionDocCore::clear_crew_seat`] removes the
/// `crew` key once the last seat empties, so an unboard restores the pre-board row shape.
pub fn clear_crew_seat(vehicle_id: String, seat_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.clear_crew_seat(&vehicle_id, &seat_id);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// T-180.8 — inverse of Apply: build a FactionDoc from the live side graph (Save / Save as).
///
/// **T-373 — what comes back is a PARTIAL, not a document.** Two `faction-library.schema.json`
/// fields have no representation anywhere in the mission graph, so this can only ever emit `None`
/// for them, no matter how the operator authored the library entry:
///
/// - **`emblem`** — the ORBAT has no emblem concept at all. Nothing in `MissionDocCore` stores one,
///   so there is nothing here to read it back out of.
/// - **vehicle `label`** — a mission vehicle row is `{id, resourceName, position, squadId}` and
///   nothing else (`map-engine-core` `doc/store.rs::add_vehicle`), and `apply_faction_library`
///   throws the library's label away on the way IN —
///   `let _ = v.label; // label is UI-only; resourceName is the graph pin`
///   (`crates/map-engine-core/src/doc/apply_faction.rs:358`).
///
/// That matters because `PUT /factions/:id` is a whole-document **replace**
/// (`apps/website/api/src/handlers/factions.rs::update_faction`) and [`FactionDoc`]'s
/// `skip_serializing_if = "Option::is_none"` **omits** an absent key rather than nulling it. PUT
/// this straight back and the operator's emblem and every authored vehicle label are deleted from
/// the library, silently, on every save. That was T-373.
///
/// Everything else here is faithful and may be trusted as authored state: `role`, `tag`,
/// `character` and `loadout` all live on the slot, Apply writes them there
/// (`apply_faction.rs` `update_slot_loadout`), and the Arsenal edits them — so a role that comes
/// back with no loadout genuinely has no loadout, it is not a gap in the derivation.
///
/// **Callers that CREATE (`Save as` → POST) may use this as-is** — a brand-new faction has no
/// emblem and no labels to lose. **Callers that REPLACE (`Save` → PUT) must merge over the stored
/// document first:** [`crate::orbat_manager::merge_faction_doc_from_side`].
///
/// `None` when there is no live doc, or when the doc's own JSON will not parse — see
/// [`faction_doc_from_side_core`].
pub fn faction_doc_from_side(side: &str) -> Option<FactionDoc> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        faction_doc_from_side_core(core, side)
    })
}

/// `None` when `slots_json` / `small_maps_json` will not parse.
///
/// **T-373 — a parse failure is not an empty side.** These two early-outs used to return
/// `FactionDoc { side, name, ..Default::default() }`, i.e. a doc with `roles: []` and
/// `vehicles: []` — byte-identical to what a side that genuinely holds no squads produces. Handed
/// to the PUT that replaces the whole library document, that turned "the mission's JSON is
/// malformed" into "delete every role and vehicle in this template", and handed to `Save as` it
/// created an empty faction and reported success. A distinguishable `None` lets the caller say so
/// instead of writing.
fn faction_doc_from_side_core(core: &MissionDocCore, side: &str) -> Option<FactionDoc> {
    let factions = faction_rows(core);
    let squads = squad_rows(core);
    let faction = factions.iter().find(|f| f.key == side);
    let name = faction
        .map(|f| f.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| side.to_string());
    let squad_ids: Vec<String> = faction.map(|f| f.squad_ids.clone()).unwrap_or_default();
    let Ok(slots_root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return None;
    };
    let Ok(small) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return None;
    };
    let mut roles = Vec::new();
    let mut vehicles = Vec::new();
    for sid in &squad_ids {
        let Some(sq) = squads.iter().find(|s| s.id == *sid) else {
            continue;
        };
        for slot_id in &sq.slot_ids {
            let Some(slot) = slots_root.get(slot_id) else {
                continue;
            };
            roles.push(FactionRole {
                role: slot
                    .get("role")
                    .and_then(|r| r.as_str())
                    .unwrap_or("Rifleman")
                    .to_string(),
                tag: slot
                    .get("tag")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                    .filter(|s| !s.is_empty()),
                character: slot
                    .get("assetId")
                    .and_then(|a| a.as_str())
                    .unwrap_or_default()
                    .to_string(),
                loadout: slot.get("loadout").cloned(),
            });
        }
        for vid in &sq.vehicle_ids {
            let Some(v) = small.get("vehiclesById").and_then(|m| m.get(vid)) else {
                continue;
            };
            vehicles.push(FactionVehicle {
                vehicle: v
                    .get("resourceName")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),
                // T-373 — structurally inexpressible, not an oversight: the graph row this reads
                // (`vehiclesById[vid]`) has no label field to read. See the fn doc.
                label: None,
            });
        }
    }
    Some(FactionDoc {
        side: side.into(),
        name,
        // T-373 — likewise inexpressible. `merge_faction_doc_from_side` carries the stored one
        // over; do NOT PUT this partial as a replacement body.
        emblem: None,
        roles,
        vehicles,
    })
}

/// One slot's map position out of a parsed `slots_json`. `None` when the id is stale (the slot was
/// removed) or the stored position is malformed.
fn slot_xy(root: &serde_json::Value, id: &str) -> Option<(f64, f64)> {
    let pos = root.get(id)?.get("position")?;
    Some((pos.get("x")?.as_f64()?, pos.get("y")?.as_f64()?))
}

/// The squad's anchor out of an already-parsed `slots_json`: the leader if it still exists, else the
/// first slot that does. `None` only for a squad with nothing live to anchor against, which leaves
/// the fallback to the caller.
///
/// **T-188 wave-1 fix — chained, not single-shot.** This used to resolve exactly one id (leader when
/// set, otherwise `slot_ids.first()`) and return `None` the moment that lookup missed.
/// [`delete_selection`] deletes straight through `core.remove_slots`, which rewrites `slotIds` but
/// never touches `leaderSlotId` — so deleting the squad leader off the map left a **stale** anchor
/// id, `root.get(&anchor_id)` missed, and both callers fell through to
/// `unwrap_or((APPLY_ANCHOR_X, APPLY_ANCHOR_Y))`: the new slot / vehicle materialised at the Everon
/// centre, the exact world-origin symptom T-188 was written to stop. Walking leader → every live
/// slot keeps the squad's own geometry as the anchor and reserves the constant for a squad that
/// genuinely has no body left.
fn squad_anchor_in(root: &serde_json::Value, sq: &crate::outliner::SquadRow) -> Option<(f64, f64)> {
    std::iter::once(sq.leader_slot_id.as_str())
        .chain(sq.slot_ids.iter().map(String::as_str))
        .filter(|id| !id.is_empty())
        .find_map(|id| slot_xy(root, id))
}

fn squad_anchor_xy(core: &MissionDocCore, sq: &crate::outliner::SquadRow) -> Option<(f64, f64)> {
    let root = serde_json::from_str::<serde_json::Value>(&core.slots_json()).ok()?;
    squad_anchor_in(&root, sq)
}

/// T-188 — where the **next** slot added to `sq` belongs, in the squad's `ORBAT_SLOT_SPACING_X`
/// row. Falls back to the Everon centre only for a squad with nothing to anchor against — the same
/// origin `apply_faction_library` mints a fresh squad at.
///
/// **Wave-1 fix — the lane comes from geometry, not from a count.** It used to be
/// `anchor + 15 m * slot_ids.len()`. `remove_slots` deletes ids without reindexing the survivors, so
/// after deleting the middle of three slots `len()` is 2 while a survivor still sits at
/// `anchor + 30 m` — the next Add Role landed exactly on top of it, re-creating the stacking this
/// ticket exists to fix. `max(x) + 15 m` is free by construction no matter which ids were removed.
fn next_slot_xy(core: &MissionDocCore, sq: &crate::outliner::SquadRow) -> (f64, f64) {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return (APPLY_ANCHOR_X, APPLY_ANCHOR_Y);
    };
    let (ax, ay) = squad_anchor_in(&root, sq).unwrap_or((APPLY_ANCHOR_X, APPLY_ANCHOR_Y));
    let x = sq
        .slot_ids
        .iter()
        .filter_map(|id| slot_xy(&root, id).map(|(x, _)| x))
        .reduce(f64::max)
        .map_or(ax, |max_x| max_x + ORBAT_SLOT_SPACING_X);
    (x, ay)
}

/// Patch inspector fields: role/tag via `update_slot`, callsign/rank via `update_slot_identity`.
pub fn orbat_update_slot_fields(
    slot_id: String,
    role: Option<String>,
    tag: Option<String>,
    callsign: Option<String>,
    rank: Option<String>,
) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        if role.is_some() || tag.is_some() {
            core.update_slot(&slot_id, role, tag, None);
        }
        if callsign.is_some() || rank.is_some() {
            core.update_slot_identity(&slot_id, callsign, rank);
        }
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/* ───────────────────────── T-180.6 — ORBAT refile (core move only) ───────────────────────── */

thread_local! {
    /// Armed slot id between ORBAT slot `pointerdown` and squad-row `pointerup` (pointer-drag, not HTML5 DnD).
    static PENDING_REFILE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Arm a slot for refile into another squad (OrbatManager pointer-drag).
pub fn begin_refile(slot_id: String) {
    PENDING_REFILE.with(|p| *p.borrow_mut() = Some(slot_id));
}

/// Clear an armed refile without mutating the doc (drop outside a squad row).
pub fn cancel_refile() {
    PENDING_REFILE.with(|p| *p.borrow_mut() = None);
}

/// Complete an armed refile onto `dest_squad_id` via [`refile_slot`].
pub fn complete_refile_onto_squad(dest_squad_id: String) -> bool {
    let Some(slot_id) = PENDING_REFILE.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    refile_slot(slot_id, dest_squad_id)
}

/// Move `slot_id` into `dest_squad_id` through core [`MissionDocCore::move_slot_to_squad`] only
/// (F-L2 — no FE `slotIds` splice), then the shared dirty tail (orbat_nodes + squad links).
pub fn refile_slot(slot_id: String, dest_squad_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            core.move_slot_to_squad(&slot_id, &dest_squad_id);
        }
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// Commit an armed place at a **world** position, then select it and run the shared post-change
/// tail. Returns `false` when nothing was armed.
///
/// A **Factions** leaf files a slot into the side's current squad
/// ([`place_character_under_side`]). A **Vehicles** leaf (T-215) writes a `vehiclesById` row at the
/// same world point — see [`place_vehicle_in_core`] for why that one does not join a squad.
///
/// `z = 0.0` / `rotation = 0.0` match the T-159.19 drag commit's DEM-not-ready case (React's
/// `terrainZ` on the flat map).
/// Commit an armed place at a **world** position with no modifiers (crewed vehicle default, one-shot
/// arm). The plain-click entry point; the Ctrl/Alt gestures use [`place_at_keep`] / [`place_at_alt`].
/// Retained as the canonical no-modifier API (and the rustdoc anchor the `place_*_in_core` helpers
/// link) even though the live pointerup routes through [`place_at_alt`] to carry the Alt override.
#[allow(dead_code)]
pub fn place_at(x: f64, y: f64) -> bool {
    place_at_impl(x, y, false, false)
}

/// T-647 PLACE-CREW-001 — place with the Alt "empty vehicle" override. `alt_empty` forces a Vehicle
/// arm to stamp `crewed: false` regardless of the DockRight crew toggle (the per-gesture override of
/// the default); it is inert for a character/object arm. One-shot: the arm is consumed, exactly like
/// [`place_at`].
pub fn place_at_alt(x: f64, y: f64, alt_empty: bool) -> bool {
    place_at_impl(x, y, alt_empty, false)
}

/// T-647 PLACE-004 — Ctrl multi-place: place, but KEEP the pending armed so the next canvas click
/// drops another of the same entity. `alt_empty` carries the [`place_at_alt`] override through each
/// stamp of a multi-place run, so Ctrl+Alt drops a string of empty vehicles.
///
/// The arm is snapshotted before the (shared) consume and re-armed on a SUCCESSFUL place; a failed
/// place (nothing armed, or a core reject) does not re-arm — a multi-place that can't commit must
/// not spin. Re-arming through the raw `pending` cell (not [`arm`]) keeps the ORIGINAL arm verbatim:
/// the Objects/side-mode guard already vetted it once at [`begin_place*`] time, and the operator has
/// not changed chips mid-gesture.
pub fn place_at_keep(x: f64, y: f64, alt_empty: bool) -> bool {
    // Snapshot the armed value before the consume (zone draws never reach here — they are
    // multi-click already and `place_at_impl` returns via `advance_zone_draw`).
    let snapshot = OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.pending.borrow().clone())
    });
    let placed = place_at_impl(x, y, alt_empty, true);
    if placed {
        if let Some(p) = snapshot {
            OPS_CTX.with(|c| {
                if let Some(ctx) = c.borrow().as_ref() {
                    *ctx.pending.borrow_mut() = Some(p);
                }
            });
        }
    }
    placed
}

/// The shared place body. `alt_empty` is the PLACE-CREW-001 override (T-647); `keep` is honoured by
/// the [`place_at_keep`] wrapper (which re-arms after this returns) — this fn always `take`s the arm
/// so its borrow discipline is unchanged, and the wrapper restores it.
fn place_at_impl(x: f64, y: f64, alt_empty: bool, keep: bool) -> bool {
    // `keep` is acted on by the caller ([`place_at_keep`]); named here so the state machine reads
    // straight ("place, keeping the arm") and a future in-body use has its parameter.
    let _ = keep;
    // T-582 — a zone draw is MULTI-CLICK (circle: centre then rim; polygon: one vertex per click),
    // so it must not reach the `take` below, which is written for the one-shot palette arms.
    if zone_draw_armed() {
        return advance_zone_draw(x, y);
    }
    let placed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        // T-180.5 / T-254 — Objects mode only commits Object arms (handled below); do not
        // blanket-reject here (that was the stub that blocked entities[] placement).
        let Some(pending) = ctx.pending.borrow_mut().take() else {
            return false;
        };
        // Scoped: the mutators open write txns, which must be gone before `after_local_edit`'s
        // read txn. `select` is the id to leave selected, or `None` — the selection is the SLOT
        // selection (`select_tool::pick` runs over the slot SoA, and SEL counts it), so putting a
        // vehicle/entity id in it would show `SEL 1` with nothing highlighted anywhere.
        let select = {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            let side = ctx.active_side.get_untracked();
            let id = mint_id(ctx, core);
            match pending {
                // T-582 — unreachable: the zone branch at the top of this function returns before
                // the `take` that produced `pending`. Written as an explicit refusal rather than a
                // `_ =>` so that a fourth arm added later cannot fall silently into a slot place.
                Pending::Zone(_) => return false,
                Pending::Vehicle(payload) => {
                    // T-076 (RIGHT-CREW-001) — read the manned/unmanned toggle at place time.
                    // T-647 PLACE-CREW-001 — Alt is the per-gesture override: it forces empty
                    // (`crewed: false`) even when the dock toggle says crewed, and never the
                    // reverse (Alt cannot conjure a crew a switched-off toggle withheld).
                    let with_crew = place_with_crew() && !alt_empty;
                    if !place_vehicle_in_core(core, &side, &id, &payload.asset_id, x, y, with_crew)
                    {
                        return false;
                    }
                    None
                }
                Pending::Object(payload) => {
                    if !place_object_in_core(core, &side, &id, &payload, x, y) {
                        return false;
                    }
                    None
                }
                // T-069 (RIGHT-MODE-006) — drop a marker at the release point, under the active
                // side's faction briefing (the ONLY schema-legal marker placement; see the block
                // header at the end of this file). The slot id minted above is not used: a marker
                // is not a slot and lives in a different address space, so it mints its own id off
                // the marker list.
                Pending::Marker(icon) => {
                    let _ = id;
                    let faction_id = ensure_side_faction(core, &side);
                    let marker_id = mint_marker_id(&marker_rows_of(core));
                    // `y` here is the canvas's second WORLD axis, which is mission `z` — the same
                    // rename `advance_zone_draw(x, z)` makes one screen down. `$defs/marker` is
                    // `{x, z}` and carries no height at all, so there is no third component to drop.
                    let (mx, mz) = (x, y);
                    // Fresh markers carry an EMPTY label: `$defs/marker` requires the key, and the
                    // author captions it in the panel. Inventing "Marker 1" would put a placeholder
                    // on a game server's map the moment someone forgot to overwrite it.
                    core.set_faction_briefing_marker(&faction_id, &marker_id, mx, mz, &icon, "");
                    // Markers are not slots: putting a marker id in the slot selection would show
                    // `SEL 1` with nothing highlighted (the `place_at` selection rule).
                    None
                }
                Pending::Composition(comp_id) => {
                    // The single `id` minted above is unused for a composition (it mints its own
                    // per-entity ids). Resolve the row, mint one id per captured entity, and stamp
                    // them all in ONE undo step via the core mutator.
                    let _ = id;
                    let Some(entities) = composition_entities_json(core, &comp_id) else {
                        return false;
                    };
                    let count = composition_entity_count(&entities);
                    if count == 0 {
                        return false;
                    }
                    let ids = mint_ids(ctx, core, count);
                    let layer_id = ensure_layer(ctx, core);
                    let b = terrain_bounds_of(core);
                    let written =
                        core.place_composition(&entities, &ids, &side, &layer_id, x, y, b[2], b[3]);
                    if written.is_empty() {
                        return false;
                    }
                    // Select the placed SLOTS (SEL runs over the slot SoA; vehicle/object ids would
                    // show `SEL n` with nothing highlighted — the `place_at` selection rule).
                    let slot_ids: Vec<String> = written
                        .into_iter()
                        .filter(|w| slot_attrs_exists(core, w))
                        .collect();
                    *ctx.selection.borrow_mut() = slot_ids;
                    None
                }
                Pending::Character(payload) => {
                    let layer_id = ensure_layer(ctx, core);
                    let asset_id = payload.asset_id.clone();
                    if place_character_under_side(
                        core,
                        &side,
                        &id,
                        &layer_id,
                        &payload.role,
                        None,
                        Some(payload.asset_id),
                        x,
                        y,
                        0.0,
                        0.0,
                    )
                    .is_err()
                    {
                        return false;
                    }
                    // T-068.15.2 — a fresh placement has no loadout: seed the character's
                    // default cargo (same borrow scope ⇒ same undo step as the place).
                    seed_cargo_in_core(core, &id, &asset_id, None);
                    Some(id)
                }
            }
        };
        if let Some(id) = select {
            *ctx.selection.borrow_mut() = vec![id];
        }
        true
    });
    if placed {
        // Rebinds the glyphs from the new SoA, bumps `doc_ver`, schedules the persist, and refreshes
        // the HUD + docks — the same tail the drag commit and undo/redo run.
        crate::mission_history::after_local_edit();
    }
    placed
}

/// T-647 CONN-GROUP-001 (map half) — move the dragged character `slot_id` into the squad of the
/// character `target_id` it was Ctrl-dropped onto. Reads the target's squad off the materialized SoA
/// ([`slot_attrs`]) and refiles through the existing T-180.6 seam ([`refile_slot`], which runs the
/// shared dirty tail), so a map regroup and an ORBAT-dock refile are the SAME core move
/// (`move_slot_to_squad`) — one undo step, one squad-membership write.
///
/// Returns `false` (no-op) when the target has no squad (an unfiled slot, or the target vanished),
/// or already shares the dragged slot's squad — `move_slot_to_squad` is itself a no-op on
/// same-squad, but declining here keeps the caller from firing the dirty tail for nothing.
pub fn regroup_slot_onto(slot_id: &str, target_id: &str) -> bool {
    if slot_id == target_id {
        return false;
    }
    let dest_squad = read_attrs(target_id).map(|a| a.squad).unwrap_or_default();
    let src_squad = read_attrs(slot_id).map(|a| a.squad).unwrap_or_default();
    if dest_squad.is_empty() || dest_squad == src_squad {
        return false;
    }
    refile_slot(slot_id.to_string(), dest_squad)
}

/* ═══════════════════ T-582 — the zone draw tool (doc-mutating half) ═══════════════════ */

// T-211 shipped `zones` + eleven mutators on `MissionDocCore`; NOTHING called them. This block is
// the caller. The pure half — the schema-driven vocabularies, the 0.1 m grid, and the two shape
// predicates — lives in `eden_chrome` because it compiles on the NATIVE target, where
// `cargo test -p website-frontend` can run it; `MissionDocCore` is a wasm32-only dependency of this
// crate (see Cargo.toml), so anything that touches it can only live here, where no test runs.
//
// The division is deliberate: every decision this file makes is delegated to a predicate over there
// that IS tested (`circle_from_clicks`, `polygon_is_committable`, `zone_types`), so the untestable
// half stays as close to pure plumbing as it can be.

use crate::eden_chrome::{
    circle_from_clicks, polygon_flat, polygon_is_committable, zone_types, ZoneShape,
};
// T-079 — `DrawTarget` is imported straight from its home module (`eden_chrome` re-exports the other
// zone-tool pure items, but this one is added here in a slice that does not own `eden_chrome`).
use crate::eden_zones::DrawTarget;

/// One authored zone, read back for the dock list and the Attributes panel.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneRow {
    pub id: String,
    /// Schema `zone.type`.
    pub kind: String,
    pub label: Option<String>,
    pub faction: Option<String>,
    /// `rules` VERBATIM, as the opaque object the doc stores. Never parsed into named fields here —
    /// see `set_zone_rules`' note in `doc/store.rs` for why a typed mirror would be the second
    /// vocabulary T-241 exists to prevent.
    pub rules: serde_json::Value,
    /// `Some((x, z, r))` for a circle.
    pub circle: Option<(f64, f64, f64)>,
    /// The ring for a polygon.
    pub polygon: Vec<(f64, f64)>,
}

impl ZoneRow {
    /// A one-line geometry summary for the dock row.
    #[must_use]
    pub fn shape_summary(&self) -> String {
        if let Some((x, z, r)) = self.circle {
            format!("circle r {r:.1} m @ {x:.0}, {z:.0}")
        } else if self.polygon.is_empty() {
            "no shape".to_string()
        } else {
            format!("polygon, {} vertices", self.polygon.len())
        }
    }
}

/// Is a zone draw in flight?
#[must_use]
pub fn zone_draw_armed() -> bool {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|ctx| matches!(*ctx.pending.borrow(), Some(Pending::Zone(_))))
    })
}

/// The in-flight draw, for the dock's live hint ("click the rim", "2 vertices — one more to close").
#[must_use]
pub fn zone_draft() -> Option<ZoneDraft> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Zone(d)) => Some(d.clone()),
            _ => None,
        }
    })
}

/// Arm a draw against `collection` ([`DrawTarget`] — the second-consumer parameter). For a ZONE,
/// `kind` is a `zone.type` and must come from [`zone_types`] — this refuses anything else rather than
/// letting an invented type reach the document, where it would save 201 and then 500 `/compiled`
/// forever (T-581 measured exactly that for `"capture"`). For a TRIGGER, `kind` is the activation
/// kind and must be one of [`TRIGGER_ACTIVATIONS`] — the same "refuse an invented value at the arm"
/// discipline, applied to the trigger's own vocabulary.
pub fn begin_zone_draw(kind: &str, shape: ZoneShape, collection: DrawTarget) -> bool {
    let valid = match collection {
        DrawTarget::Zone => zone_types().iter().any(|t| t == kind),
        DrawTarget::Trigger => TRIGGER_ACTIVATIONS.contains(&kind),
    };
    if !valid {
        return false;
    }
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        *ctx.pending.borrow_mut() = Some(Pending::Zone(ZoneDraft {
            kind: kind.to_string(),
            shape,
            centre: None,
            verts: Vec::new(),
            target: None,
            collection,
        }));
        true
    })
}

/// T-582 / T-079 — arm a RESHAPE of an existing row in `collection`: the next clicks replace its
/// `shape` through `set_*_circle` / `set_*_polygon` instead of minting a new row.
///
/// The gesture is identical to a fresh draw (circle: centre then rim; polygon: vertices then Close),
/// so there is one geometry path and one set of guards — a reshape cannot produce the `r → 0.0`
/// circle or the two-vertex ring any more than a create can. `kind` is read from the live document
/// rather than taken from the caller: reshaping is a geometry edit, and silently retyping a row
/// because the dock's picker had drifted would be a different, invisible edit. The existence check
/// reads the collection's OWN map (zones vs triggers) so a reshape can never target the wrong one.
pub fn begin_zone_reshape(row_id: &str, shape: ZoneShape, collection: DrawTarget) -> bool {
    let kind = match collection {
        DrawTarget::Zone => zone_rows()
            .into_iter()
            .find(|r| r.id == row_id)
            .map(|r| r.kind),
        // T-079 — a trigger reshape keeps the trigger's ACTIVATION (its analogue of `zone.type`) as
        // the draft `kind`, so a create and a reshape carry the same field.
        DrawTarget::Trigger => trigger_rows()
            .into_iter()
            .find(|r| r.id == row_id)
            .map(|r| r.activation),
    };
    let Some(kind) = kind else {
        return false;
    };
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        *ctx.pending.borrow_mut() = Some(Pending::Zone(ZoneDraft {
            kind,
            shape,
            centre: None,
            verts: Vec::new(),
            target: Some(row_id.to_string()),
            collection,
        }));
        true
    })
}

/// Abandon the in-flight draw without writing anything. The explicit counterpart to
/// [`cancel_pending`], which a zone draw deliberately survives.
pub fn cancel_zone_draw() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let mut p = ctx.pending.borrow_mut();
            if matches!(*p, Some(Pending::Zone(_))) {
                *p = None;
            }
        }
    });
}

/// Drop the last polygon vertex (the Undo-vertex control). Returns the remaining count.
pub fn zone_draw_pop_vertex() -> usize {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let mut p = ctx.pending.borrow_mut();
        if let Some(Pending::Zone(d)) = p.as_mut() {
            d.verts.pop();
            return d.verts.len();
        }
        0
    })
}

/// One canvas release while a zone draw is armed.
///
/// **Circle** — the first release sets the centre, the second is the rim and COMMITS. The radius is
/// the distance between them, and [`circle_from_clicks`] refuses a rim that quantises the radius to
/// zero, so the click-without-travel that produced `r = 0.04` cannot create a zone at all.
///
/// **Polygon** — every release appends a vertex and the draw stays armed; nothing is written until
/// [`close_zone_polygon`], which enforces `minItems: 3`. The doc layer deliberately does not guard
/// that (its own comment assigns it here), so a ring is never handed over short.
fn advance_zone_draw(x: f64, z: f64) -> bool {
    enum Commit {
        Circle {
            kind: String,
            geom: (f64, f64, f64),
            target: Option<String>,
            collection: DrawTarget,
        },
        None,
    }
    let commit = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Commit::None;
        };
        let mut p = ctx.pending.borrow_mut();
        let Some(Pending::Zone(d)) = p.as_mut() else {
            return Commit::None;
        };
        match d.shape {
            ZoneShape::Polygon => {
                d.verts.push((x, z));
                Commit::None
            }
            ZoneShape::Circle => match d.centre {
                None => {
                    d.centre = Some((x, z));
                    Commit::None
                }
                Some((cx, cz)) => match circle_from_clicks(cx, cz, x, z) {
                    // A rim that would quantise the radius to zero is NOT a commit and NOT a
                    // cancel: the centre stays put so the author can simply drag further out.
                    None => Commit::None,
                    Some(geom) => {
                        let (kind, target, collection) =
                            (d.kind.clone(), d.target.clone(), d.collection);
                        *p = None;
                        Commit::Circle {
                            kind,
                            geom,
                            target,
                            collection,
                        }
                    }
                },
            },
        }
    });
    match commit {
        // T-079 — the ONLY per-collection branch in the whole draw flow: which mutator pair the
        // committed geometry calls. Everything above (centre/rim accumulation, the `r → 0.0` refusal)
        // is shared verbatim between zones and triggers.
        Commit::Circle {
            kind,
            geom: (cx, cz, r),
            target,
            collection,
        } => match (collection, target) {
            // Reshape: replaces the whole `shape` object, so name / owner / activation / rules
            // survive and the `oneOf` can never end up with both branches present.
            (DrawTarget::Zone, Some(id)) => edit_zone(|core| core.set_zone_circle(&id, cx, cz, r)),
            (DrawTarget::Zone, None) => write_row(DrawTarget::Zone, |core, id| {
                core.add_circle_zone(id, &kind, cx, cz, r);
            }),
            (DrawTarget::Trigger, Some(id)) => {
                edit_zone(|core| core.set_trigger_circle(&id, cx, cz, r))
            }
            (DrawTarget::Trigger, None) => write_row(DrawTarget::Trigger, |core, id| {
                core.add_circle_trigger(id, &kind, cx, cz, r);
            }),
        },
        // A vertex / centre landed but no document write happened yet. Report progress so the dock
        // re-reads, without running the persist tail for a doc that did not change.
        Commit::None => {
            bump_doc_tick();
            zone_draw_armed()
        }
    }
}

/// Close the in-flight ring. Refuses below three vertices — `$defs/polygon` is `minItems: 3` and a
/// two-vertex ring is a document the schema rejects.
pub fn close_zone_polygon() -> bool {
    let taken = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let mut p = ctx.pending.borrow_mut();
        let Some(Pending::Zone(d)) = p.as_ref() else {
            return None;
        };
        if d.shape != ZoneShape::Polygon || !polygon_is_committable(&d.verts) {
            return None;
        }
        let out = (
            d.kind.clone(),
            d.verts.clone(),
            d.target.clone(),
            d.collection,
        );
        *p = None;
        Some(out)
    });
    let Some((kind, verts, target, collection)) = taken else {
        return false;
    };
    let flat = polygon_flat(&verts);
    match (collection, target) {
        // Reshape — see [`begin_zone_reshape`]: whole-`shape` replacement, so a circle becomes a
        // polygon without leaving both `oneOf` branches on the row.
        (DrawTarget::Zone, Some(id)) => edit_zone(|core| core.set_zone_polygon(&id, &flat)),
        (DrawTarget::Zone, None) => write_row(DrawTarget::Zone, |core, id| {
            core.add_polygon_zone(id, &kind, &flat)
        }),
        (DrawTarget::Trigger, Some(id)) => edit_zone(|core| core.set_trigger_polygon(&id, &flat)),
        (DrawTarget::Trigger, None) => write_row(DrawTarget::Trigger, |core, id| {
            core.add_polygon_trigger(id, &kind, &flat)
        }),
    }
}

/// Mint an unused id in `collection`'s OWN namespace (`z{n}` for zones, `t{n}` for triggers), proven
/// unique against that collection's live map rather than assumed — undo frees ids and an IDB restore
/// can bring back a document that already used one. Each collection is a separate namespace (the slot
/// SoA does not contain either), so `mint_id`'s slot proof does not apply here.
fn mint_row_id(core: &MissionDocCore, collection: DrawTarget) -> String {
    let (json, prefix) = match collection {
        DrawTarget::Zone => (core.zones_json(), "z"),
        DrawTarget::Trigger => (core.triggers_json(), "t"),
    };
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&json)
            .ok()
            .and_then(|v| {
                v.as_object()
                    .map(|m| m.keys().map(ToString::to_string).collect())
            })
            .unwrap_or_default();
    (1u32..)
        .map(|n| format!("{prefix}{n}"))
        .find(|id| !existing.contains(id))
        .unwrap_or_else(|| format!("{prefix}1"))
}

/// Run a row-creating mutator (`add_*_zone` / `add_*_trigger`) under a minted id in `collection`,
/// then the shared dirty tail. The write txn is scoped so it is gone before `after_local_edit` opens
/// its read txn (the `mission_history` rule).
fn write_row(collection: DrawTarget, f: impl FnOnce(&MissionDocCore, &str)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let id = mint_row_id(core, collection);
        f(core, &id);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// Nudge the reactive doc tick so the Zones panel re-reads mid-draw. Cheaper and safer than
/// `after_local_edit`, which schedules a persist for a document that has not changed yet.
fn bump_doc_tick() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let n = ctx.doc_tick.get_untracked();
            ctx.doc_tick.set(n.wrapping_add(1));
        }
    });
}

/// Every authored zone, in `zones_json` map order, for the dock list.
#[must_use]
pub fn zone_rows() -> Vec<ZoneRow> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            let map: serde_json::Value = serde_json::from_str(&core.zones_json()).ok()?;
            let obj = map.as_object()?;
            let mut rows: Vec<ZoneRow> = obj
                .iter()
                .map(|(id, z)| {
                    let shape = z.get("shape");
                    let circle = shape.and_then(|s| s.get("circle")).and_then(|c| {
                        Some((
                            c.get("x")?.as_f64()?,
                            c.get("z")?.as_f64()?,
                            c.get("r")?.as_f64()?,
                        ))
                    });
                    let polygon = shape
                        .and_then(|s| s.get("polygon"))
                        .and_then(serde_json::Value::as_array)
                        .map(|ring| {
                            ring.iter()
                                .filter_map(|p| {
                                    let a = p.as_array()?;
                                    Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    ZoneRow {
                        id: id.clone(),
                        kind: z
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        label: z
                            .get("label")
                            .and_then(|l| l.as_str())
                            .map(ToString::to_string),
                        faction: z
                            .get("faction")
                            .and_then(|f| f.as_str())
                            .map(ToString::to_string),
                        rules: z.get("rules").cloned().unwrap_or(serde_json::Value::Null),
                        circle,
                        polygon,
                    }
                })
                .collect();
            // Map iteration order is not stable across a reload; sort so the dock list does not
            // reshuffle under the author between sessions.
            rows.sort_by(|a, b| a.id.cmp(&b.id));
            Some(rows)
        })
        .unwrap_or_default()
}

/// Run a mutator against an existing zone, then the shared dirty tail.
fn edit_zone(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// Attributes — schema `zone.type`. Refuses a value outside the schema enum, for the same reason
/// [`begin_zone_draw`] does.
pub fn set_zone_kind(id: &str, kind: &str) -> bool {
    if !zone_types().iter().any(|t| t == kind) {
        return false;
    }
    edit_zone(|core| core.set_zone_type(id, kind))
}

/// Attributes — schema `zone.label`. An EMPTY string and `None` are different authored states the
/// schema allows on purpose: `Some("")` writes an empty label (which the mod reads as "use the
/// PrettyZoneTitle fallback" and is a committed golden), `None` removes the key. The panel's Clear
/// control sends `None`; typing and clearing the box sends `Some("")`.
pub fn set_zone_label(id: &str, label: Option<String>) -> bool {
    edit_zone(|core| core.set_zone_label(id, label.as_deref()))
}

/// Attributes — schema `zone.faction` (a `factionKey` slug). `None` makes the zone faction-neutral.
pub fn set_zone_faction(id: &str, faction: Option<String>) -> bool {
    edit_zone(|core| core.set_zone_faction(id, faction.as_deref()))
}

/// Attributes — set or clear ONE `rules` key, read-modify-write over the OPAQUE object.
///
/// The key is a `$defs/zoneRules` property name supplied by the schema-driven panel; this function
/// never names one. `value: None` removes the key, and once the last key is gone the whole object
/// goes with it — `set_zone_rules` treats `{}` as a removal on purpose, because "the author cleared
/// every rule" and "the author never opened the panel" must stay the same document.
pub fn set_zone_rule(id: &str, key: &str, value: Option<serde_json::Value>) -> bool {
    let next = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let map: serde_json::Value = serde_json::from_str(&core.zones_json()).ok()?;
        let mut rules = map
            .get(id)?
            .get("rules")
            .and_then(|r| r.as_object().cloned())
            .unwrap_or_default();
        match value {
            Some(v) => {
                rules.insert(key.to_string(), v);
            }
            None => {
                rules.remove(key);
            }
        }
        Some(serde_json::Value::Object(rules).to_string())
    });
    let Some(next) = next else {
        return false;
    };
    edit_zone(|core| core.set_zone_rules(id, Some(&next)))
}

/// Attributes — delete the zone.
pub fn delete_zone(id: &str) -> bool {
    edit_zone(|core| core.remove_zone(id))
}

/// How many zones the document declares — backs "does this mission define a play area?".
#[must_use]
pub fn zone_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.doc.borrow().as_ref().map(MissionDocCore::zone_count))
            .unwrap_or(0)
    })
}

/* ═══════════════════ T-079 — triggers, the editor half (RIGHT-MODE-003 + CONN-TRG-OWNER-001) ═══════════════════ */

// The trigger AREA rides the shipped zone draw tool as a SECOND CONSUMER: the draw flow above is
// parameterized by [`DrawTarget`], so `begin_zone_draw(..., DrawTarget::Trigger)` and the reshape
// pair author `triggersById` through the SAME `advance_zone_draw` / `close_zone_polygon` state
// machine — there is no forked trigger draw. This block adds only the trigger-specific reads/writes
// (name / owner / activation / rules / delete), the owner picker's source, and the owner-link line
// geometry. Doc mutators are T-079's `MissionDocCore` block; the pure line projection is
// native-tested (the `ruler_tool::project_legs` idiom).

/// The three activation kinds the ticket names — a TYPED PLACEHOLDER: STORED, not evaluated (the
/// activation/effects runtime is T-676). This is the trigger's small closed vocabulary, the analogue
/// of `zone_types()` for the draw arm. Kept here (not read from the schema like `zone_types`) because
/// the schema does not declare `triggers` yet — T-706 (wave 120) will; when it does and declares an
/// `activation` enum, this should be replaced by a schema read exactly as `zone_types` is.
pub const TRIGGER_ACTIVATIONS: &[&str] = &["presence", "radio", "timer"];

/// One authored trigger, read back for the palette list and the Attributes panel. Mirrors
/// [`ZoneRow`], plus the trigger-only `name` / `owner_id` / `activation`.
#[derive(Clone, Debug, PartialEq)]
pub struct TriggerRow {
    pub id: String,
    pub name: Option<String>,
    /// CONN-TRG-OWNER-001 — the linked placed entity, or `None` (unowned). May be DANGLING: the
    /// entity it names can have been deleted; readers resolve it to nothing, they do not clear it.
    pub owner_id: Option<String>,
    /// One of [`TRIGGER_ACTIVATIONS`] (stored, not evaluated).
    pub activation: String,
    /// `rules` VERBATIM — the opaque `$defs/zoneRules`-shaped object, never parsed into named fields
    /// (the `ZoneRow::rules` reason: a typed mirror would be the second vocabulary T-241 prevents).
    pub rules: serde_json::Value,
    /// `Some((x, z, r))` for a circle.
    pub circle: Option<(f64, f64, f64)>,
    /// The ring for a polygon.
    pub polygon: Vec<(f64, f64)>,
}

impl TriggerRow {
    /// A one-line geometry summary for the palette row (the [`ZoneRow::shape_summary`] twin).
    #[must_use]
    pub fn shape_summary(&self) -> String {
        if let Some((x, z, r)) = self.circle {
            format!("circle r {r:.1} m @ {x:.0}, {z:.0}")
        } else if self.polygon.is_empty() {
            "no shape".to_string()
        } else {
            format!("polygon, {} vertices", self.polygon.len())
        }
    }

    /// The trigger's geometric CENTRE in world metres — a circle's centre, or a polygon's vertex
    /// mean. `None` for a shapeless row. This is the trigger end of the owner-link line.
    #[must_use]
    pub fn centre(&self) -> Option<(f64, f64)> {
        if let Some((x, z, _)) = self.circle {
            return Some((x, z));
        }
        if self.polygon.is_empty() {
            return None;
        }
        let n = self.polygon.len() as f64;
        let (sx, sz) = self
            .polygon
            .iter()
            .fold((0.0, 0.0), |(ax, az), (x, z)| (ax + x, az + z));
        Some((sx / n, sz / n))
    }
}

/// Every authored trigger, sorted by id, for the palette list. Off `triggers_json`, the
/// [`zone_rows`] twin.
#[must_use]
pub fn trigger_rows() -> Vec<TriggerRow> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            let map: serde_json::Value = serde_json::from_str(&core.triggers_json()).ok()?;
            let obj = map.as_object()?;
            let mut rows: Vec<TriggerRow> = obj
                .iter()
                .map(|(id, t)| {
                    let shape = t.get("shape");
                    let circle = shape.and_then(|s| s.get("circle")).and_then(|c| {
                        Some((
                            c.get("x")?.as_f64()?,
                            c.get("z")?.as_f64()?,
                            c.get("r")?.as_f64()?,
                        ))
                    });
                    let polygon = shape
                        .and_then(|s| s.get("polygon"))
                        .and_then(serde_json::Value::as_array)
                        .map(|ring| {
                            ring.iter()
                                .filter_map(|p| {
                                    let a = p.as_array()?;
                                    Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    TriggerRow {
                        id: id.clone(),
                        name: t
                            .get("name")
                            .and_then(|n| n.as_str())
                            .map(ToString::to_string),
                        owner_id: t
                            .get("ownerId")
                            .and_then(|o| o.as_str())
                            .map(ToString::to_string),
                        activation: t
                            .get("activation")
                            .and_then(|a| a.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        rules: t.get("rules").cloned().unwrap_or(serde_json::Value::Null),
                        circle,
                        polygon,
                    }
                })
                .collect();
            rows.sort_by(|a, b| a.id.cmp(&b.id));
            Some(rows)
        })
        .unwrap_or_default()
}

/// Run a mutator against an existing trigger, then the shared dirty tail. The [`edit_zone`] twin.
fn edit_trigger(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
}

/// Attributes — the trigger `name`. `None` REMOVES the key; the panel sends `None` on an emptied box.
pub fn set_trigger_name(id: &str, name: Option<String>) -> bool {
    edit_trigger(|core| core.set_trigger_name(id, name.as_deref()))
}

/// Attributes — the stored-not-evaluated `activation` kind. Refuses a value outside
/// [`TRIGGER_ACTIVATIONS`], the same "no invented value reaches the doc" discipline `set_zone_kind`
/// applies to `zone.type`.
pub fn set_trigger_activation(id: &str, activation: &str) -> bool {
    if !TRIGGER_ACTIVATIONS.contains(&activation) {
        return false;
    }
    edit_trigger(|core| core.set_trigger_activation(id, activation))
}

/// CONN-TRG-OWNER-001 (the picker's write) — assign / clear the owner edge. `Some(id)` is a placed
/// entity id from [`placed_owner_options`]; `None` clears the link. No referential check — a later
/// deletion of the owner is TOLERATED as a dangling edge (see [`MissionDocCore::set_trigger_owner`]).
pub fn set_trigger_owner(id: &str, owner_id: Option<String>) -> bool {
    edit_trigger(|core| core.set_trigger_owner(id, owner_id.as_deref()))
}

/// Attributes — set or clear ONE `rules` key, read-modify-write over the OPAQUE object. Reuses the
/// exact `set_zone_rule` shape: the key is a `$defs/zoneRules` property supplied by the schema-driven
/// panel; this never names one. `value: None` removes the key, and clearing the last key drops the
/// whole object (the "cleared all" == "never authored" identity).
pub fn set_trigger_rule(id: &str, key: &str, value: Option<serde_json::Value>) -> bool {
    let next = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let map: serde_json::Value = serde_json::from_str(&core.triggers_json()).ok()?;
        let mut rules = map
            .get(id)?
            .get("rules")
            .and_then(|r| r.as_object().cloned())
            .unwrap_or_default();
        match value {
            Some(v) => {
                rules.insert(key.to_string(), v);
            }
            None => {
                rules.remove(key);
            }
        }
        Some(serde_json::Value::Object(rules).to_string())
    });
    let Some(next) = next else {
        return false;
    };
    edit_trigger(|core| core.set_trigger_rules(id, Some(&next)))
}

/// Attributes — delete the trigger.
pub fn delete_trigger(id: &str) -> bool {
    edit_trigger(|core| core.remove_trigger(id))
}

/// How many triggers the document declares — backs the palette header count.
#[must_use]
pub fn trigger_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.doc.borrow().as_ref().map(MissionDocCore::trigger_count))
            .unwrap_or(0)
    })
}

/// One entry the Owner picker offers: a placed entity's id and a human label. CONN-TRG-OWNER-001 —
/// "listing placed entities (slots/vehicles by label)".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerOption {
    pub id: String,
    pub label: String,
}

/// The Owner picker's source: every PLACED slot and every PLACED vehicle, by label. Slots read off
/// the materialized SoA (role + id); vehicles off [`vehicle_rows`] filtered to those with a map
/// position (an ORBAT-only vehicle with no `xy` is not on the map, so it cannot own a placed
/// trigger). Sorted by label then id for a stable picker order.
#[must_use]
pub fn placed_owner_options() -> Vec<OwnerOption> {
    let mut out: Vec<OwnerOption> = Vec::new();
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        let soa = core.materialize();
        for (i, id) in soa.ids.iter().enumerate() {
            let role = {
                let idx = soa.role_idx.get(i).copied().unwrap_or(NONE_IDX);
                if idx == NONE_IDX {
                    String::new()
                } else {
                    soa.roles.get(idx as usize).cloned().unwrap_or_default()
                }
            };
            let label = if role.is_empty() {
                format!("Slot {id}")
            } else {
                format!("{role} ({id})")
            };
            out.push(OwnerOption {
                id: id.clone(),
                label,
            });
        }
    });
    // Placed vehicles (those with a map position). `vehicle_rows` opens its own borrow, so it is
    // called OUTSIDE the borrow above.
    for v in vehicle_rows() {
        if v.xy.is_none() {
            continue;
        }
        let short = v
            .resource_name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(&v.resource_name)
            .trim_end_matches(".et");
        let label = if short.is_empty() {
            format!("Vehicle {}", v.id)
        } else {
            format!("{short} ({})", v.id)
        };
        out.push(OwnerOption { id: v.id, label });
    }
    out.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
    out
}

/// Resolve a placed entity id to its world position (slot OR vehicle), or `None` if no such placed
/// entity exists. This is what makes a DANGLING owner render nothing: an `ownerId` pointing at a
/// deleted entity resolves to `None` here, so [`owner_line_world`] yields no line — no panic, no
/// stale draw. Slots come off the SoA; vehicles off `vehicle_rows` (placed ones only).
#[must_use]
fn placed_entity_pos(id: &str) -> Option<(f64, f64)> {
    let slot = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let soa = core.materialize();
        let row = soa.ids.iter().position(|s| s == id)?;
        Some((f64::from(soa.xs[row]), f64::from(soa.ys[row])))
    });
    if slot.is_some() {
        return slot;
    }
    vehicle_rows()
        .into_iter()
        .find(|v| v.id == id)
        .and_then(|v| v.xy)
}

/// CONN-TRG-OWNER-001 (the line's data) — the world-metre endpoints of the owner-link line for the
/// currently selected trigger: `(trigger centre, owner position)`. `None` — so the overlay draws
/// nothing — when there is no selected trigger, the selected trigger has no shape, it has no owner,
/// **or its owner is dangling** (the entity was deleted). The dangling case is the ticket's
/// tolerance requirement, and it falls out of [`placed_entity_pos`] returning `None`.
///
/// `selected_trigger` is the trigger id the palette currently has selected (session UI state the
/// panel owns), passed in so this stays a pure resolve over the doc + one id.
#[must_use]
pub fn owner_line_world(selected_trigger: Option<&str>) -> Option<((f64, f64), (f64, f64))> {
    let id = selected_trigger?;
    let row = trigger_rows().into_iter().find(|r| r.id == id)?;
    let owner_id = row.owner_id.as_deref()?;
    let centre = row.centre()?;
    // Dangling owner → None → no line. NOT an error, NOT a clear — the edge is kept in the doc.
    let owner = placed_entity_pos(owner_id)?;
    Some((centre, owner))
}

/* ═══════════════ T-069 — map markers: the four schema-carried fields ═══════════════ */
//
// ## Where a marker goes, and why it is NOT `markersById`
//
// `grep -c marker editor_ops.rs` was **0** before this block: T-345 shipped
// `set_faction_briefing_marker` / `remove_faction_briefing_marker` on `MissionDocCore` and nothing
// in the product ever called them. This block is the caller — the same "T-211 shipped eleven zone
// mutators and NOTHING called them" shape the zone tool above has.
//
// T-069's registry summary says free placement needs generic add/move/remove on the `markersById`
// ROOT map. **That premise is dead**, and it is dead in a checkable way rather than as a matter of
// taste. `mission.schema.json` declares markers in exactly ONE place — `$defs/briefing.markers[]`,
// an array of `$defs/marker` — and declares no top-level `markers` property at all. On the compile
// side `flatten_to_mod_document` deserialises `EditorPayload { editor: EditorGraph { factions,
// squads, slots } }`, which declares no root key whatsoever. So the root map is a closed
// hydrate→emit loop: a marker authored there survives a save and reaches no mod subsystem. The
// store carries that argument on `set_faction_briefing_marker`'s §authority note, and
// `a_marker_in_the_root_map_never_reaches_the_compiled_document` (T-069, `store.rs`) now pins it as
// a test instead of prose. **Author on the briefing, not the root.**
//
// The practical consequence for this surface: a marker is SIDE-SCOPED. It is placed under the
// active Eden side chip through [`ensure_side_faction`] — the same faction resolution a character
// place uses — because `bridgehead-at-levie` gives both sides different orders at the same
// coordinates, and the mod looks a briefing up by the joining player's faction.
//
// ## Scope: the four schema-carried fields, and not one more
//
// `$defs/marker` = `{x, z, icon, label}`. This block authors exactly those, mapping onto
// ATTR-FIELD-MRK-POSITION (`x`/`z`), -MRK-TYPE (`icon`) and -MRK-TEXT (`label`). The schema ALSO
// declares `size` / `rotationDeg` / `shape` / `area` — all stamped "T-673, lands after T-069" in
// their own descriptions — and this block deliberately writes none of them: they are marker STYLE
// and Eden's second Area-marker model, which is a different ticket. Nothing here widens the schema.
//
// ## The icon vocabulary is READ, never typed
//
// `$defs/marker.icon` is a CLOSED enum of 64 aliases (the `TBD_MarkerIcons.EnsureAliases` register
// keys). Before that enum existed a typo validated clean and then degraded at runtime to the
// fallback DOT glyph. So the picker offers the schema's list and nothing else, and
// [`crate::eden_dock_right::marker_icon_is_authorable`] — which reads the embedded schema, not a
// hand-typed copy — is the gate every write here passes through. An unknown alias is REFUSED at
// this boundary rather than stored: the store's mutator takes an `&str` and asks no questions (it
// must stay that way — its own tests author aliases this enum does not contain), so the product
// surface is where the vocabulary is enforced.

/// One authored marker, as the dock lists it. The `(faction_id, id)` pair is the address both store
/// mutators take, carried on every row so a listed marker can be moved, re-captioned, re-iconed or
/// deleted without a second lookup.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerRow {
    /// `factionsById` key — `faction-BLUFOR` / `-OPFOR` / `-INDFOR`.
    pub faction_id: String,
    /// Doc-internal id. Addressing only; it never reaches the wire (`$defs/marker` is
    /// `additionalProperties: false`, and the serde boundary drops the key for free).
    pub id: String,
    /// ATTR-FIELD-MRK-POSITION — world metres. `$defs/marker` is `{x, z}`: a marker is a MAP glyph,
    /// so it carries no height, unlike a slot's `{x, y, z}` position.
    pub x: f64,
    pub z: f64,
    /// ATTR-FIELD-MRK-TYPE — one of the 64 closed `$defs/marker.icon` aliases.
    pub icon: String,
    /// ATTR-FIELD-MRK-TEXT — the caption, stored VERBATIM (the mod caps it at render time; capping
    /// here would destroy the authored value in the one place the author could still fix it).
    pub label: String,
}

impl MarkerRow {
    /// The side chip this marker belongs to (`BLUFOR` / `OPFOR` / `INDFOR`), derived from the
    /// `faction-{SIDE}` id [`ensure_side_faction`] mints. Falls back to the whole id for a faction
    /// that came from a library import under some other naming.
    #[must_use]
    pub fn side(&self) -> &str {
        self.faction_id
            .strip_prefix("faction-")
            .unwrap_or(&self.faction_id)
    }

    /// The palette row's right-hand readout — ATTR-FIELD-MRK-POSITION at a glance.
    #[must_use]
    pub fn position_summary(&self) -> String {
        format!("{:.0}, {:.0}", self.x, self.z)
    }
}

/// Every authored marker on every faction, in the document's own order (faction groups sorted,
/// array order preserved inside each). Reads `briefing_marker_rows_json` — the typed reader T-345
/// never shipped, which is why its two mutators had no product caller.
#[must_use]
pub fn marker_rows() -> Vec<MarkerRow> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            Some(marker_rows_of(d.as_ref()?))
        })
        .unwrap_or_default()
}

/// The parse, taking the core directly. Split out because [`place_at_impl`] already holds the doc
/// borrow when it needs the list (to mint an unused id), and re-entering through [`marker_rows`]
/// there would re-open a borrow the place path is in the middle of.
fn marker_rows_of(core: &MissionDocCore) -> Vec<MarkerRow> {
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.briefing_marker_rows_json())
    else {
        return Vec::new();
    };
    let Some(arr) = rows.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|r| {
            Some(MarkerRow {
                faction_id: r.get("factionId")?.as_str()?.to_string(),
                id: r.get("id")?.as_str()?.to_string(),
                x: r.get("x")?.as_f64()?,
                z: r.get("z")?.as_f64()?,
                icon: r
                    .get("icon")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                label: r
                    .get("label")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

/// How many markers the document carries (the palette header readout).
#[must_use]
pub fn marker_count() -> usize {
    marker_rows().len()
}

/// RIGHT-MODE-006 — an icon press ARMS a marker place. Consumed by [`place_at`] on the next canvas
/// release, dropped by [`cancel_pending`] on a release over chrome: the one-shot palette-arm
/// lifecycle, so the map's `has_pending` ghost and the Ctrl multi-place both work with no
/// marker-specific branch.
///
/// Refuses an alias outside the closed `$defs/marker.icon` enum, so a bad vocabulary cannot even be
/// armed, let alone stored.
pub fn begin_place_marker(icon: String) {
    if !crate::eden_dock_right::marker_icon_is_authorable(&icon) {
        return;
    }
    arm(Pending::Marker(icon));
}

/// The armed marker icon, or `None`. Backs the panel's "click the map to drop it" hint.
#[must_use]
pub fn armed_marker_icon() -> Option<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Marker(icon)) => Some(icon.clone()),
            _ => None,
        }
    })
}

/// Mint a marker id unused anywhere in the document.
///
/// Unique across ALL factions, not just the one being written: the dock lists every side in one
/// list and addresses a row by its id alone, so two sides sharing `mk-1` would make the list
/// ambiguous even though the store (keyed by the pair) would be perfectly happy.
fn mint_marker_id(rows: &[MarkerRow]) -> String {
    let taken: std::collections::HashSet<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    let mut n: u32 = 1;
    loop {
        let id = format!("mk-{n}");
        if !taken.contains(id.as_str()) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

/// ATTR-FIELD-MRK-TYPE — re-icon an existing marker, keeping its position and caption.
///
/// Goes through the same upsert the place does, which is what makes it an in-place replace rather
/// than a delete-and-append: array order is the order the mod renders in.
#[must_use]
pub fn set_marker_icon(faction_id: &str, marker_id: &str, icon: &str) -> bool {
    if !crate::eden_dock_right::marker_icon_is_authorable(icon) {
        return false;
    }
    upsert_marker_field(faction_id, marker_id, |row| row.icon = icon.to_string())
}

/// ATTR-FIELD-MRK-TEXT — re-caption an existing marker. The label is stored VERBATIM; the mod caps
/// it at render time and the emitter applies that cap when it compiles, so capping here would
/// destroy the authored value in the one place the author could still see and fix it.
#[must_use]
pub fn set_marker_label(faction_id: &str, marker_id: &str, label: &str) -> bool {
    upsert_marker_field(faction_id, marker_id, |row| row.label = label.to_string())
}

/// ATTR-FIELD-MRK-POSITION — move a marker to `(x, z)` world metres. Non-finite input is refused
/// rather than stored: `$defs/marker` types both as `number`, and a NaN would serialise as JSON
/// `null` and fail the validator at save time, far from the box that produced it.
#[must_use]
pub fn set_marker_position(faction_id: &str, marker_id: &str, x: f64, z: f64) -> bool {
    if !x.is_finite() || !z.is_finite() {
        return false;
    }
    upsert_marker_field(faction_id, marker_id, |row| {
        row.x = x;
        row.z = z;
    })
}

/// Delete one marker. Its siblings and the briefing prose beside them are untouched
/// (`remove_faction_briefing_marker` reads the briefing and writes it back whole).
#[must_use]
pub fn remove_marker(faction_id: &str, marker_id: &str) -> bool {
    let exists = marker_rows()
        .iter()
        .any(|r| r.faction_id == faction_id && r.id == marker_id);
    if !exists {
        return false;
    }
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        if let Some(ctx) = guard.as_ref() {
            let d = ctx.doc.borrow();
            if let Some(core) = d.as_ref() {
                core.remove_faction_briefing_marker(faction_id, marker_id);
            }
        }
    });
    crate::mission_history::after_local_edit();
    true
}

/// The shared edit body: read the row, apply `edit`, write it back through the store's UPSERT.
///
/// Read-modify-write rather than a per-field mutator because the store's writer takes the whole
/// `{x, z, icon, label}` tuple — it replaces the row in place. Editing one field therefore means
/// carrying the other three across, and doing that in one function is what stops a future caller
/// from re-captioning a marker back to the origin.
fn upsert_marker_field(
    faction_id: &str,
    marker_id: &str,
    edit: impl FnOnce(&mut MarkerRow),
) -> bool {
    let Some(mut row) = marker_rows()
        .into_iter()
        .find(|r| r.faction_id == faction_id && r.id == marker_id)
    else {
        return false;
    };
    edit(&mut row);
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        if let Some(ctx) = guard.as_ref() {
            let d = ctx.doc.borrow();
            if let Some(core) = d.as_ref() {
                core.set_faction_briefing_marker(
                    faction_id, marker_id, row.x, row.z, &row.icon, &row.label,
                );
            }
        }
    });
    crate::mission_history::after_local_edit();
    true
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// T-697 — THE DOCUMENT INDEX (3den E4 / 3DEN-TOOL-011)
// ═════════════════════════════════════════════════════════════════════════════════════════════════
//
// The read half of document search: every placed thing in the mission, projected into the ONE row
// type the pure search in `eden_dock_left` consumes. The projection lives here because the doc
// handles are the wasm-only `!Send` `Rc`s this module exists to hold; it is deliberately a READ —
// nothing below opens a transaction, so nothing below can enter the undo stack.
//
// **EVERY KIND, OR THE SEARCH IS A LIE.** The eight readers are the eight collections an author can
// place into: `slots`, `vehiclesById`, `entitiesById`, briefing `markers`, `zones`, `triggers`,
// `commentsById` and `editorLayersById`. A ninth collection appearing without a case here would be
// silently unfindable, which is exactly the failure the ticket names.
//
// **BORROW DISCIPLINE.** `vehicle_rows` / `zone_rows` / `trigger_rows` / `marker_rows` each open
// their OWN `OPS_CTX` + doc borrow, so they are called AFTER the single borrow below has been
// dropped — the module's standing rule (see `placed_owner_options`, which does the same dance).

/// T-697 — a `faction-BLUFOR` id (or a raw side/library key) as the side label the search and the
/// selection filter group by. Empty in, empty out: a row that belongs to no faction says so.
fn side_label(raw: &str) -> String {
    raw.strip_prefix("faction-").unwrap_or(raw).to_uppercase()
}

/// T-697 — a non-empty display label, or the fallback the row's own panel would show.
fn or_fallback(label: &str, fallback: &str) -> String {
    let t = label.trim();
    if t.is_empty() {
        fallback.to_string()
    } else {
        t.to_string()
    }
}

/// T-697 — push `(field, value)` only when `value` is non-blank. A blank attribute is not a
/// searchable attribute, and keeping it would let an empty query-side value match the empty string
/// and report a field that holds nothing.
fn push_text(text: &mut Vec<(&'static str, String)>, field: &'static str, value: &str) {
    let v = value.trim();
    if !v.is_empty() {
        text.push((field, v.to_string()));
    }
}

/// T-697 — **every placed entity in the mission, with its text attributes.** The input to
/// [`crate::eden_dock_left::search_document`].
///
/// The per-kind attribute sets are the fields an author actually types into and would search by —
/// a slot's role/callsign/tag/rank/description, a marker's caption, a zone's label, a trigger's
/// name, a comment's title and body — plus, on every row, its ID and (where it has one) the tail of
/// its Enfusion class name. The id is in the set because quoting an id out of a validation finding
/// and pasting it into the search box is a real workflow; the class tail is in it so a plain
/// `UAZ` finds a vehicle whose only authored text is its `resourceName`.
#[must_use]
pub fn document_entities() -> Vec<crate::eden_dock_left::DocEntity> {
    use crate::eden_dock_left::{DocEntity, DocKind};

    let mut out: Vec<DocEntity> = Vec::new();

    // ── Pass 1: everything reachable from ONE doc borrow ─────────────────────────────────────────
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };

        // A slot's side is squad → faction → key: the join the ORBAT tree already makes.
        let side_of_faction: HashMap<String, String> = faction_rows(core)
            .into_iter()
            .map(|f| {
                let key = if f.key.is_empty() { f.name } else { f.key };
                (f.id, side_label(&key))
            })
            .collect();
        let side_of_squad: HashMap<String, String> = squad_rows(core)
            .into_iter()
            .map(|s| {
                let side = side_of_faction
                    .get(&s.faction_id)
                    .cloned()
                    .unwrap_or_default();
                (s.id, side)
            })
            .collect();

        let raw = raw_slot_rows(core);
        for s in slot_details(core) {
            let asset_id = row_str(&raw, &s.id, "assetId");
            let description = row_str(&raw, &s.id, "description");
            let mut text = Vec::new();
            push_text(&mut text, "role", &s.role);
            push_text(&mut text, "callsign", &s.callsign);
            push_text(&mut text, "tag", &s.tag);
            push_text(&mut text, "rank", &s.rank);
            push_text(&mut text, "description", &description);
            push_text(&mut text, "loadout", &s.summary);
            push_text(
                &mut text,
                "class",
                crate::asset_catalog::classname_tail(&asset_id),
            );
            push_text(&mut text, "id", &s.id);
            out.push(DocEntity {
                label: or_fallback(&s.role, &format!("Slot {}", s.id)),
                faction: side_of_squad.get(&s.squad_id).cloned().unwrap_or_default(),
                class_name: asset_id,
                kind: DocKind::Slot,
                id: s.id,
                text,
            });
        }

        // Placed world objects (T-254 `entitiesById`). No typed reader exists for them — the panels
        // that render objects render the CATALOGUE, not the placed rows — so this is the read.
        if let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) {
            if let Some(map) = root.get("entitiesById").and_then(|v| v.as_object()) {
                for (id, v) in map {
                    let s = |k: &str| {
                        v.get(k)
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string()
                    };
                    let alias = s("alias");
                    let resource_name = s("resourceName");
                    let mut text = Vec::new();
                    push_text(&mut text, "alias", &alias);
                    push_text(
                        &mut text,
                        "class",
                        crate::asset_catalog::classname_tail(&resource_name),
                    );
                    push_text(&mut text, "id", id);
                    out.push(DocEntity {
                        id: id.clone(),
                        kind: DocKind::Object,
                        label: or_fallback(&alias, &format!("Object {id}")),
                        faction: side_label(&s("faction")),
                        class_name: resource_name,
                        text,
                    });
                }
            }
        }

        for cm in comment_details(core) {
            let mut text = Vec::new();
            push_text(&mut text, "title", &cm.title);
            push_text(&mut text, "note", &cm.tooltip);
            push_text(&mut text, "id", &cm.id);
            out.push(DocEntity {
                label: or_fallback(&cm.title, &format!("Comment {}", cm.id)),
                kind: DocKind::Comment,
                class_name: String::new(),
                faction: String::new(),
                id: cm.id,
                text,
            });
        }

        for l in layer_rows(core) {
            let mut text = Vec::new();
            push_text(&mut text, "name", &l.name);
            push_text(&mut text, "id", &l.id);
            out.push(DocEntity {
                label: or_fallback(&l.name, &format!("Layer {}", l.id)),
                kind: DocKind::Layer,
                class_name: String::new(),
                faction: String::new(),
                id: l.id,
                text,
            });
        }
    });

    // ── Pass 2: the readers that open their own borrow ───────────────────────────────────────────
    for v in vehicle_rows() {
        let tail = crate::asset_catalog::classname_tail(&v.resource_name).to_string();
        let mut text = Vec::new();
        push_text(&mut text, "class", &tail);
        push_text(&mut text, "id", &v.id);
        out.push(DocEntity {
            label: or_fallback(&tail, &format!("Vehicle {}", v.id)),
            kind: DocKind::Vehicle,
            faction: side_label(&v.faction_id),
            class_name: v.resource_name,
            id: v.id,
            text,
        });
    }
    for z in zone_rows() {
        let label = z.label.clone().unwrap_or_default();
        let mut text = Vec::new();
        push_text(&mut text, "label", &label);
        push_text(&mut text, "type", &z.kind);
        push_text(&mut text, "id", &z.id);
        out.push(DocEntity {
            label: or_fallback(&label, &format!("Zone {}", z.id)),
            kind: DocKind::Zone,
            class_name: String::new(),
            faction: side_label(z.faction.as_deref().unwrap_or_default()),
            id: z.id,
            text,
        });
    }
    for t in trigger_rows() {
        let name = t.name.clone().unwrap_or_default();
        let mut text = Vec::new();
        push_text(&mut text, "name", &name);
        push_text(&mut text, "activation", &t.activation);
        push_text(&mut text, "id", &t.id);
        out.push(DocEntity {
            label: or_fallback(&name, &format!("Trigger {}", t.id)),
            kind: DocKind::Trigger,
            class_name: String::new(),
            faction: String::new(),
            id: t.id,
            text,
        });
    }
    for m in marker_rows() {
        let mut text = Vec::new();
        push_text(&mut text, "caption", &m.label);
        push_text(&mut text, "icon", &m.icon);
        push_text(&mut text, "id", &m.id);
        out.push(DocEntity {
            label: or_fallback(&m.label, &or_fallback(&m.icon, &format!("Marker {}", m.id))),
            kind: DocKind::Marker,
            class_name: String::new(),
            faction: side_label(&m.faction_id),
            id: m.id,
            text,
        });
    }
    out
}

/// T-697 — the CURRENT SELECTION, projected exactly as [`document_entities`] projects the document.
///
/// Derived from the document index rather than re-read per kind, so the selection filter's chips and
/// the search's rows can never disagree about what an entity's type or faction is. Selection order
/// is not preserved (the document order is): the chips are counts and id sets, and nothing
/// downstream reads a selection as a sequence.
#[must_use]
pub fn selection_entities() -> Vec<crate::eden_dock_left::DocEntity> {
    let sel: Vec<String> = OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .map(|ctx| ctx.selection.borrow().clone())
            .unwrap_or_default()
    });
    if sel.is_empty() {
        return Vec::new();
    }
    document_entities()
        .into_iter()
        .filter(|e| sel.iter().any(|id| id == &e.id))
        .collect()
}

/// T-697 — replace the selection with `ids` (the selection filter's apply), returning how many
/// entities ended up selected.
///
/// Goes through [`set_slot_selection`], the SAME selection-only tail a folder click and a map click
/// take (engine tint + SEL readout + dock highlight, no doc edit ⇒ **no undo step**). Narrowing a
/// selection is not authored content, so it must not enter the history — the T-642 line, and the
/// reason this is not a mutator. An empty `ids` is refused rather than obeyed: "narrow to nothing"
/// is never what an author meant by a filter chip, and the pure `selection_facets` never emits one.
pub fn set_selection_ids(ids: Vec<String>) -> usize {
    if ids.is_empty() {
        return 0;
    }
    let n = ids.len();
    set_slot_selection(ids);
    n
}
