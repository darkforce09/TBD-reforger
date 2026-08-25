//! T-934.11 — the Mission Creator page's floating OVERLAY / DIALOG components, split out of
//! `mission_editor.rs` (Phase B): the transformation widget + cursor mode hint + snap readout
//! (T-648/T-795), the empty-ground asset picker (T-647), the comment editor (T-651), the
//! Connections panel (T-672) and the local-vs-server conflict dialog (T-159.26) — plus
//! [`AssetPickerState`], [`ConflictInfo`] and the widget-pivot registry the gizmo reads.
//!
//! Bodies are byte-identical to their `mission_editor.rs` originals, and `mission_editor`
//! re-exports every name here, so the page's bare mounts (`<TransformWidgetOverlay …/>`), the
//! `crate::editor::mission_editor::{AssetPickerState, ConflictInfo}` paths in
//! `state/operations/context.rs` / `state/hydrate.rs`, and the page-region mount needles all keep
//! their exact spelling. The evacuated definition pins (`t647_placement_interactions`,
//! `t726_window_esc_stack`) scrub THIS file — it deliberately carries no `#[cfg(test)]`, so
//! `class_r_scrub::live_code` keeps all of it.
//!
//! Components stay UNGATED (native-compiled) exactly as before: each one renders the same DOM
//! shell on both targets and gates its `editor_ops` / `mission_hydrate` / `web_sys` reads behind
//! `#[cfg(target_arch = "wasm32")]`, which is what lets the browser-independent tests compile and
//! scrub them. The T-797 toolbar-dispatch registry stays in `mission_editor.rs` — `eden_top_strip`
//! reaches it through `crate::editor::mission_editor::…` and it bridges the page to the strip,
//! not to these overlays.
// The same gate `mission_editor.rs` carries, for the same reason: several items here are only
// reached from `#[cfg(target_arch = "wasm32")]` closures or `#[cfg(test)]` pins.
#![allow(dead_code)]

use leptos::prelude::*;

use crate::editor::mission_editor::transform;
#[cfg(target_arch = "wasm32")]
use crate::editor::state::hydrate as mission_hydrate;
#[cfg(target_arch = "wasm32")]
use crate::editor::state::operations as editor_ops;

/// T-647 PLACE-003 — where a double-click on empty ground opened the asset picker: the WORLD point
/// the eventual place will land at, plus the SCREEN pixel to anchor the floating panel at.
///
/// Defined HERE (not in the wasm-only `editor_ops`) so the native test build — which compiles this
/// page but not `editor_ops` — can name it. `editor_ops` re-uses it via `crate::editor::mission_editor::…`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssetPickerState {
    /// World metres — the anchor the operator aimed at (parity with the ghost/CUR unproject). The
    /// actual drop still comes from the next canvas click, so this is not a bypass of the click.
    /// T-723: the picker row arms on `click` (after pointerup), so the next *canvas* click lands
    /// here — these coords remain the dblclick anchor for the panel position, not a bypass place.
    pub wx: f64,
    pub wy: f64,
    /// Client pixel of the dblclick, so the panel floats at the cursor (like Eden's create menu).
    pub screen_x: f64,
    pub screen_y: f64,
}

thread_local! {
    /// T-648 — the registered SELECTION-CENTROID getter the transform widget projects onto. Set from
    /// `MissionEditorPage`'s wasm block (which owns the `!Send` doc + selection `Rc`s); read by the
    /// native-compiled [`TransformWidgetOverlay`] via [`read_widget_pivot`]. Peer of
    /// `ruler_tool::RULER_CHAIN` — a thread_local so the overlay never touches disposed reactive
    /// state and native builds simply see `None`.
    static WIDGET_PIVOT: std::cell::RefCell<Option<std::rc::Rc<dyn Fn() -> Option<(f64, f64)>>>> =
        const { std::cell::RefCell::new(None) };
}

/// T-648 — register the selection-centroid getter (called once at mount). `#[cfg(target_arch =
/// "wasm32")]` because only the wasm host has the doc/selection `Rc`s to close over; the getter it
/// stores returns `Option<(world_x, world_y)>` — the current selection centroid, or `None` when the
/// selection is empty or the doc is not ready.
#[cfg(target_arch = "wasm32")]
pub(crate) fn register_widget_pivot(f: std::rc::Rc<dyn Fn() -> Option<(f64, f64)>>) {
    WIDGET_PIVOT.with(|c| *c.borrow_mut() = Some(f));
}

/// T-648 — the current selection centroid in world metres, or `None` (empty selection / no doc /
/// native build / pre-mount). The overlay calls this each repaint; it is a cheap doc read behind the
/// registered closure.
#[must_use]
pub(crate) fn read_widget_pivot() -> Option<(f64, f64)> {
    WIDGET_PIVOT.with(|c| c.borrow().as_ref().and_then(|f| f()))
}

/// T-648 WIDGET-CYCLE-001 / WIDGET-TRANS-001 — the TRANSFORMATION WIDGET: a lightweight
/// `pointer-events-none` SVG gizmo drawn on the selection centroid, in the ruler/LoS overlay idiom
/// (full-bleed SVG, reads the live camera off `world_assets::camera_snapshot`, projects world→screen
/// with the same `frozen_camera` the pick uses, re-runs off the `cursor`/`debug_hud`/`tick`
/// heartbeats — no new rAF loop). It is a VIEW + affordance: the actual gestures (Shift+drag rotate,
/// axis-constrained move) are captured by the map's own pointer handlers and commit through the
/// existing move / `attrs_update_position` paths — the SVG never eats a pointer.
///
/// Two variants (`WidgetVariant`, cycled by the `1`/`2` keys):
///   * **Translate** — a pair of axis ARROWS (X east, Y north) centred on the selection. A drag on
///     an arrow is the axis-constrained move; the arrows are the discoverable handle for it.
///   * **Rotate** — a RING around the selection. A drag rotates; Shift+drag on the ring snaps to the
///     rotation ladder. (Shift+drag anywhere on a selected entity already rotates — the ring makes
///     the gesture visible.)
///
/// Only drawn when something is selected (a widget with no target is nothing to show). Ungated so it
/// is native-compiled and its projection is source-pinned; the geometry itself renders only under
/// wasm (it needs the live camera + window).
#[component]
pub(crate) fn TransformWidgetOverlay(
    /// Pan heartbeat (the editor's pointer-move cursor write) — re-projects the gizmo on pan.
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// ~1 Hz zoom heartbeat (the rAF debug sampler) — re-projects after a still-pointer wheel-zoom.
    debug_hud: Option<RwSignal<String>>,
    /// Bumped when the selection changes without a pointermove, so the gizmo re-projects onto the new
    /// centroid even with a still pointer (the `ruler_tick` idiom).
    tick: RwSignal<u64>,
    /// The live widget variant (`1` translate / `2` rotate) — decides arrows vs ring.
    variant: RwSignal<transform::WidgetVariant>,
) -> impl IntoView {
    // The projected gizmo centre (screen px) + the variant, or None when there is nothing to draw.
    let projected = move || -> Option<(f64, f64, transform::WidgetVariant)> {
        // Subscribe to all heartbeats so the closure re-runs on pan (cursor), zoom (hud), selection
        // change (tick) and variant change.
        let _ = cursor.get();
        if let Some(h) = debug_hud {
            let _ = h.get();
        }
        let _ = tick.get();
        let var = variant.get();
        let (wx, wy) = read_widget_pivot()?;
        #[cfg(target_arch = "wasm32")]
        {
            let (tx, ty, zoom) = crate::editor::world_assets::camera_snapshot()?;
            let win = web_sys::window()?;
            let vw = win
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let vh = win
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if vw <= 0.0 || vh <= 0.0 {
                return None;
            }
            let cam = crate::editor::tools::select_tool::frozen_camera(vw, vh, tx, ty, zoom);
            let p = cam.project([wx, wy, 0.0]);
            if !p[0].is_finite() || !p[1].is_finite() {
                return None;
            }
            Some((p[0], p[1], var))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (wx, wy, var);
            None
        }
    };
    view! {
        // Full-bleed, non-interactive SVG in the same overlay band as the ruler/grid refs (z-10),
        // over the map but under the chrome docks. `pointer-events-none`: the gizmo is a view, the
        // gesture is the map's own pointer handlers.
        <svg
            data-transform-widget
            class="pointer-events-none absolute inset-0 z-10"
            width="100%"
            height="100%"
        >
            {move || projected().map(|(cx, cy, var)| {
                // Fixed pixel radius/arm length — the gizmo is a screen affordance, not a world
                // object, so it stays a constant size like a cursor (Eden's widget does too). The ring
                // radius is the SHARED `WIDGET_RADIUS_PX` the gesture ring hit-test reads, so the drawn
                // ring and the draggable ring are the same circle (T-795 — no more decoration).
                const R: f64 = transform::WIDGET_RADIUS_PX;
                const HEAD: f64 = 7.0;
                match var {
                    // NO WIDGET (Eden's `1`) — draw NOTHING. The selection still translates on a bare
                    // drag (the LG::Move path is variant-independent); a No-Widget mode with a gizmo
                    // would be a contradiction. An empty group keeps the `match` total.
                    transform::WidgetVariant::None => view! { <g></g> }.into_any(),
                    // TRANSLATE — X (east, +screen-x) and Y (north, −screen-y) arrows from the centre.
                    transform::WidgetVariant::Translate => view! {
                        <g>
                            // X axis arrow (east).
                            <line x1=move || format!("{cx:.1}") y1=move || format!("{cy:.1}")
                                  x2=move || format!("{:.1}", cx + R) y2=move || format!("{cy:.1}")
                                  class="stroke-primary" stroke-width="2" />
                            <polygon
                                points=move || format!(
                                    "{x0:.1},{y0:.1} {x1:.1},{y1:.1} {x1:.1},{y2:.1}",
                                    x0 = cx + R, y0 = cy,
                                    x1 = cx + R - HEAD, y1 = cy - HEAD * 0.7,
                                    y2 = cy + HEAD * 0.7)
                                class="fill-primary" />
                            // Y axis arrow (north = up on screen).
                            <line x1=move || format!("{cx:.1}") y1=move || format!("{cy:.1}")
                                  x2=move || format!("{cx:.1}") y2=move || format!("{:.1}", cy - R)
                                  class="stroke-primary" stroke-width="2" />
                            <polygon
                                points=move || format!(
                                    "{x0:.1},{y0:.1} {x1:.1},{y1:.1} {x2:.1},{y1:.1}",
                                    x0 = cx, y0 = cy - R,
                                    x1 = cx - HEAD * 0.7, y1 = cy - R + HEAD,
                                    x2 = cx + HEAD * 0.7)
                                class="fill-primary" />
                            <circle cx=move || format!("{cx:.1}") cy=move || format!("{cy:.1}")
                                    r="3" class="fill-primary" />
                        </g>
                    }.into_any(),
                    // ROTATE — a ring around the centre (drag = rotate; Shift+drag snaps).
                    transform::WidgetVariant::Rotate => view! {
                        <g>
                            <circle cx=move || format!("{cx:.1}") cy=move || format!("{cy:.1}")
                                    r=move || format!("{R:.1}")
                                    fill="none" class="stroke-primary" stroke-width="2" />
                            <circle cx=move || format!("{cx:.1}") cy=move || format!("{cy:.1}")
                                    r="3" class="fill-primary" />
                        </g>
                    }.into_any(),
                }
            })}
        </svg>
    }
}

/// T-795 — the CURSOR-ADJACENT MODE HINT: a tiny chip near the cursor naming the active transform
/// widget (`No Widget` / `Translate` / `Rotate`). One of the two active-mode indicators the ticket
/// asks for — the other is the toolbar's three-way toggle plate (T-799's owned strip, driven by the
/// `widget_digit` dispatch getter). Before T-795 no chrome anywhere told the operator which mode was
/// live, so pressing a digit was a silent state change (review F-16).
///
/// Same overlay idiom as [`TransformWidgetOverlay`]: full-bleed `pointer-events-none` band (the map's
/// own pointer handlers still get every event), re-running off the `cursor` heartbeat so the chip
/// tracks the pointer, and off `variant` so it relabels the instant a chord/button flips the mode.
/// Drawn only while the pointer is over the canvas (a `cursor` value is present) AND a widget is armed
/// — in `No Widget` mode the chip still shows, so the operator can SEE they dropped the gizmo (the
/// whole point of `1`), but it is the one mode that draws no gizmo. Positioned below-right of the
/// cursor, clamped nowhere (it is a hint, not a modal; a fuller clamp is later polish).
#[component]
pub(crate) fn WidgetModeHint(
    /// Pan heartbeat + pointer position — the chip follows the cursor and re-shows on move.
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// The live widget variant — decides the label and re-runs on a mode flip.
    variant: RwSignal<transform::WidgetVariant>,
) -> impl IntoView {
    view! {
        <div data-widget-mode-hint class="pointer-events-none absolute inset-0 z-10">
            {move || {
                let (x, y, _) = cursor.get()?;
                let label = variant.get().label();
                Some(view! {
                    <div
                        class="absolute rounded bg-surface/80 px-1.5 py-0.5 font-mono text-[11px] \
                               tabular-nums text-on-surface-variant shadow-sm"
                        style=move || format!("left:{:.0}px;top:{:.0}px", x + 16.0, y + 16.0)
                    >
                        {label}
                    </div>
                })
            }}
        </div>
    }
}

/// T-648 TOOLBAR-GRID-MOVE-001 — the snap-grid STATUS READOUT: the active step ladder in the
/// status-bar band (the T-636 readout idiom). Its own tiny `pointer-events-none` element rather than
/// a field inside `eden_toolbelt::StatusBar`, because that component is another slice's owned file —
/// this keeps the readout inside T-648's three-file boundary while sitting in the same band. Shows
/// `SNAP  move 5 m · rot 15°` (or `SNAP  off`), re-running off the `snap` signal (O-10 relabel).
#[component]
pub(crate) fn SnapReadout(snap: RwSignal<transform::SnapState>) -> impl IntoView {
    view! {
        <div
            data-snap-readout
            class="pointer-events-none absolute bottom-11 right-3 z-20 rounded bg-surface/70 px-2 \
                   py-0.5 font-mono text-[11px] tabular-nums text-on-surface-variant"
        >
            {move || snap.get().status_readout()}
        </div>
    }
}

/// T-647 PLACE-003 — the empty-ground asset picker: a floating list of placeable characters for the
/// active Eden side, opened by a double-click on empty ground. Picking a row ARMS a place
/// (`begin_place`, exactly what a DockRight leaf does) and closes the panel; the operator's next
/// canvas click lands it (the click-then-click contract PLACE-001).
///
/// **Why a floating picker, not "focus the dock's search".** The ticket offered either; this is the
/// cheaper FAITHFUL form under this slice's file boundary. It reuses the same `registry_items` +
/// `active_side` the dock's catalog is built from (`build_catalog_tree`), so a picked leaf arms the
/// identical place — no second catalog, no divergence. And it is self-contained: it does not touch
/// the DockRight, so it still works when Backspace has hidden the chrome (a hidden dock can't be
/// focused — the ticket's own guard). Boot `Failed`/no-engine never opens it: the `dblclick` handler
/// returns on a `None` engine before it can call `open_asset_picker`.
#[component]
pub(crate) fn AssetPickerOverlay(
    picker: RwSignal<Option<AssetPickerState>>,
    registry: RwSignal<Option<Vec<crate::core::dto::RegistryItem>>>,
    active_side: RwSignal<String>,
) -> impl IntoView {
    // A live query filters the flat leaf list (Eden's create-menu type-ahead). Reset on each open so
    // a stale query never leaks in.
    let query = RwSignal::new(String::new());
    Effect::new(move |_| {
        if picker.get().is_some() {
            query.set(String::new());
        }
    });
    // Esc closes (mirrors the context menu). No-op while the picker is closed.
    // T-726 — register with the modal stack so a stacked overlay (or this picker over the
    // measure-tool Esc seam) owns Escape alone; topmost consumes.
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::core::ui::modal_stack::register(move || {
            picker.try_get_untracked().flatten().is_some()
        });
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            if picker.get_untracked().is_some()
                && ev.key() == "Escape"
                && crate::core::ui::modal_stack::is_topmost_open(modal_id)
            {
                ev.prevent_default();
                editor_ops::close_asset_picker();
            }
        });
        on_cleanup(move || {
            key.remove();
            crate::core::ui::modal_stack::unregister(modal_id);
        });
    }

    move || {
        let state = picker.get()?;
        // Flatten the side-filtered character catalog to placeable leaves (label + payload). Folders
        // carry no payload, so `payload.is_some()` is exactly "a placeable leaf" (asset_catalog docs).
        let items = registry.get().unwrap_or_default();
        let tree =
            crate::editor::arsenal::asset_catalog::build_catalog_tree(&items, &active_side.get());
        let mut leaves: Vec<(String, crate::editor::arsenal::asset_catalog::PlacePayload)> =
            Vec::new();
        fn collect(
            nodes: &[crate::editor::arsenal::asset_catalog::CatalogNode],
            out: &mut Vec<(String, crate::editor::arsenal::asset_catalog::PlacePayload)>,
        ) {
            for n in nodes {
                if let Some(p) = &n.payload {
                    out.push((n.label.clone(), p.clone()));
                }
                collect(&n.children, out);
            }
        }
        collect(&tree, &mut leaves);
        let q = query.get().trim().to_lowercase();
        if !q.is_empty() {
            leaves.retain(|(label, _)| label.to_lowercase().contains(&q));
        }
        // Anchor at the dblclick pixel (like the context menu). `max-h` + scroll keep a long list on
        // screen; a fuller flip/clamp is later polish — this slice ships the picker + its arm.
        let pos = format!("left:{:.0}px;top:{:.0}px", state.screen_x, state.screen_y);
        let rows = leaves
            .into_iter()
            .map(|(label, payload)| {
                view! {
                    <button
                        class="block w-full truncate px-3 py-1.5 text-left text-sm text-on-surface hover:bg-primary/20"
                        on:click=move |ev| {
                            ev.stop_propagation();
                            // T-723 — arm on `click` (fires AFTER pointerup), not `pointerdown`.
                            // Arming on pointerdown made the selecting release land on the canvas
                            // (picker unmounted) and place at the ROW's screen position — wx/wy
                            // were never read. Click-then-click: this arms; the next canvas LMB
                            // places. `editor_ops` is wasm-only, so the arm is gated.
                            #[cfg(target_arch = "wasm32")]
                            {
                                editor_ops::begin_place(payload.clone());
                                editor_ops::close_asset_picker();
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &payload;
                        }
                    >
                        {label}
                    </button>
                }
            })
            .collect_view();
        Some(view! {
            // Click-away backdrop — transparent, full-screen, closes on any click (context-menu
            // idiom). `z-40` under the panel (`z-50`) but over the map/chrome.
            <div
                class="fixed inset-0 z-40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    editor_ops::close_asset_picker();
                }
                on:contextmenu=move |ev| ev.prevent_default()
            ></div>
            <div
                class="glass animate-dialog-in fixed z-50 flex max-h-[22rem] w-64 flex-col overflow-hidden rounded-md border border-outline-variant/30 shadow-2xl outline-none"
                style=pos
                on:contextmenu=move |ev| ev.prevent_default()
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                <div class="border-b border-outline-variant/25 px-2 py-1.5">
                    <input
                        type="search"
                        class="w-full rounded bg-surface/40 px-2 py-1 text-sm text-on-surface outline-none placeholder:text-on-surface-variant"
                        placeholder="Place asset…"
                        on:input=move |ev| query.set(event_target_value(&ev))
                    />
                </div>
                <div class="min-h-0 flex-1 overflow-y-auto py-1">{rows}</div>
            </div>
        })
    }
}

/// T-651 (`PLACE-COMMENT-001`) — **the comment editor**: the one surface that authors all three
/// `ATTR-FIELD-CMT-*` fields, plus the COPY and DELETE verbs. Opened by double-clicking a comment row
/// in the Outliner; renders no DOM while closed.
///
/// **Why its own overlay and not the Attributes modal.** Attributes reads the slot SoA
/// (`editor_ops::read_attrs`), and a comment is not in it — a comment never reaches `materialize`
/// at all, which is the same property that keeps it out of the render and off the compiled mission.
/// Pointing Attributes at a comment id would open a dialog with every field blank and every write a
/// no-op: the T-716 "live-but-inert" failure this codebase already names.
///
/// **Where each verb lands.** Title/tooltip/position write through `set_comment_*`, one core
/// transaction each, so each committed edit is one Ctrl+Z. The POSITION pair is also the drag
/// commit's surface: with a comment absent from the render SoA there is nothing on the map to grab,
/// so typed coordinates are the honest form of "drag" for this ticket — the doc-side mutator
/// (`move_comment`) is the same one a future map-drawn comment glyph would call, so wiring a
/// pointer drag later changes the CALLER and nothing else. Duplicate is the copy verb; Delete
/// removes the row and unfiles it from its folder. Filing into a layer is the Outliner drag, not a
/// control here.
///
/// Commits on `change` (blur / Enter), not on every keystroke: a per-character write would put one
/// undo step per letter on the stack of a field whose whole purpose is long prose.
#[component]
pub(crate) fn CommentEditorOverlay(
    open: RwSignal<Option<String>>,
    doc_tick: RwSignal<u64>,
) -> impl IntoView {
    // Esc closes (the picker / context-menu idiom).
    // T-726 — modal-stack gate; topmost consumes.
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::core::ui::modal_stack::register(move || {
            open.try_get_untracked().flatten().is_some()
        });
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked().is_some()
                && ev.key() == "Escape"
                && crate::core::ui::modal_stack::is_topmost_open(modal_id)
            {
                ev.prevent_default();
                editor_ops::close_comment_editor();
            }
        });
        on_cleanup(move || {
            key.remove();
            crate::core::ui::modal_stack::unregister(modal_id);
        });
    }

    move || {
        let id = open.get()?;
        // `doc_tick` is the reactive re-read trigger (the Attributes-modal idiom): an undo, a
        // duplicate or an outliner refile bumps it and this panel re-reads the row.
        let _ = doc_tick.get();
        #[cfg(target_arch = "wasm32")]
        let row = editor_ops::read_comment(&id);
        #[cfg(not(target_arch = "wasm32"))]
        let row: Option<()> = None;
        // The row vanished (deleted, or undone away while the panel was open) — close rather than
        // edit a ghost. Returning `None` renders nothing; the signal is cleared on the next open.
        let (title, tooltip, x, z) = match &row {
            #[cfg(target_arch = "wasm32")]
            Some(c) => (c.title.clone(), c.tooltip.clone(), c.x, c.z),
            #[cfg(not(target_arch = "wasm32"))]
            Some(()) => (String::new(), String::new(), 0.0, 0.0),
            None => return None,
        };
        let (id_title, id_tip, id_x, id_z, id_dup, id_del) = (
            id.clone(),
            id.clone(),
            id.clone(),
            id.clone(),
            id.clone(),
            id.clone(),
        );
        let (x_for_z, z_for_x) = (x, z);
        Some(view! {
            <div
                class="fixed inset-0 z-40 bg-scrim/40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    editor_ops::close_comment_editor();
                }
            ></div>
            <div
                class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex w-[min(28rem,92vw)] -translate-x-1/2 -translate-y-1/2 flex-col gap-3 rounded-xl border border-outline-variant/30 p-4 shadow-2xl outline-none"
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                <div class="flex items-center gap-2">
                    <span class="font-label-md text-label-md text-on-surface">"Comment"</span>
                    <span class="ml-auto font-code-sm text-code-sm text-on-surface-variant">
                        {id.clone()}
                    </span>
                </div>
                // ATTR-FIELD-CMT-TITLE
                <div class="space-y-1">
                    <label class="font-label-sm text-[11px] text-on-surface-variant">"Title"</label>
                    <input
                        type="text"
                        class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-label-md text-label-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                        prop:value=title
                        on:change=move |ev| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                editor_ops::rename_comment(
                                    id_title.clone(),
                                    event_target_value(&ev),
                                );
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = (&id_title, &ev);
                        }
                    />
                </div>
                // ATTR-FIELD-CMT-TOOLTIP — a textarea, not an input: FNF v3's surviving in-map
                // instructions ran to seven paragraphs, and a single-line box would make the field
                // useless for the one job it exists to do.
                <div class="space-y-1">
                    <label class="font-label-sm text-[11px] text-on-surface-variant">"Tooltip"</label>
                    <textarea
                        rows="5"
                        class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-label-md text-label-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                        prop:value=tooltip
                        on:change=move |ev| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                editor_ops::set_comment_tooltip(
                                    id_tip.clone(),
                                    event_target_value(&ev),
                                );
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = (&id_tip, &ev);
                        }
                    ></textarea>
                </div>
                // ATTR-FIELD-CMT-POSITION — world metres, `{x, z}` (the marker / zone-centre
                // vocabulary, never `{x, y}`). A non-numeric entry is ignored rather than written as
                // 0, which would teleport the note to the terrain corner on a stray keystroke.
                <div class="flex gap-2">
                    <div class="flex-1 space-y-1">
                        <label class="font-label-sm text-[11px] text-on-surface-variant">"X (m)"</label>
                        <input
                            type="number"
                            class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-code-md text-code-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                            prop:value=x
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                if let Ok(v) = event_target_value(&ev).trim().parse::<f64>() {
                                    editor_ops::move_comment(id_x.clone(), v, z_for_x);
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = (&id_x, &ev, z_for_x);
                            }
                        />
                    </div>
                    <div class="flex-1 space-y-1">
                        <label class="font-label-sm text-[11px] text-on-surface-variant">"Z (m)"</label>
                        <input
                            type="number"
                            class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-code-md text-code-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                            prop:value=z
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                if let Ok(v) = event_target_value(&ev).trim().parse::<f64>() {
                                    editor_ops::move_comment(id_z.clone(), x_for_z, v);
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = (&id_z, &ev, x_for_z);
                            }
                        />
                    </div>
                </div>
                <div class="flex items-center gap-2 pt-1">
                    // COPY. The new comment lands in the same folder, offset so it is not stacked
                    // invisibly on its source. The panel follows the copy — that is what makes the
                    // duplicate immediately editable instead of leaving the operator on the original.
                    <button
                        type="button"
                        class="rounded border border-border-subtle px-3 py-1.5 text-label-md text-on-surface hover:bg-primary/15"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            if let Some(new_id) =
                                editor_ops::duplicate_comment(&id_dup, COMMENT_COPY_OFFSET_M)
                            {
                                editor_ops::open_comment_editor(new_id);
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_dup;
                        }
                    >
                        "Duplicate"
                    </button>
                    <button
                        type="button"
                        class="rounded border border-error/50 px-3 py-1.5 text-label-md text-error hover:bg-error/15"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                editor_ops::delete_comment(id_del.clone());
                                editor_ops::close_comment_editor();
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_del;
                        }
                    >
                        "Delete"
                    </button>
                    <button
                        type="button"
                        class="ml-auto rounded bg-primary px-3 py-1.5 text-label-md text-on-primary"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            editor_ops::close_comment_editor();
                        }
                    >
                        "Close"
                    </button>
                </div>
            </div>
        })
    }
}

/// T-651 — how far a duplicated comment is offset from its source, in metres. Non-zero so the copy
/// is a distinct, clickable row rather than a perfect overlay of the original.
const COMMENT_COPY_OFFSET_M: f64 = 25.0;

/// T-672 — one Connections-panel row, flattened for rendering: the edge plus the findings that name
/// it. Target-independent on purpose — `editor_ops` is wasm-only, so this is what lets the panel have
/// ONE view body instead of a wasm branch and an untested native twin.
struct ConnRowView {
    kind: String,
    /// `"SL (s0) → Rifleman (s1)"` — both endpoints, label-resolved.
    head: String,
    id: String,
    /// `"CONN-DANGLING: to endpoint `x` is not a placed entity"`, one per finding on this row.
    problems: Vec<String>,
}

/// T-672 — **the Connections panel: the connection graph's SEE and CHECK surface.**
///
/// This component is the ticket's primary constraint made concrete. The framework corpus records
/// FNF v4's entire defect cluster on the connection mechanism, with the instruction "the inspector
/// and the validation rules must precede the edges — do not ship edges you cannot see or check".
/// A connection has **no map glyph** in this slice (see the `LaneRole::SquadLinks` trace note on
/// `editor_ops`'s connection block), so this panel is the ONLY place an operator can observe the
/// graph they are authoring, audit it, or delete from it. It is not an inspector bolted onto the
/// feature; it is the feature's only surface, and the edge verbs hang off it.
///
/// Three things, in the order they matter:
///   1. **EVERY edge, listed** — `kind`, `from → to` with resolved labels, in `map-engine-core`'s
///      stable content order (so the rows never reshuffle under the cursor between reads).
///   2. **EVERY finding, listed** — the four graph rules (`CONN-SELF` / `CONN-DANGLING` /
///      `CONN-DUPLICATE` / `CONN-CYCLE`, plus `CONN-KIND` for a hydrated foreign vocabulary),
///      rendered against the row they belong to AND summarised at the top, because a warning the
///      operator must scroll to find is a warning they will not read.
///   3. **A delete per row** (`CONN-DEL-001`). Eden deletes a connection by selecting its line and
///      pressing Del; there is no line here, so the addressable row is the selection.
///
/// It also shows the ARMED connect, if any, with its own cancel — the two-act connect gesture's only
/// persistent state, which would otherwise be invisible between the two right-clicks.
///
/// `doc_tick` is the reactive re-read trigger (the Attributes-modal / comment-editor idiom): a draw,
/// a delete, an undo or a hydrate bumps it and this panel re-reads. There is no doc change
/// subscription, so a panel that read once would be a stale audit — which is worse than no audit.
///
/// Mounted UNGATED beside the other floating overlays: an audit surface the operator deliberately
/// opened is not dock chrome and must survive a Backspace hide-chrome (the wave-101 mount rule).
#[component]
pub(crate) fn ConnectionsPanelOverlay(
    open: RwSignal<bool>,
    doc_tick: RwSignal<u64>,
) -> impl IntoView {
    // Esc closes (the picker / context-menu / comment-editor idiom).
    // T-726 — modal-stack gate; topmost consumes.
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::core::ui::modal_stack::register(move || {
            open.try_get_untracked().unwrap_or(false)
        });
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked()
                && ev.key() == "Escape"
                && crate::core::ui::modal_stack::is_topmost_open(modal_id)
            {
                ev.prevent_default();
                editor_ops::close_connections_panel();
            }
        });
        on_cleanup(move || {
            key.remove();
            crate::core::ui::modal_stack::unregister(modal_id);
        });
    }

    move || {
        if !open.get() {
            return None;
        }
        // Re-read on every doc mutation — see the component note.
        let _ = doc_tick.get();
        // `editor_ops` is a wasm-only module, so the doc read is behind a cfg and the whole panel
        // is expressed over the target-independent [`ConnRowView`]. That keeps ONE view body for
        // both targets — the native build renders the same empty-state DOM rather than a second,
        // untested branch (the shape `CommentEditorOverlay` uses, for the same reason).
        #[cfg(target_arch = "wasm32")]
        let (rows, finding_count, armed_line) = {
            let list = editor_ops::connection_list();
            let findings = editor_ops::connection_findings();
            // Findings keyed by the row they belong to. Built once here rather than re-scanned per
            // row: a graph with N edges and N findings would otherwise be quadratic, and the panel
            // re-renders on every document mutation.
            let mut by_row: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();
            for f in &findings {
                by_row
                    .entry(f.connection_id.clone())
                    .or_default()
                    .push(format!("{}: {}", f.code, f.detail));
            }
            let rows: Vec<ConnRowView> = list
                .into_iter()
                .map(|r| ConnRowView {
                    problems: by_row.get(&r.id).cloned().unwrap_or_default(),
                    head: format!("{} \u{2192} {}", r.from_label, r.to_label),
                    kind: r.kind,
                    id: r.id,
                })
                .collect();
            let armed_line = editor_ops::pending_connect()
                .map(|(kind, from)| format!("Connecting: {kind} from {from}"));
            (rows, findings.len(), armed_line)
        };
        #[cfg(not(target_arch = "wasm32"))]
        let (rows, finding_count, armed_line): (Vec<ConnRowView>, usize, Option<String>) =
            (Vec::new(), 0, None);

        let total = rows.len();
        // Bound out of the `view!` macro: `class:` takes a value, not a comparison expression.
        let clean = finding_count == 0;
        let empty = total == 0;

        let row_views = rows
            .into_iter()
            .map(|r| {
                let bad = !r.problems.is_empty();
                let (problems, del_id, head) = (r.problems, r.id.clone(), r.head);
                view! {
                    <div class="flex flex-col gap-0.5 border-b border-outline-variant/20 py-1.5 last:border-b-0">
                        <div class="flex items-center gap-2">
                            <span
                                class="shrink-0 rounded px-1.5 py-0.5 font-code-sm text-code-sm"
                                class:bg-surface-dim=!bad
                                class:text-on-surface-variant=!bad
                                class:bg-error-container=bad
                                class:text-on-error-container=bad
                            >
                                {r.kind}
                            </span>
                            <span class="flex-1 truncate font-label-md text-label-md text-on-surface">
                                {head}
                            </span>
                            <span class="shrink-0 font-code-sm text-code-sm text-outline">
                                {r.id}
                            </span>
                            <button
                                type="button"
                                title="Delete this connection (CONN-DEL-001) — one Ctrl+Z restores it"
                                class="shrink-0 cursor-pointer rounded px-2 py-0.5 font-label-sm text-[11px] text-on-surface-variant hover:bg-error-container hover:text-on-error-container"
                                on:click=move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        editor_ops::delete_connection(&del_id);
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let _ = &del_id;
                                }
                            >
                                "Delete"
                            </button>
                        </div>
                        {(!problems.is_empty())
                            .then(|| {
                                problems
                                    .into_iter()
                                    .map(|p| {
                                        view! {
                                            <div class="pl-2 font-code-sm text-code-sm text-error">
                                                {p}
                                            </div>
                                        }
                                    })
                                    .collect_view()
                            })}
                    </div>
                }
            })
            .collect_view();

        Some(view! {
            <div
                class="fixed inset-0 z-40 bg-scrim/40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    editor_ops::close_connections_panel();
                }
            ></div>
            <div
                class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[80vh] w-[min(40rem,94vw)] -translate-x-1/2 -translate-y-1/2 flex-col gap-3 rounded-xl border border-outline-variant/30 p-4 shadow-2xl outline-none"
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                <div class="flex items-center gap-2">
                    <span class="font-label-md text-label-md text-on-surface">"Connections"</span>
                    <span class="font-code-sm text-code-sm text-on-surface-variant">
                        {format!("{total} edge(s)")}
                    </span>
                    <button
                        type="button"
                        class="ml-auto cursor-pointer rounded px-2 py-1 font-label-sm text-[11px] text-on-surface-variant hover:bg-surface-dim"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            editor_ops::close_connections_panel();
                        }
                    >
                        "Close"
                    </button>
                </div>
                // The armed connect — the two-act gesture's only persistent state, which is
                // otherwise invisible between the two right-clicks.
                {armed_line
                    .map(|line| {
                        view! {
                            <div class="flex items-center gap-2 rounded border border-primary/40 bg-surface-dim px-2 py-1">
                                <span class="flex-1 truncate font-code-sm text-code-sm text-on-surface">
                                    {line}
                                </span>
                                <button
                                    type="button"
                                    class="shrink-0 cursor-pointer rounded px-2 py-0.5 font-label-sm text-[11px] text-on-surface-variant hover:bg-surface-bright"
                                    on:click=move |_| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            editor_ops::cancel_connect();
                                            editor_ops::open_connections_panel();
                                        }
                                    }
                                >
                                    "Cancel"
                                </button>
                            </div>
                        }
                    })}
                // The CHECK summary. Rendered at the TOP and unconditionally (including the clean
                // "no problems" case), because a validation surface that only appears when something
                // is wrong cannot be distinguished from one that is broken.
                <div
                    class="rounded px-2 py-1 font-label-sm text-[11px]"
                    class:bg-surface-dim=clean
                    class:text-on-surface-variant=clean
                    class:bg-error-container=!clean
                    class:text-on-error-container=!clean
                >
                    {if clean {
                        "No problems found in the connection graph.".to_string()
                    } else {
                        format!(
                            "{finding_count} problem(s): dangling endpoints, self-links, duplicates or ownership cycles — see the rows below.",
                        )
                    }}
                </div>
                <div class="min-h-0 flex-1 overflow-y-auto">
                    {if empty {
                        view! {
                            <div class="py-6 text-center font-label-sm text-[11px] text-on-surface-variant">
                                "No connections yet. Right-click a unit → Connect → pick a relation, then left-click the target (or right-click it and choose Complete Connection)."
                            </div>
                        }
                            .into_any()
                    } else {
                        row_views.into_any()
                    }}
                </div>
            </div>
        })
    }
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
pub(crate) fn ConflictDialog(
    conflict: RwSignal<Option<ConflictInfo>>,
    conflict_id: String,
) -> impl IntoView {
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
                                mission_hydrate::resolve_conflict_local(
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
                                mission_hydrate::resolve_conflict_server(
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
