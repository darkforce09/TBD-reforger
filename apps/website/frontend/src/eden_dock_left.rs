//! T-661 — the left dock (Editor Layers outliner), split from `eden_chrome.rs`.
//!
//! Click a folder to make it the drop target, a slot to select it. The ORBAT browse/select tree
//! moved to the top-strip ORBAT Manager modal (T-177 B1); this dock is Editor Layers only.
//!
//! T-638 — the dock COLLAPSES to a 24×24 stub in its outer (top-LEFT) corner, toggled by the tab-strip
//! chevron or the `E` key ([`crate::mission_editor`]'s editor keydown). Collapsed is not a rail and
//! not a vanish: the panel becomes exactly the stub, docked at the corner, overlaying the map, and the
//! freed width reflows into the map pane (the inset accessors in [`crate::eden_layout`] carry it). The
//! chevron glyph points OUTWARD when expanded (« left) and FLIPS when collapsed (» — "expand me"),
//! occupying the same 24×24 box either way (Eden's mechanism, measured across the 75 screenshots).
//!
//! T-696 — the dock gained a SECOND TAB beside the layers tree: **Locations**, Eden's Locations-tab
//! precedent. It holds the two halves of NEW-F7 / 3den E1 — the read-only **named-locations index**
//! (the town names the map already draws) and user-created **map bookmarks** (named camera
//! positions, persisted). Both halves fly the camera through the ONE existing mover; neither is a
//! document edit. See the `T-696` section at the foot of this file.
#![allow(dead_code)]
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::eden_layout::{DOCK_L, STUB_PX};
use crate::eden_tree::virtual_tree;
use crate::outliner::OutlinerNode;
use crate::ui::MaterialIcon;

/// T-638 — the collapse/expand chevron shared by both docks. `outward_icon` is the glyph shown while
/// EXPANDED (points out of the dock — `chevron_left` for the left dock, `chevron_right` for the
/// right); the collapsed state shows the OTHER chevron in the SAME 24×24 box (the "flip the glyph"
/// rule). `at_start` places it at the row's start (left dock, outer corner = top-left) vs end (right
/// dock, top-right). The button flips `collapsed`; `mission_editor` observes that signal to mirror the
/// [`crate::eden_layout`] inset latch + run the reflow/centre-hold, so the chevron itself stays a pure
/// toggle.
pub fn collapse_chevron(collapsed: RwSignal<bool>, expanded_is_left: bool) -> impl IntoView {
    let title = move || {
        if collapsed.get() {
            "Expand panel"
        } else {
            "Collapse panel"
        }
    };
    // Expanded → point outward; collapsed → point inward (toward the map, i.e. "open me").
    let icon = move || match (collapsed.get(), expanded_is_left) {
        (false, true) => "chevron_left",   // left dock, expanded: « outward
        (true, true) => "chevron_right",   // left dock, collapsed: » (expand)
        (false, false) => "chevron_right", // right dock, expanded: » outward
        (true, false) => "chevron_left",   // right dock, collapsed: « (expand)
    };
    view! {
        <button
            type="button"
            aria-label=title
            aria-expanded=move || (!collapsed.get()).to_string()
            title=title
            // 24×24 hit-box (STUB_PX), matching the collapsed stub bbox exactly.
            class="flex size-6 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
            on:click=move |ev: web_sys::MouseEvent| {
                ev.stop_propagation();
                collapsed.update(|c| *c = !*c);
            }
        >
            {move || view! { <MaterialIcon name=icon() class="block text-base" /> }}
        </button>
    }
}

/// Left dock — the live **Editor Layers** outliner (spec O1). Click a folder to make it the drop
/// target, a slot to select it (no camera move — React parity).
///
/// T-177 B1 — the ORBAT browse/select tree moved OUT of this dock (the dual-tree split was bad UX)
/// into the top-strip **ORBAT Manager** modal ([`OrbatManagerDialog`], the T-071.0 cutover). Squad
/// MANAGEMENT (reparent/rename/delete) stays T-071.1+. This dock is now Editor Layers only.
///
/// T-638 — `collapsed` collapses this dock to the [`STUB_PX`]-square corner stub; the `E` key and the
/// tab-strip chevron both flip it (see [`collapse_chevron`]).
#[component]
pub fn DockLeft(
    /// The Editor Layers tree, rebuilt from the doc at every mutation (`editor_ops::refresh_docks`).
    nodes: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    /// T-638 — collapse latch (owned by `mission_editor`; `E`/chevron toggle it, the accessor + reflow
    /// observe it).
    collapsed: RwSignal<bool>,
) -> impl IntoView {
    // T-172 B9 — screen-05 bottom icon strip: React's LeftSidebar BOTTOM_TABS were explicitly
    // visual-only (Hierarchy active), so present-but-disabled is the honest parity.
    let strip_btn = |icon: &'static str, label: &'static str, active: bool| {
        view! {
            <button
                type="button"
                disabled=true
                title=label
                aria-label=label
                class=if active {
                    "rounded-md p-1.5 text-primary"
                } else {
                    "rounded-md p-1.5 text-outline"
                }
            >
                <MaterialIcon name=icon class="block text-base" />
            </button>
        }
    };
    // ── T-696 — the Locations tab's state ────────────────────────────────────────────────────────
    // All of it is dock-local SESSION state: which tab is showing, the shared filter box, the lazily
    // fetched named-place index, and the persisted bookmark collection mirrored into a signal so the
    // list re-renders on a verb. NOTHING here is document state, so nothing here is undoable (T-642's
    // ruler rule: overlay/session state is not authored content).
    let tab = RwSignal::new(LeftTab::Layers);
    let query = RwSignal::new(String::new());
    let places = RwSignal::new(Vec::<NamedPlace>::new());
    // Latch so the one-shot index fetch is armed by the FIRST Locations visit and never again.
    let places_armed = RwSignal::new(false);
    let bookmarks = RwSignal::new(load_bookmarks());
    // `Some((existing name, live text))` while a bookmark row is being inline-renamed; `None` = idle.
    let renaming = RwSignal::new(Option::<(String, String)>::None);
    // `Some(live text)` while the "bookmark this view" input is armed; `None` = idle.
    let adding = RwSignal::new(Option::<String>::None);

    // Persist + re-render in one place, so no verb can mutate the collection without writing it.
    let commit_bookmarks = move |next: Bookmarks| {
        save_bookmarks(&next);
        bookmarks.set(next);
    };

    // The one-shot index load. The named places are NOT re-sourced: this re-runs the SAME
    // `parse_locations_json` over the SAME `/map-assets/<terrain>/locations.json` the town-label lane
    // reads (`world_assets::labels::LabelHost::init`), because that module's parsed set is a private
    // field of a private wasm-only module. See the section note at the foot of this file.
    let arm_places = move || {
        if places_armed.get_untracked() {
            return;
        }
        places_armed.set(true);
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            places.set(fetch_named_places().await);
        });
    };

    let tab_btn = move |t: LeftTab, label: &'static str, testid: &'static str| {
        view! {
            <button
                type="button"
                role="tab"
                data-testid=testid
                aria-selected=move || (tab.get() == t).to_string()
                class=move || {
                    if tab.get() == t {
                        "rounded px-1.5 py-0.5 text-label-sm font-semibold uppercase tracking-wide text-on-surface"
                    } else {
                        "rounded px-1.5 py-0.5 text-label-sm font-semibold uppercase tracking-wide text-outline transition-colors hover:text-on-surface"
                    }
                }
                on:click=move |ev: web_sys::MouseEvent| {
                    ev.stop_propagation();
                    tab.set(t);
                    if t == LeftTab::Places {
                        arm_places();
                    }
                }
            >
                {label}
            </button>
        }
    };

    // A "fly to" row: the label is the button (the whole name is the target), trailing children are
    // the row's hover actions. Shared by both halves so the two lists cannot drift apart.
    let fly_row = move |label: String,
                        title: String,
                        x: f64,
                        y: f64,
                        zoom: Option<f64>,
                        actions: AnyView| {
        view! {
            <div class="group flex items-center gap-1 rounded px-1 py-0.5 hover:bg-white/5">
                <button
                    type="button"
                    title=title
                    class="min-w-0 flex-1 cursor-pointer truncate text-left text-label-sm text-on-surface"
                    on:click=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        fly_to(x, y, zoom);
                    }
                >
                    {label}
                </button>
                {actions}
            </div>
        }
    };

    // T-696 — the Locations tab body: one filter box over TWO lists. Bookmarks first (the operator's
    // own places, the scarcer set), then the read-only named-locations index.
    let places_body = move || {
        view! {
            <div class="mt-1 flex min-h-0 flex-1 flex-col gap-2">
                <input
                    type="text"
                    data-testid="dock-left-places-filter"
                    aria-label="Filter bookmarks and locations"
                    placeholder="Filter places…"
                    prop:value=move || query.get()
                    class="w-full rounded border border-outline-variant/30 bg-black/20 px-1.5 py-1 text-label-sm text-on-surface placeholder:text-outline"
                    on:input=move |ev| query.set(event_target_value(&ev))
                />
                <section class="flex min-h-0 flex-col">
                    <h3 class="px-1 text-label-sm font-semibold uppercase tracking-wide text-outline">
                        "Bookmarks"
                    </h3>
                    // The ADD verb's inline input (no `window.prompt` — that is banned here and is not
                    // gate-driveable). Enter commits at the LIVE camera; Escape/blur cancels.
                    {move || {
                        adding
                            .get()
                            .map(|seed| {
                                view! {
                                    <input
                                        type="text"
                                        data-testid="dock-left-bookmark-name"
                                        aria-label="Bookmark name"
                                        autofocus
                                        prop:value=seed
                                        class="mx-1 my-0.5 rounded border border-primary/60 bg-black/30 px-1.5 py-1 text-label-sm text-on-surface"
                                        on:input=move |ev| adding.set(Some(event_target_value(&ev)))
                                        on:blur=move |_| adding.set(None)
                                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                                            match ev.key().as_str() {
                                                "Enter" => {
                                                    ev.prevent_default();
                                                    let name = adding.get_untracked().unwrap_or_default();
                                                    if let Some((x, y, zoom)) = live_camera() {
                                                        let mut next = bookmarks.get_untracked();
                                                        if next.add(&name, x, y, zoom) {
                                                            commit_bookmarks(next);
                                                        }
                                                    }
                                                    adding.set(None);
                                                }
                                                "Escape" => {
                                                    ev.stop_propagation();
                                                    adding.set(None);
                                                }
                                                _ => {}
                                            }
                                        }
                                    />
                                }
                            })
                    }}
                    <div class="max-h-40 overflow-y-auto" data-testid="dock-left-bookmark-list">
                        {move || {
                            let q = query.get();
                            let editing = renaming.get();
                            let rows = bookmarks.with(|b| filter_bookmarks(b, &q));
                            if rows.is_empty() {
                                return view! {
                                    <p class="px-1 py-1 text-label-sm text-outline">
                                        "No bookmarks yet — frame a view and use the bookmark button."
                                    </p>
                                }
                                    .into_any();
                            }
                            rows.into_iter()
                                .map(|bm| {
                                    let key = bm.name.clone();
                                    let live = editing
                                        .as_ref()
                                        .filter(|(rid, _)| rid == &key)
                                        .map(|(_, text)| text.clone());
                                    if let Some(text) = live {
                                        let from = key.clone();
                                        return view! {
                                            <input
                                                type="text"
                                                data-testid="dock-left-bookmark-rename"
                                                aria-label="Rename bookmark"
                                                autofocus
                                                prop:value=text
                                                class="mx-1 my-0.5 w-[calc(100%-0.5rem)] rounded border border-primary/60 bg-black/30 px-1.5 py-1 text-label-sm text-on-surface"
                                                on:input=move |ev| {
                                                    let v = event_target_value(&ev);
                                                    renaming
                                                        .update(|r| {
                                                            if let Some((_, t)) = r.as_mut() {
                                                                *t = v;
                                                            }
                                                        });
                                                }
                                                on:blur=move |_| renaming.set(None)
                                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                    match ev.key().as_str() {
                                                        "Enter" => {
                                                            ev.prevent_default();
                                                            if let Some((_, to)) = renaming.get_untracked() {
                                                                let mut next = bookmarks.get_untracked();
                                                                if next.rename(&from, &to) {
                                                                    commit_bookmarks(next);
                                                                }
                                                            }
                                                            renaming.set(None);
                                                        }
                                                        "Escape" => {
                                                            ev.stop_propagation();
                                                            renaming.set(None);
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            />
                                        }
                                            .into_any();
                                    }
                                    let rename_key = key.clone();
                                    let remove_key = key.clone();
                                    let actions = view! {
                                        <span class="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                                            <button
                                                type="button"
                                                aria-label="Rename bookmark"
                                                title="Rename"
                                                class="flex size-5 cursor-pointer items-center justify-center rounded text-outline hover:bg-white/10 hover:text-on-surface"
                                                on:click=move |ev: web_sys::MouseEvent| {
                                                    ev.stop_propagation();
                                                    renaming
                                                        .set(Some((rename_key.clone(), rename_key.clone())));
                                                }
                                            >
                                                <MaterialIcon name="edit" class="block text-sm" />
                                            </button>
                                            <button
                                                type="button"
                                                aria-label="Remove bookmark"
                                                title="Remove"
                                                class="flex size-5 cursor-pointer items-center justify-center rounded text-outline hover:bg-white/10 hover:text-error"
                                                on:click=move |ev: web_sys::MouseEvent| {
                                                    ev.stop_propagation();
                                                    let mut next = bookmarks.get_untracked();
                                                    next.remove(&remove_key);
                                                    commit_bookmarks(next);
                                                }
                                            >
                                                <MaterialIcon name="close" class="block text-sm" />
                                            </button>
                                        </span>
                                    }
                                        .into_any();
                                    // A bookmark restores its ZOOM as well as its centre — that is what
                                    // makes it a saved VIEW rather than a saved point.
                                    fly_row(
                                            key,
                                            format!(
                                                "Fly to {} ({:.0} m, {:.0} m, zoom {:.1})",
                                                bm.name,
                                                bm.x,
                                                bm.y,
                                                bm.zoom,
                                            ),
                                            bm.x,
                                            bm.y,
                                            Some(bm.zoom),
                                            actions,
                                        )
                                        .into_any()
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>
                </section>
                <section class="flex min-h-0 flex-1 flex-col">
                    <h3 class="px-1 text-label-sm font-semibold uppercase tracking-wide text-outline">
                        "Locations"
                    </h3>
                    <div
                        class="min-h-0 flex-1 overflow-y-auto"
                        data-testid="dock-left-location-list"
                    >
                        {move || {
                            let q = query.get();
                            let rows = places.with(|p| filter_places(p, &q));
                            if rows.is_empty() {
                                return view! {
                                    <p class="px-1 py-1 text-label-sm text-outline">
                                        {move || {
                                            if places.with(Vec::is_empty) {
                                                "No named locations for this terrain."
                                            } else {
                                                "No location matches that filter."
                                            }
                                        }}
                                    </p>
                                }
                                    .into_any();
                            }
                            rows.into_iter()
                                .map(|p| {
                                    let kind = p.kind.clone();
                                    let badge = view! {
                                        <span class="shrink-0 text-label-sm lowercase text-outline">{kind}</span>
                                    }
                                        .into_any();
                                    // A named place carries no authored zoom, so flying to one KEEPS the
                                    // current zoom (the `center_on_selection` rule).
                                    fly_row(
                                            p.name.clone(),
                                            format!("Fly to {} ({:.0} m, {:.0} m)", p.name, p.x, p.y),
                                            p.x,
                                            p.y,
                                            None,
                                            badge,
                                        )
                                        .into_any()
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>
                </section>
            </div>
        }
    };

    let full = move || {
        view! {
            <aside class=DOCK_L>
                // T-666 — header: the title doubles as a ROOT DROPZONE (drop a dragged folder here to
                // move it to the root — `complete_layer_drop_onto_root`), plus a "+" create button
                // (LAYER-CREATE-001: new folder under the active layer, or a root when none is active).
                // The dropzone is the whole header row so it is an easy target; `pointerup` completes an
                // armed folder drag and no-ops otherwise.
                // T-638 — the collapse chevron leads the row (the dock's outer top-LEFT corner, inside the
                // tab strip); « while expanded, flips to » collapsed.
                <div
                    class="group flex items-center justify-between gap-2 rounded px-1 py-0.5"
                    title="Drop a folder here to move it to the top level"
                    on:pointerup=move |ev: web_sys::PointerEvent| {
                        ev.stop_propagation();
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = crate::editor_ops::complete_layer_drop_onto_root();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                >
                    // T-696 — the title is now a two-tab strip ("Editor Layers" | "Locations"), Eden's
                    // Locations-tab-beside-Entities precedent. The chevron still leads the row.
                    <div class="flex min-w-0 items-center gap-1" role="tablist">
                        {collapse_chevron(collapsed, true)}
                        {tab_btn(LeftTab::Layers, "Editor Layers", "dock-left-tab-layers")}
                        {tab_btn(LeftTab::Places, "Locations", "dock-left-tab-places")}
                    </div>
                    // T-696 — the header's trailing verb follows the tab: create a layer on the tree,
                    // bookmark the live camera on Locations.
                    {move || {
                        if tab.get() == LeftTab::Places {
                            view! {
                                <button
                                    type="button"
                                    aria-label="Bookmark this view"
                                    data-testid="dock-left-bookmark-add"
                                    title="Bookmark this view (name the current camera position)"
                                    class="flex size-5 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        adding
                                            .set(
                                                Some(bookmarks.with_untracked(default_bookmark_name)),
                                            );
                                    }
                                >
                                    <MaterialIcon name="bookmark_add" class="block text-base" />
                                </button>
                            }
                                .into_any()
                        } else {
                            view! {
                                <button
                                    type="button"
                                    aria-label="New layer"
                                    title="New layer (child of the selected layer)"
                                    class="flex size-5 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let _ = crate::editor_ops::create_layer();
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                >
                                    <MaterialIcon name="add" class="block text-base" />
                                </button>
                            }
                                .into_any()
                        }
                    }}
                </div>
                // T-666 — a release that reaches this wrapper landed on NEITHER a folder row nor the
                // header root-dropzone (both `stop_propagation` + complete their own drop), so it is a
                // stray drag (released over empty tree space or a non-target slot row) — clear the latch
                // so a later click on a folder can't complete a stale reparent/refile.
                // T-696 — the tab body. The layers half is byte-for-byte the T-666 tree (stray-drag
                // cancel latch included); the Locations half is the new index + bookmarks panel.
                {move || {
                    if tab.get() == LeftTab::Places {
                        places_body().into_any()
                    } else {
                        view! {
                            <div
                                class="mt-1"
                                on:pointerup=move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    crate::editor_ops::cancel_layer_drag();
                                }
                            >
                                {virtual_tree(
                                    nodes,
                                    selected,
                                    active_layer,
                                    "editorLayers",
                                    "No objects placed yet.",
                                    false,
                                    // T-666 — this is the layer-authoring tree.
                                    true,
                                )}
                            </div>
                        }
                            .into_any()
                    }
                }}
                <div class="mt-auto flex items-center justify-between border-t border-outline-variant/20 pt-2">
                    {strip_btn("account_tree", "Hierarchy (visual only)", true)}
                    {strip_btn("layers", "Layers (visual only)", false)}
                    {strip_btn("inventory_2", "Assets (visual only)", false)}
                    {strip_btn("history", "History (visual only)", false)}
                    {strip_btn("settings", "Settings (visual only)", false)}
                </div>
            </aside>
        }
    };
    // T-638 — collapsed: render ONLY the 24×24 stub (the expand chevron) at the outer top-left corner,
    // overlaying the map. Its wrapper in `mission_editor` shrinks to STUB_PX so the freed area is
    // click-through to the map. The full outliner is unmounted while collapsed.
    let stub = move || {
        view! {
            <div
                class="pointer-events-auto flex items-start bg-surface-container-lowest/55 backdrop-blur-xl"
                style=format!("width:{STUB_PX}px;height:{STUB_PX}px")
            >
                {collapse_chevron(collapsed, true)}
            </div>
        }
    };
    // T-638 — swap the whole dock for the corner stub while collapsed.
    move || {
        if collapsed.get() {
            stub().into_any()
        } else {
            full().into_any()
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// T-696 — MAP BOOKMARKS + THE NAMED-LOCATIONS INDEX (NEW-F7 / 3den E1)
// ═════════════════════════════════════════════════════════════════════════════════════════════════
//
// A 12.8 km map with zero bookmarks and no way to list — let alone reach — the places it already
// draws by name. Two halves, one tab, one camera mover.
//
// **THE INDEX IS NOT NEW DATA.** The town names come from `/map-assets/<terrain>/locations.json`,
// parsed by `map_engine_core::world::parse_locations_json` — the SAME file and the SAME parser
// `world_assets::labels::LabelHost::init` reads to feed the Town-labels lane. Nothing here re-sources
// or re-authors a location. It cannot read `LabelHost`'s already-parsed `towns` directly because
// `world_assets` declares `mod labels` PRIVATE and `towns` is a private field, and neither file is
// this slice's to touch — so the honest in-owns move is to re-run the shared parser over the shared
// URL (the browser serves the second GET from cache). The proper fix is a
// `world_assets::named_locations()` accessor over `LabelHost`; filed in the slice report.
//
// **THE CAMERA.** Flying to a place must not be a SECOND camera mover. The one mover is
// `RenderEngine::set_view` followed by `on_camera_changed` and a viewport flush — the path
// `editor_ops::center_on_selection` (Space), the validation panel's finding jump, and the
// initial view all take. That path is reachable from this file only through the closure
// `mission_editor::register_editor_cam` installs on `window.__editorCamSet` in the SAME block as
// `world_assets::register_render_ctx` (unconditional, every wasm build, the moment the engine
// exists) — `world_assets` exposes a camera READER (`camera_snapshot`) and no writer, and adding
// one would mean editing `world_assets/mod.rs` or `mission_editor.rs`, neither of which is in this
// slice's owns. [`fly_to`] therefore calls that closure: same `set_view`, same `on_camera_changed`,
// same flush, zero duplicated camera math. Filed in the slice report as the seam to promote.
//
// **UNDO.** Neither half is undoable, deliberately, and both are kept out of the document: a camera
// move and a bookmark are session/overlay state, not authored content — exactly the line T-642 drew
// for rulers. Nothing in this file reaches the local-edit history tail, and a source pin below holds
// exactly that.
//
// **BOOKMARKS STORE ZOOM.** A bookmark is `(name, x, y, zoom)` and restoring one restores all three;
// that is what makes it a saved VIEW rather than a saved point. A named LOCATION carries no authored
// zoom, so flying to one keeps the current zoom (`center_on_selection`'s rule).
//
// **KEY NAMESPACE + VERSION.** `tbd-<area>-<thing>` holding a JSON blob with an integer `version`,
// defaults-on-parse-failure, one migration chokepoint — the convention `world_layer_prefs`
// (`tbd-mc-editor-prefs`), `auth` (`tbd-auth`), `editor_session` (`tbd-editor-session`) and T-695
// (`tbd-mc-editor-favourites`) already follow. [`BOOKMARKS_KEY`] is the third editor-local key where
// the T-691 design says one; `world_layer_prefs::EditorPrefs` is the seam it should be a field on,
// and that file is not in this slice's owns either. Reported, not silently repeated a fourth time.

/// T-696 — which half of the left dock is showing. Eden puts Locations beside Entities; TBD puts it
/// beside the Editor Layers tree, which is this dock's Entities equivalent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeftTab {
    /// The T-666 layer-authoring outliner (the dock's original and default content).
    Layers,
    /// The T-696 named-locations index + map bookmarks.
    Places,
}

/// T-696 — one row of the named-locations index, projected out of
/// `map_engine_core::world::LocationLabel` so the list/filter logic compiles (and is tested) on the
/// native build, where map-engine-core's `world` feature is off.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedPlace {
    pub name: String,
    pub x: f64,
    pub y: f64,
    /// The source row's `kind` (`"town"`, `"village"`, …), empty when the source omits it.
    pub kind: String,
}

/// T-696 — the shared filter predicate: case-insensitive substring, empty query matches everything.
/// One predicate for both lists so the bookmark half and the location half can never disagree about
/// what the filter box means.
#[must_use]
pub fn matches_query(name: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || name.to_lowercase().contains(&q)
}

/// T-696 — the named-locations index, filtered. Order is the caller's (see [`sort_places`]).
#[must_use]
pub fn filter_places(places: &[NamedPlace], query: &str) -> Vec<NamedPlace> {
    places
        .iter()
        .filter(|p| matches_query(&p.name, query))
        .cloned()
        .collect()
}

/// T-696 — the index's display order: alphabetical, case-insensitive. An index exists to be scanned
/// by eye, so it is sorted by NAME, not by the source file's order or by importance (importance
/// drives the map's declutter, not a list a human reads).
pub fn sort_places(places: &mut [NamedPlace]) {
    places.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// T-696 — the one localStorage key the bookmark collection persists under.
const BOOKMARKS_KEY: &str = "tbd-mc-editor-bookmarks";
/// T-696 — the persisted blob's schema version. Bump when a field's shape changes in a way a raw
/// serde load of an older blob cannot absorb (adding a `#[serde(default)]` field does NOT need a
/// bump); [`migrate_bookmarks`] then owns the upgrade.
const BOOKMARKS_VERSION: u32 = 1;
/// T-696 — how many bookmarks the collection keeps. localStorage is a shared, small, synchronously
/// parsed budget and nothing else bounds an add loop; 200 named views on one 12.8 km map is far past
/// any plausible working set.
const BOOKMARKS_MAX: usize = 200;

/// T-696 — one saved camera position: a name plus the full view (`x`/`y` in world metres, `zoom` in
/// deck zoom — the exact triple `RenderEngine::set_view` takes and `camera_snapshot` returns).
///
/// The NAME is the identity. There is no separate id: bookmarks are operator-authored and few, a
/// duplicate name would make the rename/remove buttons ambiguous, and a synthetic id would have to
/// be generated, persisted and kept unique for no benefit the operator can see.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

/// T-696 — the persisted bookmarks blob: a version plus the saved views, newest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bookmarks {
    /// Schema version of the persisted blob (see [`BOOKMARKS_VERSION`]).
    #[serde(default)]
    pub version: u32,
    /// The saved views in display order — most recently added first.
    #[serde(default)]
    pub items: Vec<Bookmark>,
}

impl Default for Bookmarks {
    fn default() -> Self {
        Self {
            version: BOOKMARKS_VERSION,
            items: Vec::new(),
        }
    }
}

/// The identity key for a bookmark name: trimmed and case-folded, so "Levie" and " levie " are one
/// bookmark rather than two rows whose remove buttons look identical.
fn bookmark_key(name: &str) -> String {
    name.trim().to_lowercase()
}

impl Bookmarks {
    /// Parse a persisted blob, falling back to empty on any serde failure and normalising through
    /// [`migrate_bookmarks`]. Pure — no localStorage — so the whole storage contract is testable on
    /// the native build.
    #[must_use]
    fn from_json(raw: &str) -> Self {
        migrate_bookmarks(serde_json::from_str::<Self>(raw).unwrap_or_default())
    }

    /// Serialize for persistence (empty string only if serde itself fails, which the round-trip test
    /// precludes for this shape).
    #[must_use]
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        let k = bookmark_key(name);
        self.items.iter().any(|b| bookmark_key(&b.name) == k)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The ADD verb — save the given camera position under `name`. Newest first, so the view just
    /// saved is the one the operator sees. Returns whether anything was added: an empty name and a
    /// duplicate name are both refused rather than stored (a nameless bookmark can never be found
    /// again, and a duplicate would make the row actions ambiguous).
    pub fn add(&mut self, name: &str, x: f64, y: f64, zoom: f64) -> bool {
        let name = name.trim();
        if name.is_empty() || self.contains(name) {
            return false;
        }
        self.items.insert(
            0,
            Bookmark {
                name: name.to_string(),
                x,
                y,
                zoom,
            },
        );
        self.items.truncate(BOOKMARKS_MAX);
        true
    }

    /// The RENAME verb. Refuses an empty target and a collision with a DIFFERENT bookmark; renaming
    /// a bookmark to its own name (or to a different casing of it) is allowed and just rewrites the
    /// label. Returns whether the collection changed.
    pub fn rename(&mut self, from: &str, to: &str) -> bool {
        let to = to.trim();
        if to.is_empty() {
            return false;
        }
        let from_key = bookmark_key(from);
        let to_key = bookmark_key(to);
        if to_key != from_key && self.contains(to) {
            return false;
        }
        for b in &mut self.items {
            if bookmark_key(&b.name) == from_key {
                if b.name == to {
                    return false;
                }
                b.name = to.to_string();
                return true;
            }
        }
        false
    }

    /// The REMOVE verb. Idempotent — removing an absent bookmark is a no-op.
    pub fn remove(&mut self, name: &str) {
        let k = bookmark_key(name);
        self.items.retain(|b| bookmark_key(&b.name) != k);
    }
}

/// T-696 — bring a freshly-loaded blob up to the current version and normalise it. Idempotent.
///
/// Beyond the version stamp this is the integrity floor for a blob another tab (or a person with
/// devtools) may have written: unnamed entries are dropped, duplicate names collapse to their first
/// occurrence, non-finite coordinates are dropped (a `NaN` centre would send the camera nowhere
/// recoverable), and the list is capped.
fn migrate_bookmarks(mut bm: Bookmarks) -> Bookmarks {
    if bm.version < BOOKMARKS_VERSION {
        // No field-shape migrations exist yet (v0 → v1 is field-compatible via serde defaults);
        // future versions add their transforms here, gated on the incoming `version`.
        bm.version = BOOKMARKS_VERSION;
    }
    let mut seen = std::collections::HashSet::new();
    bm.items.retain(|b| {
        !b.name.trim().is_empty()
            && b.x.is_finite()
            && b.y.is_finite()
            && b.zoom.is_finite()
            && seen.insert(bookmark_key(&b.name))
    });
    bm.items.truncate(BOOKMARKS_MAX);
    bm
}

/// T-696 — the bookmark list, filtered by the shared query.
#[must_use]
pub fn filter_bookmarks(bm: &Bookmarks, query: &str) -> Vec<Bookmark> {
    bm.items
        .iter()
        .filter(|b| matches_query(&b.name, query))
        .cloned()
        .collect()
}

/// T-696 — the name the "bookmark this view" input is seeded with: the first free `View <n>`. Seeded
/// rather than blank so the fastest path (click, Enter) still produces a distinct, findable name.
#[must_use]
pub fn default_bookmark_name(bm: &Bookmarks) -> String {
    (1..=BOOKMARKS_MAX + 1)
        .map(|n| format!("View {n}"))
        .find(|n| !bm.contains(n))
        .unwrap_or_else(|| String::from("View"))
}

#[cfg(target_arch = "wasm32")]
fn bookmarks_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// T-696 — load the bookmark collection. Off wasm (the native test build) this is always empty,
/// exactly like `world_layer_prefs::load_store` and `eden_dock_right::load_favourites`.
#[must_use]
pub fn load_bookmarks() -> Bookmarks {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = bookmarks_storage() {
            if let Ok(Some(raw)) = s.get_item(BOOKMARKS_KEY) {
                return Bookmarks::from_json(&raw);
            }
        }
    }
    Bookmarks::default()
}

/// T-696 — persist the bookmark collection (no-op off wasm). The version is stamped current on write
/// so a load never sees a stale version this build wrote itself.
pub fn save_bookmarks(bm: &Bookmarks) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = bookmarks_storage() {
            let mut out = bm.clone();
            out.version = BOOKMARKS_VERSION;
            let _ = s.set_item(BOOKMARKS_KEY, &out.to_json());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = bm;
}

/// T-696 — the live camera `(x, y, zoom)` a new bookmark records, read through the same
/// `world_assets::camera_snapshot` seam the scale bar and grid-reference overlay use. `None` before
/// the engine mounts (and always on the native build), in which case the add verb declines rather
/// than saving a bookmark pointing at nothing.
#[must_use]
pub fn live_camera() -> Option<(f64, f64, f64)> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::world_assets::camera_snapshot()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// T-696 — FLY TO `(x, y)`, optionally at `zoom` (a bookmark carries one; a named location does not,
/// and keeps the current zoom).
///
/// This is NOT a second camera mover. It calls the closure `mission_editor::register_editor_cam`
/// installs on `window.__editorCamSet`, whose body is exactly `set_view` → `on_camera_changed` →
/// `world_assets::flush_viewport` — the same sequence `editor_ops::center_on_selection` runs. See
/// the section header for why that closure is the only reachable handle from this file's owns.
/// A no-op before the engine mounts.
#[allow(unused_variables)]
pub fn fly_to(x: f64, y: f64, zoom: Option<f64>) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, JsValue};
        let Some(z) = zoom.or_else(|| live_camera().map(|(_, _, z)| z)) else {
            return; // no engine yet — nothing to fly.
        };
        let Some(win) = web_sys::window() else {
            return;
        };
        let Ok(f) = js_sys::Reflect::get(&win, &JsValue::from_str("__editorCamSet")) else {
            return;
        };
        let Ok(f) = f.dyn_into::<js_sys::Function>() else {
            return;
        };
        let _ = f.call3(
            &JsValue::NULL,
            &JsValue::from_f64(x),
            &JsValue::from_f64(y),
            &JsValue::from_f64(z),
        );
    }
}

/// T-696 — fetch + parse the named-locations index for the document's terrain, once per editor
/// session. Same URL and same parser as the town-label lane (see the section header); a missing or
/// unparseable file yields an empty index rather than an error state, because the index is an
/// accelerator and the map is still fully usable without it.
#[cfg(target_arch = "wasm32")]
async fn fetch_named_places() -> Vec<NamedPlace> {
    let terrain = crate::editor_ops::read_env().terrain;
    let url = format!("/map-assets/{terrain}/locations.json");
    let Ok(resp) = gloo_net::http::Request::get(&url).send().await else {
        return Vec::new();
    };
    if !(200..300).contains(&resp.status()) {
        return Vec::new();
    }
    let Ok(txt) = resp.text().await else {
        return Vec::new();
    };
    let Ok(rows) = map_engine_core::world::parse_locations_json(&txt) else {
        return Vec::new();
    };
    let mut out: Vec<NamedPlace> = rows
        .into_iter()
        .filter(|l| !l.name.trim().is_empty())
        .map(|l| NamedPlace {
            name: l.name,
            x: l.x,
            y: l.y,
            kind: l.kind.unwrap_or_default(),
        })
        .collect();
    sort_places(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::{
        default_bookmark_name, filter_bookmarks, filter_places, matches_query, sort_places,
        Bookmarks, LeftTab, NamedPlace, BOOKMARKS_KEY, BOOKMARKS_VERSION,
    };

    const SRC: &str = include_str!("eden_dock_left.rs");

    fn place(name: &str, x: f64, y: f64) -> NamedPlace {
        NamedPlace {
            name: name.to_string(),
            x,
            y,
            kind: "town".to_string(),
        }
    }

    /// T-696 — the storage contract: a NAMESPACED key and a VERSIONED blob, following the convention
    /// the frontend already uses (`tbd-mc-editor-prefs`, `tbd-auth`, `tbd-mc-editor-favourites`)
    /// rather than inventing one.
    ///
    /// The version is load-bearing in both directions: a fresh blob carries it on the wire (so the
    /// first shape change has something to branch on), and a blob written before the field existed
    /// (`version` absent ⇒ serde default 0) is stamped forward on load instead of being discarded.
    /// Perturbation RED: drop the stamp in `migrate_bookmarks` and the v0 assertion fails; widen the
    /// key to an un-namespaced string and the prefix assertion fails.
    #[test]
    fn bookmarks_key_is_namespaced_and_versioned() {
        assert!(
            BOOKMARKS_KEY.starts_with("tbd-"),
            "the key must carry the frontend's `tbd-` namespace, got {BOOKMARKS_KEY:?}"
        );
        assert!(
            BOOKMARKS_KEY.contains("bookmark"),
            "the key must say what it holds, got {BOOKMARKS_KEY:?}"
        );
        // It must not collide with any sibling editor-local store.
        assert_ne!(BOOKMARKS_KEY, "tbd-mc-editor-prefs");
        assert_ne!(BOOKMARKS_KEY, "tbd-mc-editor-favourites");
        assert_ne!(BOOKMARKS_KEY, "tbd-auth");
        assert_ne!(BOOKMARKS_VERSION, 0, "an unversioned blob is banned");

        let mut bm = Bookmarks::default();
        assert!(bm.add("Levie approach", 7100.0, 9300.0, 2.5));
        let raw = bm.to_json();
        assert!(
            raw.contains(&format!("\"version\":{BOOKMARKS_VERSION}")),
            "the persisted blob must carry its version, got {raw}"
        );

        // A pre-version blob (what a hand-written or older writer would leave) loads and is stamped
        // forward rather than thrown away.
        let v0 = r#"{"items":[{"name":"Levie approach","x":7100.0,"y":9300.0,"zoom":2.5}]}"#;
        let loaded = Bookmarks::from_json(v0);
        assert_eq!(loaded.version, BOOKMARKS_VERSION, "v0 blob must migrate");
        assert!(loaded.contains("Levie approach"), "v0 entry must survive");

        // Outright garbage falls back to empty rather than panicking (the defaults floor).
        assert!(Bookmarks::from_json("not json at all").is_empty());
    }

    /// T-696 — the three bookmark verbs plus the reload, and the ZOOM decision. A bookmark stores
    /// the full view, so a round-trip through the persisted string must reproduce `zoom` as well as
    /// the centre; if it did not, "fly to a bookmark" would be a pan, not a restored view.
    #[test]
    fn bookmarks_add_rename_remove_and_survive_a_reload() {
        let mut bm = Bookmarks::default();
        assert!(bm.is_empty());

        assert!(bm.add("Montignac", 4600.0, 9100.0, 1.5));
        assert!(bm.add("Levie", 7100.0, 9300.0, 3.25));
        assert_eq!(bm.len(), 2);
        // Newest first — the view just saved is the one the panel shows at the top.
        assert_eq!(bm.items[0].name, "Levie");

        // Refusals: an empty name, a whitespace-only name, and a duplicate (case-insensitively).
        assert!(!bm.add("", 1.0, 2.0, 3.0));
        assert!(!bm.add("   ", 1.0, 2.0, 3.0));
        assert!(!bm.add("levie", 1.0, 2.0, 3.0), "duplicate name refused");
        assert_eq!(bm.len(), 2);

        // The reload: persist, then load exactly what was persisted — zoom included.
        let reloaded = Bookmarks::from_json(&bm.to_json());
        assert_eq!(reloaded, bm, "a reload must reproduce the collection");
        let levie = reloaded
            .items
            .iter()
            .find(|b| b.name == "Levie")
            .expect("Levie survives");
        assert!(
            (levie.zoom - 3.25).abs() < 1e-9,
            "a bookmark stores ZOOM as well as centre, got {}",
            levie.zoom
        );
        assert!((levie.x - 7100.0).abs() < 1e-9 && (levie.y - 9300.0).abs() < 1e-9);

        // RENAME.
        let mut bm = reloaded;
        assert!(bm.rename("Levie", "Levie ridge"));
        assert!(bm.contains("Levie ridge") && !bm.contains("Levie"));
        assert!(
            !bm.rename("Levie ridge", "montignac"),
            "renaming onto another bookmark's name is refused"
        );
        assert!(!bm.rename("Levie ridge", "   "), "empty rename refused");
        assert!(
            !bm.rename("nothing here", "x"),
            "renaming an absent bookmark"
        );

        // REMOVE, and it is idempotent.
        bm.remove("Levie ridge");
        assert_eq!(bm.len(), 1);
        bm.remove("Levie ridge");
        assert_eq!(bm.len(), 1, "removing an absent bookmark is a no-op");
    }

    /// T-696 — the migration chokepoint is also the integrity floor for a blob written by another
    /// tab or by hand: duplicates collapse, unnamed rows go, and a non-finite coordinate (which
    /// would fly the camera to nowhere recoverable) is dropped rather than restored.
    #[test]
    fn migrate_bookmarks_drops_junk_rows() {
        let raw = r#"{"version":1,"items":[
            {"name":"A","x":1.0,"y":2.0,"zoom":1.0},
            {"name":"a","x":9.0,"y":9.0,"zoom":9.0},
            {"name":"  ","x":1.0,"y":2.0,"zoom":1.0},
            {"name":"NaN centre","x":null,"y":2.0,"zoom":1.0}
        ]}"#;
        // The `null` row fails serde for the whole blob, so prove the finite/dup/empty floor on a
        // blob serde CAN read, and the hard-failure floor separately.
        assert!(
            Bookmarks::from_json(raw).is_empty(),
            "a blob serde cannot read falls back to empty"
        );

        let ok = r#"{"version":1,"items":[
            {"name":"A","x":1.0,"y":2.0,"zoom":1.0},
            {"name":"a","x":9.0,"y":9.0,"zoom":9.0},
            {"name":"  ","x":1.0,"y":2.0,"zoom":1.0}
        ]}"#;
        let bm = Bookmarks::from_json(ok);
        assert_eq!(bm.len(), 1, "duplicate + unnamed rows are dropped: {bm:?}");
        assert_eq!(bm.items[0].name, "A", "the FIRST occurrence wins");
    }

    /// T-696 — `default_bookmark_name` seeds the next free slot, so click-Enter twice yields two
    /// distinct bookmarks instead of a refused duplicate.
    #[test]
    fn default_bookmark_name_finds_the_next_free_slot() {
        let mut bm = Bookmarks::default();
        assert_eq!(default_bookmark_name(&bm), "View 1");
        let n = default_bookmark_name(&bm);
        assert!(bm.add(&n, 0.0, 0.0, 1.0));
        assert_eq!(default_bookmark_name(&bm), "View 2");
        assert!(bm.add("View 2", 0.0, 0.0, 1.0));
        assert_eq!(default_bookmark_name(&bm), "View 3");
    }

    /// T-696 — ONE filter predicate over BOTH lists. The index is what makes the 12.8 km map
    /// navigable, and it is only navigable if the filter is case-insensitive, substring (not
    /// prefix), and empty-means-everything.
    #[test]
    fn one_filter_predicate_serves_both_lists() {
        assert!(matches_query("Montignac", ""), "empty query matches all");
        assert!(matches_query("Montignac", "   "), "blank query matches all");
        assert!(matches_query("Montignac", "montignac"), "case-insensitive");
        assert!(matches_query("Montignac", "TIGN"), "substring, not prefix");
        assert!(!matches_query("Montignac", "levie"));

        let places = vec![
            place("Montignac", 4600.0, 9100.0),
            place("Levie", 7100.0, 9300.0),
        ];
        assert_eq!(filter_places(&places, "lev").len(), 1);
        assert_eq!(filter_places(&places, "").len(), 2);
        assert!(filter_places(&places, "zzz").is_empty());

        let mut bm = Bookmarks::default();
        assert!(bm.add("Montignac", 1.0, 2.0, 3.0));
        assert!(bm.add("Levie", 4.0, 5.0, 6.0));
        assert_eq!(filter_bookmarks(&bm, "MONT").len(), 1);
        assert_eq!(filter_bookmarks(&bm, "").len(), 2);
    }

    /// T-696 — the index is sorted for a HUMAN scanning it: alphabetical, case-insensitive, and NOT
    /// the source file's order (which is importance/authoring order — that drives the map's
    /// declutter, not a list).
    #[test]
    fn the_index_is_sorted_case_insensitively_by_name() {
        let mut places = vec![
            place("levie", 1.0, 1.0),
            place("Montignac", 2.0, 2.0),
            place("Chotain", 3.0, 3.0),
        ];
        sort_places(&mut places);
        let names: Vec<&str> = places.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["Chotain", "levie", "Montignac"]);
    }

    /// T-696 — the two tabs are distinct and the dock defaults to the layers tree (the Locations tab
    /// is additive; it must not displace what the dock was).
    #[test]
    fn the_dock_has_two_tabs_and_defaults_to_layers() {
        assert_ne!(LeftTab::Layers, LeftTab::Places);
        assert!(
            SRC.contains("let tab = RwSignal::new(LeftTab::Layers)"),
            "the dock must still open on the Editor Layers tree"
        );
        for needle in [
            "dock-left-tab-layers",
            "dock-left-tab-places",
            "dock-left-bookmark-add",
            "dock-left-bookmark-name",
            "dock-left-bookmark-rename",
            "dock-left-bookmark-list",
            "dock-left-location-list",
            "dock-left-places-filter",
        ] {
            assert!(SRC.contains(needle), "missing driveable testid {needle}");
        }
    }

    /// T-696 — source pins for the wasm-only halves, in the `eden_tree::source_pins` idiom: the
    /// index reads the SHIPPED source (same URL, same parser as the town-label lane) and the fly-to
    /// rides the SHIPPED camera path rather than a second mover.
    #[test]
    fn the_index_and_the_fly_to_reuse_the_shipped_paths() {
        assert!(
            SRC.contains("parse_locations_json"),
            "the index must reuse map-engine-core's locations parser, not a private one"
        );
        assert!(
            SRC.contains("/map-assets/{terrain}/locations.json"),
            "the index must read the same file the town-label lane reads"
        );
        assert!(
            SRC.contains("__editorCamSet"),
            "fly-to must ride the installed set_view/on_camera_changed closure"
        );
        // Split so the needle itself is not a hit (the personnel.rs `production_src` idiom).
        let second_mover = format!("{}{}", "set_view", "(");
        assert!(
            !SRC.contains(&second_mover),
            "there must be no SECOND camera mover in this dock"
        );
        assert!(
            SRC.contains("camera_snapshot"),
            "a new bookmark must record the LIVE camera, not a guess"
        );
    }

    /// T-696 — camera state is NOT authored content (T-642's ruler rule). Neither flying to a place
    /// nor any bookmark verb may enter the undo stack or touch the document, so this file must never
    /// reach the history/edit tail. Perturbation RED: add an `after_local_edit()` call and this
    /// fails.
    #[test]
    fn bookmarks_and_fly_to_are_not_document_edits() {
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("the production half precedes the test module");
        for banned in [
            "mission_history::",
            "after_local_edit",
            "add_slot",
            "remove_slots",
        ] {
            assert!(
                !production.contains(banned),
                "camera/bookmark state must not be a document edit, found {banned}"
            );
        }
    }
}
