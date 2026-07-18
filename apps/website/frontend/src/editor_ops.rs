//! T-159.22 — the dock commands: outliner select / active layer, and palette drag-to-place.
//!
//! Peer of `mission_history` / `mission_commands`, and the same shape for the same reason: the doc /
//! engine / selection handles are `!Send` wasm-only `Rc`s that can't cross the
//! `#[cfg(target_arch = "wasm32")]` boundary into the native view shell, so the dock buttons reach
//! them through a `thread_local` [`OpsCtx`] set from `mission_editor::on_load` — exactly how the
//! Undo button reaches the undo stack.
//!
//! **Placement mints a default squad + layer** (React's `ensureDefaultSquad` +
//! `ensureDefaultLayer`, T-168). [`ensure_default_squad`] reuses the first existing squad, else
//! lazily mints the default faction + squad; [`ensure_layer`] resolves / lazily mints the default
//! layer. Both are minted under the LOCAL origin so they are **undoable** — a boot-time
//! faction/squad/layer would break the save/export gate, which asserts the graph is empty at boot
//! (`smoke_save_export_editor` uses the seed only, so nothing is minted there). The placed slot
//! joins the squad (`squad.slotIds`) and the layer (`layer.entityIds`); the ORBAT tree derives
//! from the squads (`build_orbat`). Seed slots still carry a dangling `squadId` with no squad in
//! the map (`store.rs:369`) — they list under Unfiled / no ORBAT squad until placed-through.
//!
//! Consequence, recorded in the verify log: the **first** place is up to **three** undo steps
//! (layer, faction+squad, slot) since `add_editor_layer` / `add_faction`+`add_squad` / `add_slot`
//! are separate core transactions where React's `ydoc.addSlot` wraps them in one `transact`; every
//! later place is one step.
//!
//! **Borrow discipline** (the `mission_history` rule): each `pub fn` opens exactly one `OPS_CTX`
//! borrow; doc `borrow_mut`s are scoped so they drop before `mission_history::after_local_edit`
//! opens its read borrows.
#![cfg(target_arch = "wasm32")]

use std::cell::{Cell, RefCell};

use leptos::prelude::{GetUntracked, RwSignal, Set};
use map_engine_core::doc::MissionDocCore;
use map_engine_core::doc::NONE_IDX;

use crate::asset_catalog::PlacePayload;
use crate::mission_doc::DocHandle;
use crate::outliner::{build_outliner, LayerRow, OutlinerNode, SlotRow};
use crate::select_tool::{EngineHandle, SelectionHandle};

/// The lazily-minted default layer (React's `ensureDefaultLayer`).
const DEFAULT_LAYER_ID: &str = "layer-1";
const DEFAULT_LAYER_NAME: &str = "Layer 1";
/// T-168 — the lazily-minted default faction + squad (React's `ensureDefaultSquad`), so a placed
/// slot always joins an ORBAT squad instead of the pre-T-168 `squadId=""`.
const DEFAULT_FACTION_ID: &str = "faction-1";
const DEFAULT_FACTION_NAME: &str = "Faction 1";
const DEFAULT_SQUAD_ID: &str = "squad-1";
const DEFAULT_SQUAD_NAME: &str = "Squad 1";

struct OpsCtx {
    doc: DocHandle,
    engine: EngineHandle,
    selection: SelectionHandle,
    /// The drop target folder (React's `activeLayerId`). `None` ⇒ the place path resolves one.
    active_layer: RwSignal<Option<String>>,
    /// Dock mirrors — `MissionDocCore` has no change subscription, so these are pushed from
    /// [`refresh_docks`] at every mutation site, like the OBJ/SEL readouts.
    outliner_nodes: RwSignal<Vec<OutlinerNode>>,
    /// T-168 — the ORBAT dock tree mirror (faction/squad/slot), rebuilt alongside `outliner_nodes`.
    orbat_nodes: RwSignal<Vec<OutlinerNode>>,
    selected_ids: RwSignal<Vec<String>>,
    /// T-159.26 — the Attributes modal's open slot id (`None` = closed). The dbl-click pick and the
    /// outliner activate set it; the modal component reads it reactively.
    attrs_open: RwSignal<Option<String>>,
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
    outliner_nodes: RwSignal<Vec<OutlinerNode>>,
    orbat_nodes: RwSignal<Vec<OutlinerNode>>,
    selected_ids: RwSignal<Vec<String>>,
    attrs_open: RwSignal<Option<String>>,
    doc_tick: RwSignal<u64>,
) {
    OPS_CTX.with(|c| {
        *c.borrow_mut() = Some(OpsCtx {
            doc,
            engine,
            selection,
            active_layer,
            outliner_nodes,
            orbat_nodes,
            selected_ids,
            attrs_open,
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
pub fn begin_place(payload: PlacePayload) {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
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

/// T-169 smoke hook — bulk-add `n` slots under the default layer + squad, then refresh the docks,
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
        let squad_id = ensure_default_squad(core);
        for _ in 0..n {
            let id = mint_id(ctx, core);
            core.add_slot(
                &id, &squad_id, &layer_id, 0, "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
            );
        }
    });
    crate::mission_history::after_local_edit();
}

/// T-168 — resolve the squad a placed slot joins (React's `ensureDefaultSquad`): reuse the first
/// existing squad, else lazily mint the default faction + squad. Returns the squad id.
fn ensure_default_squad(core: &MissionDocCore) -> String {
    if let Some(first) = squad_rows(core).first() {
        return first.id.clone();
    }
    // No squad yet — mint the default faction (if absent) then the default squad under it.
    if !faction_rows(core)
        .iter()
        .any(|f| f.id == DEFAULT_FACTION_ID)
    {
        core.add_faction(DEFAULT_FACTION_ID, DEFAULT_FACTION_ID, DEFAULT_FACTION_NAME);
    }
    core.add_squad(
        DEFAULT_SQUAD_ID,
        DEFAULT_FACTION_ID,
        DEFAULT_SQUAD_NAME,
        None,
    );
    DEFAULT_SQUAD_ID.to_string()
}

/// Commit an armed place at a **world** position: file a slot under the resolved layer, select it,
/// and run the shared post-change tail. Returns `false` when nothing was armed.
///
/// `z = 0.0` / `rotation = 0.0` match the T-159.19 drag commit's DEM-not-ready case (React's
/// `terrainZ` on the flat map). `index: 0` is the ordinal within the slot's squad — inert here,
/// since no squad is minted (see the module docs).
pub fn place_at(x: f64, y: f64) -> bool {
    let placed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
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
            let squad_id = ensure_default_squad(core); // T-168 place-mint
            let id = mint_id(ctx, core);
            core.add_slot(
                &id,
                &squad_id,
                &layer_id,
                0,
                &payload.role,
                None,
                Some(payload.asset_id),
                x,
                y,
                0.0,
                0.0,
            );
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
