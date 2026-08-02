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
    apply_faction_library, FactionLibraryInput, FactionLibraryRole, FactionLibraryVehicle,
    MissionDocCore, APPLY_ANCHOR_X, APPLY_ANCHOR_Y, NONE_IDX,
};

use crate::asset_catalog::PlacePayload;
use crate::dto::{FactionDoc, FactionRole, FactionVehicle};
use crate::mission_doc::DocHandle;
use crate::outliner::{build_outliner, LayerRow, OutlinerNode, SlotRow};
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
    /// T-582 — RESHAPE target. `None` creates a new zone (`add_*_zone`); `Some(id)` re-shapes that
    /// existing one (`set_zone_circle` / `set_zone_polygon`).
    ///
    /// The two reshape mutators replace the WHOLE `shape` object, which is why a circle can become a
    /// polygon and back: `$defs/shape` is a `oneOf`, so a row carrying both keys is schema-INVALID,
    /// and a partial edit would leave exactly that. Re-shaping through them keeps every other
    /// authored field — label, faction, and the opaque `rules` — untouched, which is the whole point
    /// of offering reshape instead of delete-and-redraw.
    pub target: Option<String>,
}

/// One slot's editable attributes, read from the materialized SoA for the Attributes modal.
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

/// Delete/Backspace — remove the selected slots in one undoable step (React `removeEntities`).
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
            core.remove_slots(ids);
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
            zs.push(0.0); // DEM not ready — byte-parity with the flat-map case
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

/* ───────────────────────── Attributes modal (T-159.26 / .23 spec) ───────────────────────── */

/// Open Attributes for `id` — the React dbl-click contract (A1): a multi-selection (>1) suppresses
/// the open. Selects the slot (replace) so the modal, SEL readout, and tint agree.
/// Leaves the Attributes tab index alone (default Identity until the user changes it).
pub fn open_attributes(id: String) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        if ctx.selection.borrow().len() > 1 {
            return;
        }
        *ctx.selection.borrow_mut() = vec![id.clone()];
        let ids = ctx.selection.borrow().clone();
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_selection(ids);
        }
        ctx.attrs_open.set(Some(id));
    });
    crate::mission_history::refresh_selection();
}

/// T-180.9 — Open Attributes on the Arsenal tab (`TABS[3]`) for `id`. Same multi-select suppress
/// and selection replace as [`open_attributes`].
pub fn open_arsenal(id: String) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        if ctx.selection.borrow().len() > 1 {
            return;
        }
        *ctx.selection.borrow_mut() = vec![id.clone()];
        let ids = ctx.selection.borrow().clone();
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_selection(ids);
        }
        ctx.attrs_tab.set(3);
        ctx.attrs_open.set(Some(id));
    });
    crate::mission_history::refresh_selection();
}

/// Close the modal (Esc / backdrop / close button).
pub fn close_attributes() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.attrs_open.set(None);
        }
    });
}

/// Read one slot's editable attributes from the materialized SoA (the modal's field values).
/// `None` when the slot no longer exists (undone away while open → the modal closes).
pub fn read_attrs(id: &str) -> Option<SlotAttrs> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let soa = core.materialize();
        let row = soa.ids.iter().position(|s| s == id)?;
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
        })
    })
}

/// Attributes Transform commit — `update_slot_position` (x/y clamp to terrain bounds, rotation
/// normalizes, manual z sticks) + the shared post-change tail (A4: one commit = one undo step).
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
        // Clamp to the mission's terrain bounds (React clamps to the live terrain; the seed's
        // null meta falls through to everon 12800², compile.rs's own default).
        let terrain = serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
            .ok()
            .and_then(|v| v.get("meta")?.get("terrain")?.as_str().map(str::to_string))
            .unwrap_or_default();
        let b = map_engine_core::mission::compile::terrain_bounds(&terrain);
        core.update_slot_position(id, x, y, z, rotation, b[2], b[3]);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
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
        Some(json) => {
            core.update_slot_loadout(id, Some(json));
            true
        }
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
        core.update_slot_loadout(id, Some(json.clone()));
        Some(json)
    });
    if seeded.is_some() {
        crate::mission_history::after_local_edit();
    }
    seeded
}

/// Set/clear a slot's `loadout` (Arsenal commit) + the shared tail (one undo step). `None`/empty
/// clears the key.
pub fn set_loadout(id: &str, loadout_json: Option<String>) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.update_slot_loadout(id, loadout_json);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
}

/// Attributes Identity/stance commit — `update_slot(role/tag/stance)` + the shared tail.
pub fn attrs_update_slot(
    id: &str,
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
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
        core.update_slot(id, role, tag, stance);
        true
    });
    if did {
        crate::mission_history::after_local_edit();
    }
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
                        build_outliner(&layer_rows(core), &slots),
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
        ctx.selected_ids.set(ctx.selection.borrow().clone());
        ctx.doc_tick
            .set(ctx.doc_tick.get_untracked().wrapping_add(1));
    });
}

/// Selection-only dock mirror: push `selected_ids` (the trees' fine-grained `is_sel` source)
/// without rebuilding the node trees. Pairs with `mission_history::refresh_selection` (T-172 B8).
pub fn refresh_selection_mirrors() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.selected_ids.set(ctx.selection.borrow().clone());
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

/// Arm a place. Objects mode only accepts [`Pending::Object`]; side modes reject Object so a
/// leftover Objects arm cannot commit after the chip switches away.
fn arm(pending: Pending) {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let objects = ctx.objects_mode.get_untracked();
            let ok = match &pending {
                Pending::Object(_) => objects,
                Pending::Character(_) | Pending::Vehicle(_) => !objects,
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
pub fn cancel_pending() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let mut p = ctx.pending.borrow_mut();
            if matches!(*p, Some(Pending::Zone(_))) {
                return;
            }
            *p = None;
        }
    });
}

/// Mint an unused slot id. The counter keeps this O(1) amortized, but uniqueness is **proven**
/// against the live doc rather than assumed: undo frees ids, and an IDB restore can bring back a
/// document that already used `n0`.
fn mint_id(ctx: &OpsCtx, core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> = core.materialize().ids.into_iter().collect();
    loop {
        let id = format!("n{}", ctx.next_id.get());
        ctx.next_id.set(ctx.next_id.get().saturating_add(1));
        if !existing.contains(&id) {
            return id;
        }
    }
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
    core.add_vehicle(
        vehicle_id,
        resource_name,
        Some(x),
        Some(y),
        Some(0.0),
        Some(0.0),
    );
    core.set_vehicle_faction(vehicle_id, &faction_id);
    // T-076 (RIGHT-CREW-001) — stamp the manned/unmanned placement intent. `set_vehicle_crewed`
    // omits the key for the with-crew default, so a manned placement's row is unchanged; only the
    // toggle-off case writes `crewed: false`. Same undo step as the place (same borrow scope).
    core.set_vehicle_crewed(vehicle_id, with_crew);
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

/// Arm a zone draw. `kind` must come from [`zone_types`] — this refuses anything else rather than
/// letting an invented `zone.type` reach the document, where it would save 201 and then 500
/// `/compiled` forever (T-581 measured exactly that for `"capture"`).
pub fn begin_zone_draw(kind: &str, shape: ZoneShape) -> bool {
    if !zone_types().iter().any(|t| t == kind) {
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
        }));
        true
    })
}

/// T-582 — arm a RESHAPE of an existing zone: the next clicks replace its `shape` through
/// `set_zone_circle` / `set_zone_polygon` instead of minting a new row.
///
/// The gesture is identical to a fresh draw (circle: centre then rim; polygon: vertices then Close),
/// so there is one geometry path and one set of guards — a reshape cannot produce the `r → 0.0`
/// circle or the two-vertex ring any more than a create can. `kind` is read from the live document
/// rather than taken from the caller: reshaping is a geometry edit, and silently retyping a zone
/// because the dock's type picker had drifted would be a different, invisible edit.
pub fn begin_zone_reshape(zone_id: &str, shape: ZoneShape) -> bool {
    let Some(row) = zone_rows().into_iter().find(|r| r.id == zone_id) else {
        return false;
    };
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        *ctx.pending.borrow_mut() = Some(Pending::Zone(ZoneDraft {
            kind: row.kind,
            shape,
            centre: None,
            verts: Vec::new(),
            target: Some(zone_id.to_string()),
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
                        let (kind, target) = (d.kind.clone(), d.target.clone());
                        *p = None;
                        Commit::Circle { kind, geom, target }
                    }
                },
            },
        }
    });
    match commit {
        Commit::Circle {
            kind,
            geom: (cx, cz, r),
            target,
        } => match target {
            // Reshape: replaces the whole `shape` object, so label / faction / rules survive and
            // the `oneOf` can never end up with both branches present.
            Some(id) => edit_zone(|core| core.set_zone_circle(&id, cx, cz, r)),
            None => write_zone(|core, id| core.add_circle_zone(id, &kind, cx, cz, r)),
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
        let out = (d.kind.clone(), d.verts.clone(), d.target.clone());
        *p = None;
        Some(out)
    });
    let Some((kind, verts, target)) = taken else {
        return false;
    };
    let flat = polygon_flat(&verts);
    match target {
        // Reshape — see [`begin_zone_reshape`]: whole-`shape` replacement, so a circle becomes a
        // polygon without leaving both `oneOf` branches on the row.
        Some(id) => edit_zone(|core| core.set_zone_polygon(&id, &flat)),
        None => write_zone(|core, id| core.add_polygon_zone(id, &kind, &flat)),
    }
}

/// Mint an unused zone id. `zones` is its OWN id namespace (`mint_id` proves uniqueness against the
/// slot SoA, which does not contain zones), and uniqueness is proven against the live map rather
/// than assumed — undo frees ids and an IDB restore can bring back a document that already used one.
fn mint_zone_id(core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&core.zones_json())
            .ok()
            .and_then(|v| {
                v.as_object()
                    .map(|m| m.keys().map(ToString::to_string).collect())
            })
            .unwrap_or_default();
    (1u32..)
        .map(|n| format!("z{n}"))
        .find(|id| !existing.contains(id))
        .unwrap_or_else(|| "z1".to_string())
}

/// Run a zone-creating mutator under a minted id, then the shared dirty tail. The write txn is
/// scoped so it is gone before `after_local_edit` opens its read txn (the `mission_history` rule).
fn write_zone(f: impl FnOnce(&MissionDocCore, &str)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let id = mint_zone_id(core);
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
