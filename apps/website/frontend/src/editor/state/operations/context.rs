//! T-934.7 — ops context & plumbing half of the old `state/operations.rs`: the `OpsCtx`
//! thread-local, `set_ctx`, registered overlay/panel signals, environment + title reads,
//! the dock-mirror row builders and `refresh_docks` / `bump_doc_tick`.
//! Split from `operations.rs` (same-commit move); the façade re-exports keep every
//! `crate::editor::state::operations::X` path stable.

use crate::editor::arsenal::asset_catalog::PlacePayload;
use crate::editor::eden_chrome::ZoneShape;
use crate::editor::panels::outliner::{
    build_outliner_with_comments, LayerRow, OutlinerNode, SlotRow,
};
use crate::editor::panels::zones_panel::DrawTarget;
use crate::editor::state::doc_host::DocHandle;
use crate::editor::state::history as mission_history;
use crate::editor::tools::select_tool::{EngineHandle, SelectionHandle};
use leptos::prelude::{GetUntracked, RwSignal, Set};
use map_engine_core::doc::{MissionDocCore, NONE_IDX};
use std::cell::{Cell, RefCell};

#[allow(unused_imports)]
use super::{attrs::*, cargo::*, compositions::*, entity::*, transform::*};

pub(super) struct OpsCtx {
    pub(super) doc: DocHandle,
    pub(super) engine: EngineHandle,
    pub(super) selection: SelectionHandle,
    /// The drop target folder (React's `activeLayerId`). `None` ⇒ the place path resolves one.
    pub(super) active_layer: RwSignal<Option<String>>,
    /// T-180.1 — active Eden side for place (`BLUFOR`/`OPFOR`/`INDFOR`). Chips write this in T-180.5.
    pub(super) active_side: RwSignal<String>,
    /// T-180.5 / T-254 — Objects chip: when true, the right dock shows the Objects palette and
    /// [`begin_place_object`] / [`place_at`] mint `entitiesById` rows.
    pub(super) objects_mode: RwSignal<bool>,
    /// Dock mirrors — `MissionDocCore` has no change subscription, so these are pushed from
    /// [`refresh_docks`] at every mutation site, like the OBJ/SEL readouts.
    pub(super) outliner_nodes: RwSignal<Vec<OutlinerNode>>,
    /// T-168 — the ORBAT dock tree mirror (faction/squad/slot), rebuilt alongside `outliner_nodes`.
    pub(super) orbat_nodes: RwSignal<Vec<OutlinerNode>>,
    pub(super) selected_ids: RwSignal<Vec<String>>,
    /// T-159.26 — the Attributes modal's open slot id (`None` = closed). The dbl-click pick and the
    /// outliner activate set it; the modal component reads it reactively.
    pub(super) attrs_open: RwSignal<Option<String>>,
    /// T-180.9 — Attributes tab index (`TABS[3] == "Arsenal"`). Lifted so [`open_arsenal`] can
    /// select the Arsenal tab; [`open_attributes`] leaves it alone.
    pub(super) attrs_tab: RwSignal<usize>,
    /// T-159.26 — reactive doc-change tick (the modal's re-read trigger; `doc_ver` is non-reactive).
    pub(super) doc_tick: RwSignal<u64>,
    /// The in-flight palette drag: `Some` between a leaf `pointerdown` and the canvas `pointerup`.
    pub(super) pending: RefCell<Option<Pending>>,
    /// Monotonic minter for placed-slot ids; [`mint_id`] still proves uniqueness against the doc.
    pub(super) next_id: Cell<u32>,
}

/// T-215 — which palette armed the in-flight place. The two tabs hand the map the same
/// [`PlacePayload`] but write **different entities**: a Factions leaf becomes a `slots` row through
/// `place_character_under_side`, a Vehicles leaf becomes a `vehiclesById` row through `add_vehicle`.
///
/// The discriminant lives here, on the armed value, rather than on a separate "current tab" signal:
/// the tab can change (or the dock can unmount) between the leaf's `pointerdown` and the canvas's
/// `pointerup`, and a place must commit the entity the operator actually picked up.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Pending {
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

thread_local! {
    pub(super) static OPS_CTX: RefCell<Option<OpsCtx>> = const { RefCell::new(None) };
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
// names the type through `crate::editor::mission_editor::AssetPickerState`.
use crate::editor::mission_editor::AssetPickerState;

thread_local! {
    /// T-647 PLACE-003 — the picker signal, installed once from `mission_editor::on_load` (the same
    /// pattern as [`crate::editor::panels::context_menu::set_menu_signal`] and the Attributes `attrs_open`). Kept as
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

/// The doc's terrain + environment fields — relocated to the always-compiled [`crate::core::dto`] so the
/// native `eden_chrome` view shell can build a default; re-exported here for wasm callers.
pub use crate::core::dto::MissionEnv;

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
        mission_history::after_local_edit();
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
        mission_history::after_local_edit();
    }
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
pub(super) fn open_attrs_modal(id: String, arsenal_tab: bool) {
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
    mission_history::refresh_selection();
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

/// Close the modal (Esc / backdrop / close button).
pub fn close_attributes() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.attrs_open.set(None);
        }
    });
}

/// Read the doc's `editorLayers` as rows for the tree. There is **no** public `editor_layers`
/// accessor on the core, and `materialize()`'s `layers` dict holds layer *ids* only — the names /
/// `parentId` / `entityIds` live in `small_maps_json()`'s `editorLayersById` (`store.rs:153`).
///
/// Sorted by id so the tree order can't depend on `serde_json`'s map type (`preserve_order` or not).
pub(super) fn layer_rows(core: &MissionDocCore) -> Vec<LayerRow> {
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
pub(super) fn faction_rows(
    core: &MissionDocCore,
) -> Vec<crate::editor::panels::outliner::FactionRow> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return Vec::new();
    };
    let Some(map) = root.get("factionsById").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    map.values()
        .filter_map(|v| {
            let o = v.as_object()?;
            Some(crate::editor::panels::outliner::FactionRow {
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
pub(super) fn squad_rows(core: &MissionDocCore) -> Vec<crate::editor::panels::outliner::SquadRow> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return Vec::new();
    };
    let Some(map) = root.get("squadsById").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    map.values()
        .filter_map(|v| {
            let o = v.as_object()?;
            Some(crate::editor::panels::outliner::SquadRow {
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
pub(super) fn slot_rows(core: &MissionDocCore) -> Vec<SlotRow> {
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
                        crate::editor::panels::outliner::build_orbat(
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
///
/// **T-788 F-29** — this is the one funnel every *selection-only* change flows through
/// (`mission_history::refresh_selection` → here), so it is where an open Attributes modal is kept
/// honest against a selection that moved under it. The modal body re-reads the live target set
/// ([`attrs_multi_ids`] / [`attrs_selection_len`] / [`read_attrs_diff`]) on every render, but its
/// only render triggers are `attrs_open` and `doc_tick` — and a Ctrl+A (or any pick) bumps NEITHER,
/// so the panel used to keep showing the single slot it opened on while the SEL count climbed to 9.
/// Re-poking `attrs_open` forces exactly that re-render (the header flips to `N slots · multi-edit`
/// within the same frame), and if the id the modal is editing is no longer selected, the modal
/// closes rather than stranding a single-edit view on a slot the operator just deselected.
///
/// It lives HERE and not in [`mirror_selection`] on purpose: `mirror_selection` also runs from
/// [`refresh_docks`] on every DOCUMENT change (each commit), and re-poking `attrs_open` there would
/// re-run the modal's `opts.reset()` effect and wipe the operator's per-field ticks on every commit
/// — the T-649 latch is deliberately re-armed off `attrs_open` alone so a commit leaves the ticks
/// intact. A selection change re-arming them is correct (the target set changed); a commit doing so
/// is the bug that separation avoids.
pub fn refresh_selection_mirrors() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            mirror_selection(ctx);
            if let Some(open_id) = ctx.attrs_open.get_untracked() {
                if ctx.selection.borrow().iter().any(|s| *s == open_id) {
                    // Still editing a selected slot — re-render the open modal against the live
                    // selection so single-edit flips to multi (and back) as the set changes.
                    ctx.attrs_open.set(Some(open_id));
                } else {
                    // The modal's target left the selection — close it instead of showing a stale
                    // single slot (spec F-29: "re-render against the live selection OR close").
                    ctx.attrs_open.set(None);
                }
            }
        }
    });
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

/// Nudge the reactive doc tick so the Zones panel re-reads mid-draw. Cheaper and safer than
/// `after_local_edit`, which schedules a persist for a document that has not changed yet.
pub(super) fn bump_doc_tick() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let n = ctx.doc_tick.get_untracked();
            ctx.doc_tick.set(n.wrapping_add(1));
        }
    });
}
