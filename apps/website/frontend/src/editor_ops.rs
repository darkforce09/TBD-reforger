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
    /// T-180.5 — Objects chip stub: when true, [`begin_place`] / [`place_at`] no-op.
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
    pending: RefCell<Option<PlacePayload>>,
    /// Monotonic minter for placed-slot ids; [`mint_id`] still proves uniqueness against the doc.
    next_id: Cell<u32>,
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
                view_distance: env
                    .and_then(|e| e.get("viewDistance"))
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(1600),
                thermals: env
                    .and_then(|e| e.get("thermals"))
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
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
        let g = |v: &serde_json::Value, k: &str| {
            v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
        };
        let gp = |v: &serde_json::Value, k: &str| {
            v.get("position")
                .and_then(|p| p.get(k))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
        };
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

/// Palette leaf `pointerdown` → arm a place. Consumed by [`place_at`] on a canvas release, or
/// dropped by [`cancel_pending`] on a release over chrome.
///
/// T-180.5 — no-op while the Objects chip is active (stub catalog; place must not panic).
pub fn begin_place(payload: PlacePayload) {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            if ctx.objects_mode.get_untracked() {
                *ctx.pending.borrow_mut() = None;
                return;
            }
            *ctx.pending.borrow_mut() = Some(payload);
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
pub fn cancel_pending() {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            *ctx.pending.borrow_mut() = None;
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

/// G6 — add a role (slot) into an existing squad; default role Rifleman. Not `place_character_under_side`.
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
        core.add_slot(
            &slot_id, &squad_id, &layer_id, index, &role, None, None, 0.0, 0.0, 0.0, 0.0,
        );
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
pub fn orbat_apply_faction(side: String, doc: FactionDoc) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
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
        apply_faction_library(core, &side, &layer_id, &input).is_ok()
    });
    if did {
        crate::mission_history::after_local_edit();
    }
    did
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

/// T-180.8 — inverse of Apply: build a FactionDoc from the live side graph (Save / Save as).
pub fn faction_doc_from_side(side: &str) -> Option<FactionDoc> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        Some(faction_doc_from_side_core(core, side))
    })
}

fn faction_doc_from_side_core(core: &MissionDocCore, side: &str) -> FactionDoc {
    let factions = faction_rows(core);
    let squads = squad_rows(core);
    let faction = factions.iter().find(|f| f.key == side);
    let name = faction
        .map(|f| f.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| side.to_string());
    let squad_ids: Vec<String> = faction.map(|f| f.squad_ids.clone()).unwrap_or_default();
    let Ok(slots_root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return FactionDoc {
            side: side.into(),
            name,
            ..Default::default()
        };
    };
    let Ok(small) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return FactionDoc {
            side: side.into(),
            name,
            ..Default::default()
        };
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
                label: None,
            });
        }
    }
    FactionDoc {
        side: side.into(),
        name,
        emblem: None,
        roles,
        vehicles,
    }
}

fn squad_anchor_xy(core: &MissionDocCore, sq: &crate::outliner::SquadRow) -> Option<(f64, f64)> {
    let anchor_id = if !sq.leader_slot_id.is_empty() {
        sq.leader_slot_id.clone()
    } else {
        sq.slot_ids.first()?.clone()
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return None;
    };
    let pos = root.get(&anchor_id)?.get("position")?;
    let x = pos.get("x")?.as_f64()?;
    let y = pos.get("y")?.as_f64()?;
    Some((x, y))
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

/// Commit an armed place at a **world** position: mint a new squad under [`OpsCtx::active_side`],
/// file the slot as sole member / leader, select it, and run the shared post-change tail. Returns
/// `false` when nothing was armed.
///
/// `z = 0.0` / `rotation = 0.0` match the T-159.19 drag commit's DEM-not-ready case (React's
/// `terrainZ` on the flat map).
pub fn place_at(x: f64, y: f64) -> bool {
    let placed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        // T-180.5 — Objects stub: drop any armed place and do not mint.
        if ctx.objects_mode.get_untracked() {
            *ctx.pending.borrow_mut() = None;
            return false;
        }
        let Some(payload) = ctx.pending.borrow_mut().take() else {
            return false;
        };
        // Scoped: the mutators open write txns, which must be gone before `after_local_edit`'s
        // read txn.
        let id = {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            let layer_id = ensure_layer(ctx, core);
            let side = ctx.active_side.get_untracked();
            let id = mint_id(ctx, core);
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
            id
        };
        *ctx.selection.borrow_mut() = vec![id];
        true
    });
    if placed {
        // Rebinds the glyphs from the new SoA, bumps `doc_ver`, schedules the persist, and refreshes
        // the HUD + docks — the same tail the drag commit and undo/redo run.
        crate::mission_history::after_local_edit();
    }
    placed
}
