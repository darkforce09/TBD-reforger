//! Mission Creator editor (/missions/:id/edit) — the wgpu boundary collapse (T-159.15).
//!
//! **T-159.15.0 (foundation):** the Leptos app owns `RenderEngine` DIRECTLY as plain Rust — created
//! from a canvas `NodeRef` via `spawn_local`, no `map-engine-wasm` shim, one wasm module / one
//! linear memory. That slice mounted the canvas and rendered a single frame.
//!
//! **T-159.15.1 (this slice):** a damage-driven continuous render loop + wheel-zoom + resize, engine
//! owned directly (D5). Two things unblock the loop on the second `render()` submit (which panicked
//! `wgpu` "Buffer is already mapped" on the 15.0 foundation):
//!   1. `disable_frame_timing()` — drops the `GpuTimer` timestamp-readback lane. Headless is
//!      **WebGPU/Dawn** (not WebGL2 as first assumed), where that lane's `map_async` double-maps its
//!      16-byte buffer on the 2nd submit. The editor has no fps/GPU-time HUD, so the lane is pure
//!      overhead — dropping it removes the offending map. This is the actual fix.
//!   2. `engine.poll()` per frame after `render()` — drains readback `map_async` callbacks for the
//!      WebGL2-fallback path and the cull-counter lane that later world slices add. A no-op on
//!      real-browser WebGPU (the event loop resolves maps).
//! A `window.__selfChecks` bridge exposes the byte-exact GPU readback gate (calibration + texture)
//! the headless driver awaits — under `?force=webgl`, since `self_check`'s polled readback only
//! resolves on WebGL2 headless.
//!
//! The full Eden docked shell (Top Command Strip, Left Outliner, Right Asset Palette, Bottom
//! Toolbelt, doc host) lands across T-159.16–.22. Route is `chromeless` + `full_bleed` (AppLayout
//! hides the platform nav). Verified by GPU readback (not DOM diff) as the map lane grows.
#![allow(dead_code)]
use leptos::prelude::*;

/// Round CSS px → device-pixel backing size (≥1), matching the React oracle's `deviceSize`.
#[cfg(target_arch = "wasm32")]
fn device_size(css_w: f64, css_h: f64, dpr: f64) -> (u32, u32) {
    let r = |v: f64| ((v * dpr + 0.5).floor().max(1.0)) as u32;
    (r(css_w), r(css_h))
}

#[component]
pub fn MissionEditorPage() -> impl IntoView {
    let container_ref = NodeRef::<leptos::html::Div>::new();
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    // T-159.20 — Save Version + Export controls. The signals live on both targets (the view binds
    // them); the doc-touching command bodies are wasm-gated (native has no `MissionDocCore`).
    let save_semver = RwSignal::new("0.1.0".to_string());
    let save_status = RwSignal::new(String::new());

    // T-159.21 — Eden chrome state. All four are HUD *mirrors* of non-reactive state (the doc's undo
    // stack, its slot count, the leaked selection handle): `MissionDocCore` has no change
    // subscription and the selection is an `Rc<RefCell<…>>`, so `mission_history::refresh_*` pushes
    // onto these at every mutation site instead. `cursor` is fed by the pointer-move unproject.
    // Declared on both targets — the view binds them; only the wasm block ever sets them.
    let can_undo = RwSignal::new(false);
    let can_redo = RwSignal::new(false);
    let obj_count = RwSignal::new(0usize);
    let sel_count = RwSignal::new(0usize);
    let cursor = RwSignal::new(None::<(f64, f64)>);

    // T-159.22 — dock state. `outliner_nodes` / `selected_ids` are the same kind of pull-mirror as
    // OBJ/SEL above (pushed by `editor_ops::refresh_docks` from `mission_history::refresh_signals`,
    // i.e. at every mutation site). `active_layer` is the drop target (React's `activeLayerId`);
    // `catalog` holds the `/registry` fetch state and never leaves `Loading` on the native shell,
    // where `api_get` doesn't exist.
    let outliner_nodes = RwSignal::new(Vec::<crate::outliner::OutlinerNode>::new());
    // T-168 — the ORBAT dock tree mirror (faction/squad/slot), rebuilt alongside `outliner_nodes`.
    let orbat_nodes = RwSignal::new(Vec::<crate::outliner::OutlinerNode>::new());
    let selected_ids = RwSignal::new(Vec::<String>::new());
    let active_layer = RwSignal::new(None::<String>);
    let catalog = RwSignal::new(crate::asset_catalog::CatalogState::Loading);
    // T-159.26 — Attributes modal: the open slot id + a doc-change tick the modal re-reads on
    // (`doc_ver` is a plain Rc<Cell>, not reactive; refresh_docks bumps this signal instead).
    let attrs_open = RwSignal::new(None::<String>);
    let doc_tick = RwSignal::new(0u64);
    let settings_open = RwSignal::new(false);
    // T-167 — Faction Manager dialog toggle (launched from the Factions dock "Manage" button).
    let fm_open = RwSignal::new(false);
    // T-159.27 — the flat registry gear rows for the Attributes Arsenal tab (populated by the same
    // /registry fetch that builds the Factions palette). None until it lands.
    let registry_items = RwSignal::new(None::<Vec<crate::dto::RegistryItem>>);
    // T-167 — the compat edge feed for the Smart Arsenal (optic/magazine edge rows + validation).
    // Fetched once alongside /registry; starts Loading, degrades to Unavailable on error.
    let compat = RwSignal::new(crate::arsenal_rules::CompatFeed::default());
    // T-159.26 — server hydrate / conflict / dirty (data-safety). `conflict` holds an offered
    // server payload when local IDB content diverges; `dirty` is the unsaved-changes flag;
    // `current_semver` tracks the adopted server version.
    let dirty = RwSignal::new(false);
    let conflict = RwSignal::new(None::<ConflictInfo>);
    let current_semver = RwSignal::new(None::<String>);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = current_semver;

    // T-159.17 — mission id from the `:id` route param (`/missions/:id/edit`; `smoke` on the gate
    // route). One-shot untracked read at mount (id is static per route mount). Fallback `draft`
    // mirrors the React `missionId ?? 'draft'` persistence key. Hoisted out of the wasm block in
    // T-159.21: the chrome's title binds it, and the view compiles on the native target too.
    let mission_id = {
        use leptos_router::hooks::use_params_map;
        use_params_map()
            .get_untracked()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "draft".to_string())
    };

    // The engine is created + owned on the wasm target only (wgpu is wasm32-gated). Native builds
    // (cargo check) compile the shell without touching the GPU stack.
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::task::spawn_local;
        use std::cell::Cell;
        use std::cell::RefCell;
        use std::rc::Rc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;

        const TERRAIN_W: f64 = 12_800.0;
        const TERRAIN_H: f64 = 12_800.0;
        const INITIAL_TARGET: (f64, f64) = (6_400.0, 6_400.0);
        const INITIAL_ZOOM: f64 = -2.0;
        const WHEEL_ZOOM_PER_PX: f64 = 1.0 / 500.0;
        /// T-159.22 — matches the chrome host div in the view below (and thus every panel inside
        /// it), for the wheel guard's `closest()`. A `data-` attribute, not a class: the class list
        /// is a styling contract that a Tailwind edit could silently change under the guard.
        const CHROME_SEL: &str = "[data-eden-chrome]";

        // T-159.21 — the id is read once in the page body (the chrome's title binds it too).
        let mission_id = mission_id.clone();

        // T-159.20 — auth store for the Save Version POST. Read here in the reactive body (the
        // owner is live); `on_load` is a non-reactive closure, and `AuthStore` is `Copy` so it moves
        // into it cleanly. Provided by `AppLayout` above `<AppRoutes/>`, so present on this route.
        let auth = expect_context::<crate::auth::AuthStore>();

        // T-159.22 — the Factions palette catalog. Fetched once at mount (not in `on_load`): it is
        // engine-independent, so the dock fills even if wgpu never comes up. `kind == "character"`
        // rows only — `build_catalog_tree` is the T-068.3 `buildCatalogTree` port.
        spawn_local({
            use crate::asset_catalog::{build_catalog_tree, CatalogState};
            async move {
                match crate::client::api_get::<crate::dto::RegistryResponse>(auth, "/registry")
                    .await
                {
                    Ok(r) => {
                        registry_items.set(Some(r.data.clone()));
                        catalog.set(CatalogState::Ready(build_catalog_tree(&r.data)));
                    }
                    Err(_) => catalog.set(CatalogState::Failed),
                }
            }
        });

        // T-167 — compat edge feed for the Smart Arsenal (optic/magazine rows + validation). Own
        // fetch so a compat outage degrades the Arsenal to dumb dropdowns without touching /registry.
        spawn_local({
            use crate::arsenal_rules::{CompatFeed, CompatGraph, CompatStatus};
            async move {
                match crate::client::api_get::<crate::dto::RegistryCompatResponse>(
                    auth,
                    "/registry/compat",
                )
                .await
                {
                    Ok(r) => compat.set(CompatFeed {
                        status: CompatStatus::Ready,
                        graph: CompatGraph::from_edges(&r.data),
                    }),
                    Err(_) => compat.set(CompatFeed {
                        status: CompatStatus::Unavailable,
                        graph: CompatGraph::default(),
                    }),
                }
            }
        });

        canvas_ref.on_load(move |canvas: web_sys::HtmlCanvasElement| {
            let Some(container) = container_ref.get_untracked() else {
                return;
            };
            let container: web_sys::HtmlDivElement = container;
            let win = web_sys::window().expect("window");

            // Backend override for the headless readback gate: `?force=webgl` → WebGL2/SwiftShader,
            // where the byte-exact self_check readback resolves via `device.poll` (on webgpu/Dawn
            // headless the offscreen map never fires). Default (no query) = prefer WebGPU, matching
            // prod. Mirrors the React `WgpuCanvas` spike's `?force=webgl`.
            let force_webgl = win
                .location()
                .search()
                .map(|s| s.contains("force=webgl"))
                .unwrap_or(false);

            // Size the backing store BEFORE create (the engine reads canvas.width/height).
            let dpr0 = win.device_pixel_ratio();
            let rect0 = container.get_bounding_client_rect();
            let (dw, dh) = device_size(rect0.width(), rect0.height(), dpr0);
            canvas.set_width(dw);
            canvas.set_height(dh);

            let engine: Rc<RefCell<Option<map_engine_render::RenderEngine>>> =
                Rc::new(RefCell::new(None));
            // T-166 — shared map-asset host (camera-settle refresh after wheel/pan).
            let map_host = crate::world_assets::new_host_handle();
            let disposed = Arc::new(AtomicBool::new(false));

            // T-159.16 — MissionDoc host. Built + seeded + bridged synchronously (before the async
            // engine create), so the `window.__missionDoc` Class R gate does not depend on the wgpu
            // engine coming up. The doc leaks on route-leave like the engine (`!Send` `Rc`, and
            // `on_cleanup` is `Send`-bound) — no double-free (plain Rust `Drop`). The optional
            // doc→engine bind (D5) happens below once the engine is `Some`.
            let doc = crate::mission_doc::new_seeded_doc();
            let doc_ver = Rc::new(Cell::new(1u32));
            crate::mission_doc::register_mission_doc(doc.clone(), doc_ver.clone());

            // T-159.20 — editor commands (Save/Export) context + the `__editorCommands` smoke bridge
            // (peer of `__missionDoc`). `set_ctx` shares the same `Rc` the persistence swap targets,
            // so both the buttons and the bridge see an IDB-restored doc.
            crate::mission_commands::set_ctx(doc.clone(), auth, mission_id.clone(), current_semver);
            crate::mission_commands::register_editor_commands(doc.clone());

            // T-159.18 — LMB select foundation. Selection is app-side state (NOT the Y.Doc — it never
            // lived in the document, matching React's Zustand), held in the editor's leaked-handle
            // idiom so the `window.__editorSelection` smoke bridge (peer of __missionDoc) never reads
            // reactive-owner state a route change could dispose. `left` carries the in-flight LMB
            // gesture (T-159.19 `LeftGesture`: Pending → Move | Marquee — a frozen ortho camera copied
            // at the press drives every unproject) between pointerdown/move/up. Registered
            // synchronously (engine still `None` here — `probe()` reads it lazily; `pick_selfcheck()`
            // needs only the synchronously-seeded doc).
            let selection: crate::select_tool::SelectionHandle = Rc::new(RefCell::new(Vec::new()));
            let left: Rc<RefCell<Option<crate::select_tool::LeftGesture>>> =
                Rc::new(RefCell::new(None));
            crate::select_tool::register_editor_selection(
                selection.clone(),
                doc.clone(),
                engine.clone(),
                container.clone(),
            );

            // T-159.21 — undo/redo. The ctx carries every handle a post-change rebind needs (doc +
            // engine + selection + doc_ver + id) plus the HUD signal mirrors, so the toolbar buttons,
            // the keyboard shortcuts, and the `__editorHistory` bridge all drive ONE path. Registered
            // here (after the selection exists, engine still `None` — the rebind reads it lazily).
            // `refresh_hud` seeds the HUD from the freshly-seeded doc: OBJ = 8, SEL = 0, and
            // can_undo = false (the seed runs under INIT origin, so it is not an undo step).
            crate::mission_history::set_ctx(
                doc.clone(),
                engine.clone(),
                selection.clone(),
                doc_ver.clone(),
                mission_id.clone(),
                can_undo,
                can_redo,
                obj_count,
                sel_count,
                dirty,
            );
            // T-159.22 — dock commands (outliner select / active layer / palette place). Registered
            // BEFORE `refresh_hud()` below, because that call funnels into
            // `editor_ops::refresh_docks` — without the ctx the outliner would render empty until
            // the first edit.
            crate::editor_ops::set_ctx(
                doc.clone(),
                engine.clone(),
                selection.clone(),
                active_layer,
                outliner_nodes,
                orbat_nodes,
                selected_ids,
                attrs_open,
                doc_tick,
            );

            crate::mission_history::register_editor_history();
            crate::mission_history::register_key_handler();
            crate::mission_history::refresh_hud();

            // T-159.26 — editor keyboard actions (MissionCreatorPage onKeyDown): Delete/Backspace
            // (remove selection), Space (center on centroid), Ctrl/Cmd+C/V (copy/paste at cursor).
            // A SEPARATE window keydown from the undo/redo one (which owns Ctrl+Z/Y) — each guards
            // its own keys, both skip editable fields. `cursor` feeds the paste anchor (world coords).
            {
                let onkeydown = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                    move |ev: web_sys::KeyboardEvent| {
                        if crate::mission_history::in_editable_field() {
                            return;
                        }
                        let modk = ev.ctrl_key() || ev.meta_key();
                        let (cx, cy) = match cursor.get_untracked() {
                            Some((x, y)) => (Some(x), Some(y)),
                            None => (None, None),
                        };
                        // Each arm returns whether it acted; prevent the browser default once.
                        let handled = match ev.code().as_str() {
                            "KeyC" if modk && !ev.alt_key() && !ev.shift_key() => {
                                crate::editor_ops::copy_selection()
                            }
                            "KeyV" if modk && !ev.alt_key() && !ev.shift_key() => {
                                crate::editor_ops::paste_at_cursor(cx, cy)
                            }
                            "Space" if !modk => crate::editor_ops::center_on_selection(),
                            "Delete" | "Backspace" if !modk => {
                                crate::editor_ops::delete_selection()
                            }
                            _ => false,
                        };
                        if handled {
                            ev.prevent_default();
                        }
                    },
                );
                if let Some(win) = web_sys::window() {
                    let _ = win.add_event_listener_with_callback(
                        "keydown",
                        onkeydown.as_ref().unchecked_ref(),
                    );
                }
                onkeydown.forget();
            }

            // T-159.17 — persistence layer (additive; the SYNCHRONOUS seed above keeps the doc smoke
            // synchronous — `smoke_doc_editor` still sees 8 slots immediately on its own cold origin).
            // The `window.__missionPersist` bridge is installed synchronously (so the gate can wait on
            // it); the IDB load / initial-persist / warm-mark run async below and flip `ready` last.
            let persist_ready = Rc::new(Cell::new(false));
            let persist_loaded = Rc::new(Cell::new(false));
            crate::yrs_persist::register_mission_persist(
                doc.clone(),
                mission_id.clone(),
                persist_ready.clone(),
                persist_loaded.clone(),
            );
            spawn_local({
                let doc = doc.clone();
                let id = mission_id.clone();
                let ready = persist_ready.clone();
                let loaded = persist_loaded.clone();
                async move {
                    // 1. Restore from IDB if a blob exists — SWAP a fresh core (mirrors React's
                    //    empty-shell + apply; rests on the tested fresh-peer path + persist_roundtrip_ok,
                    //    NOT on reapply-idempotence). The swap is a synchronous block: no `borrow`/
                    //    `borrow_mut` is ever held across an `.await` (the engine task shares this `Rc`).
                    if let Some(blob) = crate::yrs_persist::load_state(&id).await {
                        if !blob.is_empty() {
                            let fresh = map_engine_core::doc::MissionDocCore::new();
                            fresh.set_origin_init(true);
                            let ok = fresh.apply_update(&blob).is_ok();
                            fresh.set_origin_init(false);
                            if ok {
                                *doc.borrow_mut() = Some(fresh);
                                loaded.set(true);
                                // T-159.21 — the restored core is a DIFFERENT document: its slot
                                // count may differ and its undo stack is empty (the replay ran under
                                // INIT). Re-seed the HUD mirrors off it, or the toolbelt would show
                                // the pre-restore counts. Not `after_local_edit`: nothing was edited,
                                // and re-arming the persist writer here would echo the restore back.
                                crate::mission_history::refresh_hud();
                            }
                        }
                    }
                    // 1.5 T-159.26 — server hydrate / conflict / dirty (UUID missions only; the
                    //     `smoke` gate route is non-UUID and skips this, so the editor smokes are
                    //     untouched). Replaces the seed with the saved version, or prompts on a
                    //     genuine local-vs-server conflict — the data-safety guarantee.
                    crate::mission_hydrate::hydrate_from_server(
                        doc.clone(),
                        id.clone(),
                        auth,
                        loaded.get(),
                        current_semver,
                        conflict,
                    )
                    .await;
                    // 2. Initial persist through the debounced writer (get_bytes read at write time;
                    //    cancel when the doc Option is cleared). No mutator hook exists yet, so this
                    //    post-seed/post-load encode is the writer's trigger this slice.
                    {
                        let doc_get = doc.clone();
                        let doc_cancel = doc.clone();
                        crate::yrs_persist::save_state_debounced(
                            &id,
                            Box::new(move || {
                                doc_get
                                    .borrow()
                                    .as_ref()
                                    .map(|c| c.encode_state())
                                    .unwrap_or_default()
                            }),
                            Box::new(move || doc_cancel.borrow().is_none()),
                            crate::yrs_persist::debounce_ms(),
                        );
                    }
                    // 3. Warm-session marker after the doc is ready.
                    let n = doc
                        .borrow()
                        .as_ref()
                        .map(|c| c.slot_count() as u32)
                        .unwrap_or(0);
                    crate::editor_session::mark_ready(&id, n, None);
                    // 4. Flush-on-hide listeners (visibilitychange/hidden + pagehide).
                    crate::yrs_persist::register_flush_on_hide(id.clone());
                    // 5. Ready LAST — the gate waits on this before asserting.
                    ready.set(true);
                }
            });

            spawn_local({
                let engine = engine.clone();
                let disposed = disposed.clone();
                let doc = doc.clone();
                let canvas = canvas.clone();
                let map_host = map_host.clone();
                let (cw, ch) = (rect0.width(), rect0.height());
                async move {
                    match map_engine_render::RenderEngine::create(canvas, force_webgl).await {
                        Ok(mut eng) => {
                            if disposed.load(Ordering::Relaxed) {
                                return;
                            }
                            let _ = eng.resize(cw, ch, dpr0);
                            eng.set_camera_bounds(0.0, 0.0, TERRAIN_W, TERRAIN_H);
                            eng.set_view(INITIAL_TARGET.0, INITIAL_TARGET.1, INITIAL_ZOOM);
                            eng.hide_calibration();
                            // Drop the GpuTimer readback lane: no fps HUD in the editor yet, and on
                            // headless WebGPU its map_async double-maps the 16-byte buffer on the 2nd
                            // submit ("Buffer is already mapped"). `poll()` (below, per frame) keeps
                            // the WebGL2-fallback + future cull-counter readback honest.
                            eng.disable_frame_timing();
                            eng.set_continuous_render(false); // damage-driven, matches the prod oracle
                            *engine.borrow_mut() = Some(eng);
                            register_self_checks(engine.clone());
                            register_editor_cam(engine.clone(), map_host.clone());
                            // T-159.16 — optional doc→engine bind (D5). `slots_bind_soa` early-returns
                            // while the slot atlas is unuploaded (the editor uploads none yet), so this
                            // is a safe cache write that proves the SoA wire compiles + runs; the tiny
                            // seeded slot set renders nothing until a later slice uploads the atlas.
                            let soa = doc.borrow().as_ref().map(|c| c.materialize());
                            if let (Some(soa), Some(e)) =
                                (soa.as_ref(), engine.borrow_mut().as_mut())
                            {
                                e.slots_bind_soa(soa.ids.clone(), &soa.xy);
                            }
                            start_raf(engine.clone(), disposed.clone());
                            // T-166 — full map-asset host (hillshade + sat + DEM vectors + world +
                            // forest). Terrain from doc meta (seed/hydrate; default everon).
                            {
                                let terrain = doc
                                    .borrow()
                                    .as_ref()
                                    .and_then(|c| {
                                        serde_json::from_str::<serde_json::Value>(
                                            &c.small_maps_json(),
                                        )
                                        .ok()?
                                        .get("meta")?
                                        .get("terrain")?
                                        .as_str()
                                        .map(str::to_string)
                                    })
                                    .unwrap_or_else(|| "everon".to_string());
                                let host = map_host.clone();
                                spawn_local(crate::world_assets::bootstrap(
                                    engine.clone(),
                                    terrain,
                                    host,
                                ));
                            }
                        }
                        Err(e) => leptos::logging::error!("RenderEngine::create: {e:?}"),
                    }
                }
            });

            // T-159.15.2 — pan gesture state: `Some((last_client_x, last_client_y))` while an
            // MMB/RMB drag-pan is in flight, else `None`. The pan feeds INCREMENTAL client-px deltas
            // to `engine.pan` (the camera does `target -= dΧ/scale` at the LIVE scale — Rust owns the
            // ortho math; this mirrors the `WgpuCanvas` oracle, NOT the Deck frozen-viewport path
            // that `useSelectTool` uses and the language gate forbids here). `(f64, f64)` is `Copy`,
            // so a `Cell` suffices (no `RefCell`); JS is single-threaded, so these pointer handlers
            // never reenter the rAF loop's `borrow_mut`.
            let pan_px: Rc<Cell<Option<(f64, f64)>>> = Rc::new(Cell::new(None));

            // Wheel → zoom_at (engine self-clamps zoom to [-6, 6]). Capture + non-passive so we can
            // preventDefault and beat any child handler. CSS origin = the container rect (same basis
            // as the pan/pick math).
            let onwheel = Closure::<dyn FnMut(web_sys::WheelEvent)>::new({
                let engine = engine.clone();
                let container = container.clone();
                let pan_px = pan_px.clone();
                let map_host = map_host.clone();
                move |ev: web_sys::WheelEvent| {
                    // T-159.22 — the wheel is capture-phase on the CONTAINER, so it fires before any
                    // dock could stop it (that is deliberate: it is what lets `prevent_default` beat
                    // a child, and the panels are descendants). The chrome therefore can't opt out
                    // by listener order — this handler has to look at the target and decline.
                    // Returning BEFORE `prevent_default` is the whole point: it leaves the event
                    // native, so a dock's `overflow-y-auto` scrolls instead of the map zooming
                    // (T-159.21 deferred item #1). A wheel over the free canvas is untouched.
                    if ev
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                        .is_some_and(|el| el.closest(CHROME_SEL).ok().flatten().is_some())
                    {
                        return;
                    }
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        ev.prevent_default();
                        let rect = container.get_bounding_client_rect();
                        e.zoom_at(
                            -ev.delta_y() * WHEEL_ZOOM_PER_PX,
                            ev.client_x() as f64 - rect.left(),
                            ev.client_y() as f64 - rect.top(),
                        );
                        // P5 mid-pan rebase (T-151.11.6): keep an in-flight pan alive across a
                        // mid-pan zoom. Under the single-pointer invariant a `pointermove` precedes
                        // any `wheel`, so `wheel.client == last_px`; this refresh is a provable no-op
                        // that also defensively re-syncs the start px. The next incremental
                        // `engine.pan` then rides the LIVE post-zoom scale, so panning continues
                        // seamlessly with no re-press. (The incremental model has no frozen zoom to
                        // go stale — the Deck bug T-151.11.6 fixed does not exist here.)
                        if pan_px.get().is_some() {
                            pan_px.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                        }
                        crate::world_assets::schedule_camera_settle(
                            map_host.clone(),
                            engine.clone(),
                        );
                    }
                }
            });
            let wheel_opts = web_sys::AddEventListenerOptions::new();
            wheel_opts.set_passive(false);
            wheel_opts.set_capture(true);
            let _ = container.add_event_listener_with_callback_and_add_event_listener_options(
                "wheel",
                onwheel.as_ref().unchecked_ref(),
                &wheel_opts,
            );

            // T-159.15.2 — MMB/RMB drag-pan (LMB deferred to the doc host / .16: no marquee / slot
            // move yet). Pointer capture keeps deltas flowing if the drag leaves the div; the
            // contextmenu is suppressed so an RMB-drag isn't interrupted by the browser menu (P3).
            // All five closures leak like the wheel/resize ones above (the engine leaks too;
            // `on_cleanup` only stops the loop — a `!Send` drop handle is later polish).
            let onpointerdown = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let container = container.clone();
                let engine = engine.clone();
                let left = left.clone();
                move |ev: web_sys::PointerEvent| {
                    // Middle (1) or right (2) button starts a pan.
                    if ev.button() == 1 || ev.button() == 2 {
                        ev.prevent_default();
                        let _ = container.set_pointer_capture(ev.pointer_id());
                        pan_px.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                    } else if ev.button() == 0 {
                        // T-159.18/.19 — LMB pending-left: freeze the ortho camera at press (X-05: the
                        // live engine unproject is deleted; a live unproject would feedback-loop
                        // mid-pan). No pointer capture yet — a sub-threshold release is a click; the
                        // first past-threshold `pointermove` (T-159.19) promotes to Move|Marquee and
                        // captures then. `engine.borrow()` is safe: JS is single-threaded, so this never
                        // reenters the rAF loop's `borrow_mut`.
                        if let Some(e) = engine.borrow().as_ref() {
                            let rect = container.get_bounding_client_rect();
                            let cam = crate::select_tool::frozen_camera(
                                rect.width(),
                                rect.height(),
                                e.target_x(),
                                e.target_y(),
                                e.zoom(),
                            );
                            *left.borrow_mut() = Some(crate::select_tool::LeftGesture::Pending(
                                crate::select_tool::PendingLeft {
                                    start_x: ev.client_x() as f64 - rect.left(),
                                    start_y: ev.client_y() as f64 - rect.top(),
                                    cam,
                                },
                            ));
                        }
                    }
                }
            });
            let onpointermove = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let engine = engine.clone();
                let left = left.clone();
                let doc = doc.clone();
                let selection = selection.clone();
                let container = container.clone();
                move |ev: web_sys::PointerEvent| {
                    use crate::select_tool::{self as st, LeftGesture as LG};
                    let rect = container.get_bounding_client_rect();
                    let (px, py) = (
                        ev.client_x() as f64 - rect.left(),
                        ev.client_y() as f64 - rect.top(),
                    );
                    // T-159.21 — CUR read-out. FIRST: both the pan branch and the no-gesture case
                    // below return early, and the cursor must keep tracking through both. Unprojects
                    // against the same `frozen_camera` the pick uses, so CUR always names the world
                    // point a click would hit. The borrow is scoped — the pan branch takes
                    // `borrow_mut` two lines down, and an overlapping borrow would panic.
                    // Un-throttled by design: React rAF-throttles because its cursor write
                    // re-rendered the page, whereas this feeds two text nodes through Leptos's
                    // fine-grained bindings. NaN (singular matrix) reads as off-map.
                    let world = {
                        let g = engine.borrow();
                        g.as_ref().map(|e| {
                            st::frozen_camera(
                                rect.width(),
                                rect.height(),
                                e.target_x(),
                                e.target_y(),
                                e.zoom(),
                            )
                            .unproject_xy(px, py)
                        })
                    };
                    cursor.set(
                        world
                            .filter(|c| c[0].is_finite() && c[1].is_finite())
                            .map(|c| (c[0], c[1])),
                    );
                    if let Some((lx, ly)) = pan_px.get() {
                        let (cx, cy) = (ev.client_x() as f64, ev.client_y() as f64);
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            e.pan(cx - lx, cy - ly);
                        }
                        pan_px.set(Some((cx, cy)));
                        return;
                    }
                    // T-159.19 — LMB drag gesture. Own the gesture across the update (take → compute →
                    // put back) so a Pending→Move/Marquee transition never aliases a `&mut`, and so no
                    // `left` borrow is held across the inner `left.borrow_mut()` put-back (the `if let`
                    // temporary-lifetime footgun). Frozen cam (M2/X-05 — no live unproject). Live preview
                    // via `engine.set_drag` (drag) / `engine.upload_marquee` (marquee rect).
                    let taken = left.borrow_mut().take();
                    let Some(g0) = taken else { return };
                    // Promote a Pending press once it clears the threshold; else keep the active drag.
                    let active = match g0 {
                        LG::Pending(p) => {
                            let moved =
                                ((px - p.start_x).powi(2) + (py - p.start_y).powi(2)).sqrt();
                            if moved < st::DRAG_THRESHOLD_PX {
                                *left.borrow_mut() = Some(LG::Pending(p));
                                return;
                            }
                            // Real drag now: capture so it survives leaving the canvas (React :200).
                            let _ = container.set_pointer_capture(ev.pointer_id());
                            let sw = p.cam.unproject_xy(p.start_x, p.start_y);
                            let hit = doc.borrow().as_ref().and_then(|c| {
                                st::pick(&p.cam, &c.materialize(), p.start_x, p.start_y)
                            });
                            match hit {
                                Some(id) => {
                                    // Drag an already-selected slot → move the whole selection; else
                                    // replace the selection with the dragged slot (React :204).
                                    let cur = selection.borrow().clone();
                                    let ids = st::compute_move_ids(&id, &cur);
                                    if !cur.iter().any(|s| *s == id) {
                                        *selection.borrow_mut() = ids.clone();
                                        if let Some(e) = engine.borrow_mut().as_mut() {
                                            e.set_selection(ids.clone());
                                        }
                                    }
                                    LG::Move {
                                        ids,
                                        start_wx: sw[0],
                                        start_wy: sw[1],
                                        cam: p.cam,
                                        dx: 0.0,
                                        dy: 0.0,
                                    }
                                }
                                None => LG::Marquee {
                                    start_x: p.start_x,
                                    start_y: p.start_y,
                                    start_wx: sw[0],
                                    start_wy: sw[1],
                                    cam: p.cam,
                                },
                            }
                        }
                        other => other,
                    };
                    // Drive the live preview for the (possibly just-promoted) state, coalescing the
                    // world delta / marquee rect into `active` for the pointerup commit.
                    let next = match active {
                        LG::Move {
                            ids,
                            start_wx,
                            start_wy,
                            cam,
                            ..
                        } => {
                            let (dx, dy) = st::drag_delta(&cam, start_wx, start_wy, px, py);
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.set_drag(ids.clone(), dx as f32, dy as f32);
                            }
                            LG::Move {
                                ids,
                                start_wx,
                                start_wy,
                                cam,
                                dx,
                                dy,
                            }
                        }
                        LG::Marquee {
                            start_x,
                            start_y,
                            start_wx,
                            start_wy,
                            cam,
                        } => {
                            let end = cam.unproject_xy(px, py);
                            if end[0].is_finite() && end[1].is_finite() {
                                let (min_x, max_x) = (start_wx.min(end[0]), start_wx.max(end[0]));
                                let (min_y, max_y) = (start_wy.min(end[1]), start_wy.max(end[1]));
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.upload_marquee(min_x, min_y, max_x, max_y, true);
                                }
                            }
                            LG::Marquee {
                                start_x,
                                start_y,
                                start_wx,
                                start_wy,
                                cam,
                            }
                        }
                        LG::Pending(p) => LG::Pending(p),
                    };
                    *left.borrow_mut() = Some(next);
                }
            });
            let onpointerup = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let container = container.clone();
                let engine = engine.clone();
                let left = left.clone();
                let doc = doc.clone();
                let selection = selection.clone();
                let map_host = map_host.clone();
                // T-159.21 — no `mission_id` capture: the persist tail now runs inside
                // `mission_history::after_local_edit`, which reads the id from its ctx.
                move |ev: web_sys::PointerEvent| {
                    // T-159.22 — palette drag-to-place. FIRST: a place is armed by a `pointerdown`
                    // on a palette leaf, which the chrome host stops from reaching the map — so
                    // `left`/`pan_px` are both None here and no gesture branch below would fire.
                    //
                    // The host stops `pointerdown` only, so a release over a dock ALSO bubbles here:
                    // the chrome insets decide. They are the same consts `select_tool`'s probe grid
                    // insets by, so "not under chrome" means one thing editor-wide.
                    if crate::editor_ops::has_pending() {
                        let rect = container.get_bounding_client_rect();
                        let (px, py) = (
                            ev.client_x() as f64 - rect.left(),
                            ev.client_y() as f64 - rect.top(),
                        );
                        let on_canvas = px >= crate::eden_chrome::DOCK_LEFT_PX
                            && px <= rect.width() - crate::eden_chrome::DOCK_RIGHT_PX
                            && py >= crate::eden_chrome::STRIP_TOP_PX
                            && py <= rect.height() - crate::eden_chrome::TOOLBELT_BAND_PX;
                        // Same frozen-camera unproject the pick + CUR use, so the slot lands exactly
                        // where CUR said it would.
                        let world = if on_canvas {
                            let g = engine.borrow();
                            g.as_ref().map(|e| {
                                crate::select_tool::frozen_camera(
                                    rect.width(),
                                    rect.height(),
                                    e.target_x(),
                                    e.target_y(),
                                    e.zoom(),
                                )
                                .unproject_xy(px, py)
                            })
                        } else {
                            None
                        };
                        match world.filter(|c| c[0].is_finite() && c[1].is_finite()) {
                            Some(c) => {
                                crate::editor_ops::place_at(c[0], c[1]);
                            }
                            None => crate::editor_ops::cancel_pending(),
                        }
                        return;
                    }
                    // Pan end (MMB/RMB).
                    if pan_px.get().is_some() {
                        pan_px.set(None);
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                        crate::world_assets::schedule_camera_settle(
                            map_host.clone(),
                            engine.clone(),
                        );
                    }
                    // LMB gesture end. `take()` into a `let` first so the RefMut drops before the
                    // per-branch re-borrows below (the `if let` temporary-lifetime footgun). If a pan
                    // just ended, `left` is None ⇒ this returns.
                    let taken = left.borrow_mut().take();
                    let Some(g) = taken else { return };
                    use crate::select_tool::{self as st, LeftGesture as LG};
                    let rect = container.get_bounding_client_rect();
                    let up_x = ev.client_x() as f64 - rect.left();
                    let up_y = ev.client_y() as f64 - rect.top();
                    match g {
                        // T-159.18/.53 — sub-threshold press = a click: pick against the FROZEN press
                        // camera (X-05) and toggle/replace/clear the selection.
                        LG::Pending(p) => {
                            let moved =
                                ((up_x - p.start_x).powi(2) + (up_y - p.start_y).powi(2)).sqrt();
                            if moved < st::DRAG_THRESHOLD_PX {
                                let additive = ev.ctrl_key() || ev.meta_key();
                                let hit = doc.borrow().as_ref().and_then(|c| {
                                    st::pick(&p.cam, &c.materialize(), p.start_x, p.start_y)
                                });
                                {
                                    let mut sel = selection.borrow_mut();
                                    st::apply_click(&mut sel, hit, additive);
                                }
                                let ids = selection.borrow().clone();
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.set_selection(ids); // tint lane
                                }
                                // T-159.21 — SEL readout only: a click changes the selection, not the
                                // document (no rebind / persist / undo step / tree rebuild).
                                crate::mission_history::refresh_selection();
                            }
                        }
                        // T-159.19 M4/M5 — drag-move commit. Release capture; if it actually moved,
                        // commit ONE `move_entities` txn (one undo step), re-bind the moved glyphs, keep
                        // the moved slots selected, and schedule the first edit-driven persist.
                        LG::Move { ids, dx, dy, .. } => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if dx != 0.0 || dy != 0.0 {
                                // `move_entities(&self)` opens a mut txn; the borrow is scoped so it
                                // drops before `after_local_edit`'s read txn. `zs = 0` is the
                                // DEM-not-ready byte-parity case (React `terrainZ` on flat map).
                                {
                                    let guard = doc.borrow();
                                    let Some(core) = guard.as_ref() else {
                                        return;
                                    };
                                    core.move_entities(ids.clone(), dx, dy, vec![0.0; ids.len()]);
                                }
                                // T-159.21 — the rebind/persist tail moved to `mission_history` so
                                // undo/redo run the exact same sequence. Equivalent to the inline
                                // T-159.19 code it replaces: it rebinds from the selection, which at
                                // a Move commit IS `ids` (see `after_doc_change`'s docs).
                                crate::mission_history::after_local_edit();
                            } else if let Some(e) = engine.borrow_mut().as_mut() {
                                e.set_drag(Vec::new(), 0.0, 0.0); // no move → just clear the overlay
                            }
                        }
                        // T-159.19 M3 — marquee commit. Release capture; a ≥1×1 px box replaces the
                        // selection with the enclosed slots (`pick_rect` over the frozen-cam world AABB);
                        // hide the rect.
                        LG::Marquee {
                            start_x,
                            start_y,
                            start_wx,
                            start_wy,
                            cam,
                        } => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if (up_x - start_x).abs() >= 1.0 && (up_y - start_y).abs() >= 1.0 {
                                let ids = doc
                                    .borrow()
                                    .as_ref()
                                    .map(|c| {
                                        st::marquee_ids(
                                            &cam,
                                            &c.materialize(),
                                            start_wx,
                                            start_wy,
                                            up_x,
                                            up_y,
                                        )
                                    })
                                    .unwrap_or_default();
                                *selection.borrow_mut() = ids.clone();
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.set_selection(ids);
                                }
                                // T-159.21 — SEL readout only (selection change, not a doc edit).
                                crate::mission_history::refresh_selection();
                            }
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.upload_marquee(0.0, 0.0, 0.0, 0.0, false); // hide
                            }
                        }
                    }
                }
            });
            let oncontextmenu =
                Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
                    ev.prevent_default()
                });
            // T-159.21 — pointer off the map ⇒ the CUR read-out shows the em-dash cells (React's
            // `onPointerLeave → null`). Fires when the pointer enters a chrome panel too, which is
            // correct: those px are not map coordinates.
            let onpointerleave = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                move |_ev: web_sys::PointerEvent| cursor.set(None)
            });
            // T-159.18/.19 — pointercancel ends BOTH a pan and any LMB gesture, but (unlike pointerup)
            // is NOT a commit: it drops the gesture without picking / moving / selecting, and clears any
            // live preview (drag overlay / marquee rect) + releases capture.
            let onpointercancel = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let container = container.clone();
                let left = left.clone();
                let engine = engine.clone();
                move |ev: web_sys::PointerEvent| {
                    // T-159.22 — a cancelled pointer drops an armed place, like every other
                    // in-flight gesture below (pointercancel is never a commit).
                    crate::editor_ops::cancel_pending();
                    if pan_px.get().is_some() {
                        pan_px.set(None);
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                    }
                    use crate::select_tool::LeftGesture as LG;
                    let taken = left.borrow_mut().take();
                    match taken {
                        Some(LG::Move { .. }) => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.set_drag(Vec::new(), 0.0, 0.0);
                            }
                        }
                        Some(LG::Marquee { .. }) => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.upload_marquee(0.0, 0.0, 0.0, 0.0, false);
                            }
                        }
                        _ => {}
                    }
                }
            });
            let _ = container.add_event_listener_with_callback(
                "pointerdown",
                onpointerdown.as_ref().unchecked_ref(),
            );
            let _ = container.add_event_listener_with_callback(
                "pointermove",
                onpointermove.as_ref().unchecked_ref(),
            );
            let _ = container.add_event_listener_with_callback(
                "pointerup",
                onpointerup.as_ref().unchecked_ref(),
            );
            // pointercancel ends the pan + a pending LMB without a click (T-159.18).
            let _ = container.add_event_listener_with_callback(
                "pointercancel",
                onpointercancel.as_ref().unchecked_ref(),
            );
            let _ = container.add_event_listener_with_callback(
                "contextmenu",
                oncontextmenu.as_ref().unchecked_ref(),
            );
            let _ = container.add_event_listener_with_callback(
                "pointerleave",
                onpointerleave.as_ref().unchecked_ref(),
            );
            // T-159.26 A1 — native dblclick on a slot opens Attributes (the T-054 SEL-MAP-004
            // contract). Picks with a FRESH frozen camera at the event px (the same pick the
            // click path uses); a miss is a no-op; multi-select suppression lives in
            // `open_attributes`. The chrome subtree stops pointerdown, so dblclicks over docks
            // never reach here.
            let ondblclick = Closure::<dyn FnMut(web_sys::MouseEvent)>::new({
                let container = container.clone();
                let engine = engine.clone();
                let doc = doc.clone();
                move |ev: web_sys::MouseEvent| {
                    if ev.button() != 0 {
                        return;
                    }
                    let rect = container.get_bounding_client_rect();
                    let (px, py) = (
                        ev.client_x() as f64 - rect.left(),
                        ev.client_y() as f64 - rect.top(),
                    );
                    let cam = {
                        let g = engine.borrow();
                        let Some(e) = g.as_ref() else { return };
                        crate::select_tool::frozen_camera(
                            rect.width(),
                            rect.height(),
                            e.target_x(),
                            e.target_y(),
                            e.zoom(),
                        )
                    };
                    let hit = doc
                        .borrow()
                        .as_ref()
                        .and_then(|c| crate::select_tool::pick(&cam, &c.materialize(), px, py));
                    if let Some(id) = hit {
                        crate::editor_ops::open_attributes(id);
                    }
                }
            });
            let _ = container
                .add_event_listener_with_callback("dblclick", ondblclick.as_ref().unchecked_ref());

            let onresize = Closure::<dyn FnMut()>::new({
                let engine = engine.clone();
                let canvas = canvas.clone();
                let container = container.clone();
                move || {
                    let dpr = web_sys::window()
                        .map(|w| w.device_pixel_ratio())
                        .unwrap_or(1.0);
                    let rect = container.get_bounding_client_rect();
                    let (dw, dh) = device_size(rect.width(), rect.height(), dpr);
                    canvas.set_width(dw);
                    canvas.set_height(dh);
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        let _ = e.resize(rect.width(), rect.height(), dpr);
                    }
                }
            });
            let _ =
                win.add_event_listener_with_callback("resize", onresize.as_ref().unchecked_ref());

            // The engine + these listeners intentionally leak on route-leave: `on_cleanup` is
            // `Send`-bound and can't hold the `!Send` engine, so we only move `disposed` (Send) into
            // it. Stopping the loop is what prevents a runaway render; a proper `!Send` drop handle
            // is a later polish.
            onwheel.forget();
            onresize.forget();
            onpointerdown.forget();
            onpointermove.forget();
            onpointerup.forget();
            onpointercancel.forget();
            oncontextmenu.forget();
            onpointerleave.forget();
            ondblclick.forget();
            on_cleanup(move || disposed.store(true, Ordering::Relaxed));
        });
    }

    view! {
        <div
            node_ref=container_ref
            class="relative h-screen w-screen overflow-hidden bg-background"
        >
            <canvas node_ref=canvas_ref class="absolute inset-0 z-0 block h-full w-full"></canvas>
            // T-159.21 — Eden chrome host (React MissionCreatorPage:272). The container class is
            // deliberately UNCHANGED and the canvas stays full-bleed underneath: every `select_tool`
            // probe derives its camera from this container's bounding rect, so shrinking it would
            // silently invalidate the pan/select/marquee/move gates.
            //
            // `pointer-events-none` hands the whole rect to the map; each panel re-enables
            // `pointer-events-auto` for itself. The panels are DESCENDANTS of the gesture container,
            // so without `stop_propagation` a chrome click would bubble into `onpointerdown` and open
            // an LMB map gesture — clicking Undo would also deselect. Its corollary is the
            // chrome-free inset in `select_tool::farthest_empty_px`.
            // T-159.22 — `data-eden-chrome` marks the whole chrome subtree for the wheel guard's
            // `closest()` (see CHROME_SEL): a wheel whose target is inside it must scroll the dock,
            // not zoom the map.
            <div
                data-eden-chrome
                class="pointer-events-none absolute inset-0 z-10"
                on:pointerdown=|ev| ev.stop_propagation()
            >
                <div class="absolute inset-x-0 top-0 h-12">
                    <crate::eden_chrome::TopCommandStrip
                        title=mission_id.clone()
                        can_undo
                        can_redo
                        save_semver
                        save_status
                        dirty
                        settings_open
                    />
                </div>
                <div class="absolute bottom-0 left-0 top-12 w-64">
                    <crate::eden_chrome::DockLeft
                        nodes=outliner_nodes
                        orbat=orbat_nodes
                        selected=selected_ids
                        active_layer
                    />
                </div>
                <div class="absolute bottom-0 right-0 top-12 w-80">
                    <crate::eden_chrome::DockRight catalog fm_open />
                </div>
                <div class="absolute bottom-5 left-1/2 -translate-x-1/2">
                    <crate::eden_chrome::BottomToolbelt cursor sel_count obj_count />
                </div>
                // T-159.26 — Attributes modal (fixed overlay; no DOM while closed). Inside the
                // chrome subtree so its pointerdowns never open a map gesture.
                <div class="pointer-events-auto">
                    <crate::attributes::AttributesModal attrs_open doc_tick registry_items compat />
                </div>
                <div class="pointer-events-auto">
                    <crate::eden_chrome::MissionSettingsDialog open=settings_open doc_tick />
                    <crate::faction_manager::FactionManagerDialog open=fm_open registry=registry_items />
                </div>
                // T-159.26 — local-vs-server conflict prompt (React's ConflictDialog). Renders only
                // when `conflict` is Some (a divergent local doc on cold boot). Data-safety: the
                // user chooses which version wins before any Save.
                <div class="pointer-events-auto">
                    <ConflictDialog conflict conflict_id=mission_id.clone() />
                </div>
            </div>
        </div>
    }
}

/// The rAF render loop. Each frame renders then polls the device (see `RenderEngine::poll`) so
/// readback `map_async` callbacks drain on the WebGL2-fallback + cull-counter path. (The timer
/// double-map that panicked the 15.0 loop is handled upstream by `disable_frame_timing`.) Stops
/// (and drops itself) once `disposed` is set.
#[cfg(target_arch = "wasm32")]
fn start_raf(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
    disposed: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::Ordering;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if disposed.load(Ordering::Relaxed) {
            f.borrow_mut().take(); // drop the loop closure — no further frames
            return;
        }
        if let Some(e) = engine.borrow_mut().as_mut() {
            let _ = e.render();
            e.poll(); // ★ T-159.15.1: drain readback map_async so the next submit can't double-map
        }
        let cb_ref = f.borrow();
        if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut()>));
    let cb_ref = g.borrow();
    if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    }
}

/// Expose the byte-exact GPU readback self-checks on `window.__selfChecks` — the map-lane gate the
/// headless driver awaits (see [[wgpu-headless-gpu-verify]]). Both checks are scene-independent
/// (`self_check` renders its own fixed calibration probe scene; `texture_self_check` a synthetic
/// 2×2 texture) and `&self`: they clone their GPU handles up front, so the shared `borrow()` here is
/// released before the async readback runs — no contention with the rAF loop's `borrow_mut` (JS is
/// single-threaded). Each resolves to a JSON string with a `pass` field.
#[cfg(target_arch = "wasm32")]
fn register_self_checks(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
) {
    use wasm_bindgen::prelude::*;

    let obj = js_sys::Object::new();

    let calibration = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move || {
            engine
                .borrow()
                .as_ref()
                .map(|e| e.self_check())
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut() -> js_sys::Promise>)
    };
    let texture = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move || {
            engine
                .borrow()
                .as_ref()
                .map(|e| e.texture_self_check())
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut() -> js_sys::Promise>)
    };

    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("calibration"),
        calibration.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("texture"), texture.as_ref());
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__selfChecks"), &obj);
    }
    // The harness reads these across the page lifetime; leak them (the engine leaks too).
    calibration.forget();
    texture.forget();
}

/// Expose the camera view-state on `window.__editorCam()` for the headless pan smoke (T-159.15.2 /
/// spec P6): a JSON string `{"tx","ty","z","backend"}` read from the `&self` getters `target_x()` /
/// `target_y()` / `zoom()` / `backend()`. (`#[wasm_bindgen(getter)]` fns are plain method calls from
/// Rust.) All are `&self` behind a shared `borrow()`, released before return — no contention with the
/// rAF loop's `borrow_mut` (JS is single-threaded). Registered once the engine is `Some`; the closure
/// leaks like the self-checks. The smoke drives pan via getter deltas (never `unproject_xy`, X-05).
///
/// T-166 — also installs `window.__editorCamSet(tx, ty, z)` so `smoke_fullmap` can Class-R probe
/// tree glyphs at zoom ≥ 0 without relying on CDP `mouseWheel` → DOM `wheel` delivery.
#[cfg(target_arch = "wasm32")]
fn register_editor_cam(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
    map_host: crate::world_assets::HostHandle,
) {
    use wasm_bindgen::prelude::*;

    let cam = Closure::wrap(Box::new({
        let engine = engine.clone();
        move || -> JsValue {
            engine
                .borrow()
                .as_ref()
                .map(|e| {
                    JsValue::from_str(&format!(
                        r#"{{"tx":{},"ty":{},"z":{},"backend":"{}"}}"#,
                        e.target_x(),
                        e.target_y(),
                        e.zoom(),
                        e.backend()
                    ))
                })
                .unwrap_or_else(|| JsValue::from_str("null"))
        }
    }) as Box<dyn FnMut() -> JsValue>);

    let cam_set = Closure::wrap(Box::new({
        let engine = engine.clone();
        let map_host = map_host.clone();
        move |tx: f64, ty: f64, z: f64| {
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.set_view(tx, ty, z);
            }
            // Immediate flush so smoke_fullmap A_trees_on does not race the 120 ms debounce.
            crate::world_assets::flush_viewport(map_host.clone(), engine.clone());
        }
    }) as Box<dyn FnMut(f64, f64, f64)>);

    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCam"), cam.as_ref());
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCamSet"), cam_set.as_ref());
    }
    cam.forget();
    cam_set.forget();
}

/// T-159.26 — the local-vs-server conflict payload the [`ConflictDialog`] offers to load. Un-gated
/// (two Strings, no wasm types) so the shared editor view can hold the signal; `mission_hydrate`
/// (wasm-only) produces and consumes it.
#[derive(Clone)]
pub struct ConflictInfo {
    pub payload_json: String,
    pub semver: Option<String>,
}

/// The conflict prompt (React `ConflictDialog`): renders only when `conflict` is `Some`. "Load
/// server version" hydrates the offered payload (data replaced); "Keep local copy" keeps the local
/// doc and marks it divergent. Renders no DOM while `None` — V-capture-safe.
#[component]
fn ConflictDialog(conflict: RwSignal<Option<ConflictInfo>>, conflict_id: String) -> impl IntoView {
    let id = StoredValue::new(conflict_id);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = id;
    move || {
        conflict.get().map(|c| {
            let _ = &c;
            #[cfg(target_arch = "wasm32")]
            let (id_server, id_local) = (id.get_value(), id.get_value());
            let semver_label = c
                .semver
                .clone()
                .map(|s| format!("Saved version v{s}"))
                .unwrap_or_else(|| "A saved version".to_string());
            view! {
                <div class="fixed inset-0 z-[60] bg-black/50 backdrop-blur-sm"></div>
                <div class="glass fixed top-1/2 left-1/2 z-[60] flex w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none">
                    <div class="border-b border-outline-variant/30 px-6 py-4">
                        <h2 class="text-headline-sm text-on-surface">"Unsaved local changes"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            {semver_label}
                            " on the server differs from your local copy. Which version should win?"
                        </p>
                    </div>
                    <div class="flex justify-end gap-2 px-6 py-4">
                        <button
                            type="button"
                            aria-label="Keep local copy"
                            class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                #[cfg(target_arch = "wasm32")]
                                crate::mission_hydrate::resolve_conflict_local(
                                    id_local.clone(),
                                    conflict,
                                );
                            }
                        >
                            "Keep local copy"
                        </button>
                        <button
                            type="button"
                            aria-label="Load server version"
                            class="rounded-lg bg-primary px-4 py-2 text-label-md font-medium text-on-primary"
                            on:click=move |_| {
                                #[cfg(target_arch = "wasm32")]
                                crate::mission_hydrate::resolve_conflict_server(
                                    id_server.clone(),
                                    conflict,
                                );
                            }
                        >
                            "Load server version"
                        </button>
                    </div>
                </div>
            }
        })
    }
}
