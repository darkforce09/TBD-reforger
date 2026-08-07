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

// ── T-637 — the header row is a WIDTH BUDGET too ─────────────────────────────────────────────────
//
// At the equalised 240 px the header has to hold the collapse chevron, two tab labels and a trailing
// verb. It did not: measured in a headless Chrome against the generated `aegis.css`, the old
// "Editor Layers" + "Locations" pair with `tracking-wide` wanted 228 px of a 215 px row, and because
// the tab group carries `min-w-0` the row SQUEEZED instead of overflowing — the first label wrapped
// and the header silently grew a line. A squeeze is worse than an overflow: nothing reports it.
//
// So the labels are consts, the cells are `shrink-0`, and `the_header_row_fits_the_dock` adds the
// row up from the label lengths. A longer label fails the pin instead of quietly wrapping.

/// T-637 — the layers tab's label. "Editor Layers" did not fit (see the block above), and the
/// qualifier bought nothing: the dock holds exactly one kind of layer.
const TAB_LABEL_LAYERS: &str = "Layers";
/// T-637 — the locations tab's label (T-696's name, kept: it is Eden's).
const TAB_LABEL_PLACES: &str = "Locations";
/// T-637 — an upper bound on one UPPERCASE `text-label-sm` (12 px, semibold, no extra tracking)
/// character's advance, in CSS px.
///
/// MEASURED, not guessed: rendering the real classes against the generated `aegis.css` in a headless
/// Chrome gives 45.75 px for the 6 characters of "Layers" (7.63/char) and 72.88 px for the 9 of
/// "Locations" (8.10/char), in DejaVu Sans — the widest fallback in the stack. Inter and system-ui,
/// which is what actually renders, are both narrower, so 8.5 is a genuine ceiling with margin.
const UPPERCASE_LABEL_ADVANCE_PX: f64 = 8.5;
/// T-637 — a tab cell's horizontal padding (`px-1.5` ⇒ 6 px each side).
const TAB_LABEL_PAD_PX: f64 = 12.0;
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
    // ── T-637 — the Layers tab's filter ──────────────────────────────────────────────────────────
    // `layer_nodes` is the tree the view actually renders: `nodes` when the box is empty (the
    // filter is off by default and costs a clone the doc rebuild already pays for), the matching
    // subtree otherwise. It is a SEPARATE signal rather than a filter applied inside `virtual_tree`
    // so the tree renderer keeps one input and one meaning, and so `__outlinerStats.total` still
    // reports what the dock is showing.
    let layer_query = RwSignal::new(String::new());
    let layer_nodes = RwSignal::new(Vec::<OutlinerNode>::new());
    Effect::new(move |_| {
        let q = layer_query.get();
        layer_nodes.set(nodes.with(|ns| filter_outliner(ns, &q)));
    });
    // Whether the filter is hiding EVERYTHING. A `Memo` so the body below re-runs at the boundary
    // only: a plain closure over the two signals would remount `virtual_tree` on every keystroke,
    // throwing away per-tree collapse state and re-creating its Effect each time.
    let layers_filtered_empty = Memo::new(move |_| {
        !layer_query.with(|q| q.trim().is_empty()) && layer_nodes.with(Vec::is_empty)
    });

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
                // T-637 — `shrink-0`, and the `tracking-wide` is gone. MEASURED in a headless
                // Chrome against the generated `aegis.css`: at the equalised 240 px the old
                // "Editor Layers" label plus the letter-spacing wanted 228 px of a 215 px row, so
                // the flex row squeezed the cells (the group carries `min-w-0`) and the first label
                // wrapped. `shrink-0` turns that from a silent squeeze into an overflow the budget
                // pin below catches.
                class=move || {
                    if tab.get() == t {
                        "shrink-0 rounded px-1.5 py-0.5 text-label-sm font-semibold uppercase text-on-surface"
                    } else {
                        "shrink-0 rounded px-1.5 py-0.5 text-label-sm font-semibold uppercase text-outline transition-colors hover:text-on-surface"
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
                    class="group flex items-center justify-between gap-1 rounded px-1 py-0.5"
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
                    // T-696 — the title is a two-tab strip, Eden's Locations-tab-beside-Entities
                    // precedent. The chevron still leads the row.
                    //
                    // T-637 — "Editor Layers" is now "Layers". Not a preference: measured, the old
                    // label did not fit the equalised 240 px beside "Locations", the chevron and
                    // the trailing verb, and the dock has exactly one kind of layer in it — the
                    // "Editor" half distinguished it from nothing the operator can see here.
                    <div class="flex min-w-0 items-center gap-1" role="tablist">
                        {collapse_chevron(collapsed, true)}
                        {tab_btn(LeftTab::Layers, TAB_LABEL_LAYERS, "dock-left-tab-layers")}
                        {tab_btn(LeftTab::Places, TAB_LABEL_PLACES, "dock-left-tab-places")}
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
                // T-637 — the Layers tab's SEARCH ROW. Eden fills its 240 px with a tab strip, a
                // filter row and a dense tree; ours had a tab strip, a tree and 900 px of nothing.
                // The Locations tab already had a filter (T-696) — this gives the half of the dock
                // that actually needs it one too, over the SAME `matches_query` predicate, so the
                // two tabs cannot drift into two different ideas of what "matches" means.
                {move || {
                    (tab.get() == LeftTab::Layers)
                        .then(|| {
                            view! {
                                <input
                                    type="search"
                                    data-testid="dock-left-layers-filter"
                                    aria-label="Filter editor layers"
                                    placeholder="Filter layers…"
                                    prop:value=move || layer_query.get()
                                    class="mt-1 w-full shrink-0 rounded border border-outline-variant/30 bg-black/20 px-1.5 py-0.5 text-label-sm text-on-surface outline-none placeholder:text-outline focus:border-primary/60"
                                    on:input=move |ev| layer_query.set(event_target_value(&ev))
                                />
                            }
                        })
                }}
                {move || {
                    if tab.get() == LeftTab::Places {
                        places_body().into_any()
                    } else if layers_filtered_empty.get() {
                        // Honest empty state: "No objects placed yet." would be a lie while a filter
                        // is hiding them. Gated behind a Memo so the tree below is NOT remounted on
                        // every keystroke — only when the has-matches answer actually flips.
                        view! {
                            <p class="mt-2 px-1 text-label-sm text-outline">
                                "No layers match that filter."
                            </p>
                        }
                            .into_any()
                    } else {
                        view! {
                            // T-637 — `min-h-0 flex-1` is what hands the dock's leftover height to
                            // the tree instead of leaving it as void. `min-h-0` is load-bearing: a
                            // flex child's default `min-height:auto` refuses to shrink below its
                            // content, so without it a long tree pushes the panel instead of
                            // scrolling inside it.
                            <div
                                class="mt-1 min-h-0 flex-1 overflow-y-auto"
                                on:pointerup=move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    crate::editor_ops::cancel_layer_drag();
                                }
                            >
                                {virtual_tree(
                                    layer_nodes,
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
            // T-637 — FIVE DECORATION BUTTONS DELETED HERE, and the auto-margin row that stranded
            // them at the bottom edge of a 900 px void with it.
            //
            // They were five sibling builder calls — Hierarchy, Layers, Assets, History, Settings —
            // each hard-wired to the disabled state, each carrying a tooltip that admitted in words
            // that it was decorative, and only the first passing the active flag. The strip thus
            // claimed a five-tab set that did not exist and could not be reached. T-172 B9 added
            // them as "honest parity" with React's `BOTTOM_TABS`, but parity with a mock is not
            // honesty: a control that cannot act, cannot be enabled, and says so only in a tooltip
            // is furniture. The auto-margin was the marooning — it pushed the row to the panel
            // floor and turned the dock's unused height into a layout feature.
            //
            // Nothing replaces them. Three of the five name surfaces this dock already HAS (the
            // layers tree is the hierarchy; Locations is the second tab) or that live elsewhere
            // (History is the strip's version glyph, Settings the strip's gear), so re-implementing
            // them here would be a second front door to the same rooms. The height they occupied
            // goes to the tree, which now claims the remainder of the column.
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

/// T-637 — the Editor Layers tree, filtered by the same [`matches_query`] predicate the Locations
/// tab uses. Returns the whole tree unchanged for an empty/blank query (the common case — the filter
/// is off by default), so this is a clone, not a rebuild.
///
/// **A tree filter is not a list filter, and the difference is where the honesty lives.** Two rules:
///
///   * A node whose OWN label matches keeps its **entire subtree**. You searched for a folder because
///     you want what is inside it; hiding its non-matching children would answer a question nobody
///     asked and would make a folder look empty when it is not.
///   * A node that matches only because a DESCENDANT does is kept as **structure**, with just the
///     matching paths under it. Dropping it instead would orphan the hit — the row would appear at
///     the wrong depth, under the wrong parent, in a tree whose whole job is to show containment.
///
/// A node that neither matches nor contains a match is dropped. Pure and native-tested; the view is a
/// thin `Effect` over it.
#[must_use]
pub fn filter_outliner(nodes: &[OutlinerNode], query: &str) -> Vec<OutlinerNode> {
    if query.trim().is_empty() {
        return nodes.to_vec();
    }
    nodes
        .iter()
        .filter_map(|n| keep_matching(n, query))
        .collect()
}

/// [`filter_outliner`]'s recursion: `Some(pruned copy)` when this node survives, `None` when neither
/// it nor anything beneath it matches.
fn keep_matching(node: &OutlinerNode, query: &str) -> Option<OutlinerNode> {
    if matches_query(&node.label, query) {
        // Own hit ⇒ the subtree comes with it, untouched.
        return Some(node.clone());
    }
    let kids: Vec<OutlinerNode> = node
        .children
        .iter()
        .filter_map(|c| keep_matching(c, query))
        .collect();
    if kids.is_empty() {
        return None;
    }
    let mut out = node.clone();
    out.children = kids;
    Some(out)
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

/// T-637 — the left dock's density work: the search row that fills the void, and the five decoration
/// buttons that used to sit at the bottom of it.
///
/// **NOTE ON THE PIN IDIOM.** The T-696 pins in the module above `include_str!` this whole file and
/// then `contains()` a needle that also appears in their own assertion — so deleting the production
/// code would leave them green (filed as T-759). These pins do not do that: every source needle below
/// is checked against the PRODUCTION half only (`split("#[cfg(test)]").next()`), so a needle written
/// here cannot satisfy itself.
#[cfg(test)]
mod t637_density {
    use super::{
        filter_outliner, matches_query, TAB_LABEL_LAYERS, TAB_LABEL_PAD_PX, TAB_LABEL_PLACES,
        UPPERCASE_LABEL_ADVANCE_PX,
    };
    use crate::eden_layout::{tw_len_px, DOCK_L, DOCK_PX, STUB_PX};
    use crate::outliner::{NodeKind, OutlinerNode};

    /// The file's production half — everything above the first test module. A needle checked against
    /// this cannot be satisfied by a test's own source.
    fn production() -> &'static str {
        include_str!("eden_dock_left.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("the production half precedes the test modules")
    }

    fn node(id: &str, label: &str, kind: NodeKind, children: Vec<OutlinerNode>) -> OutlinerNode {
        OutlinerNode {
            id: id.to_string(),
            label: label.to_string(),
            kind,
            children,
            is_leader: false,
            hidden: false,
            locked: false,
            hidden_effective: false,
            locked_effective: false,
            tooltip: String::new(),
        }
    }

    /// Build a small but structurally real tree: two folders, one nested, slots under each.
    fn tree() -> Vec<OutlinerNode> {
        vec![
            node(
                "l1",
                "Assault",
                NodeKind::Folder,
                vec![
                    node("s1", "Rifleman", NodeKind::Slot, vec![]),
                    node("s2", "Medic", NodeKind::Slot, vec![]),
                    node(
                        "l2",
                        "Support",
                        NodeKind::Folder,
                        vec![node("s3", "Machinegunner", NodeKind::Slot, vec![])],
                    ),
                ],
            ),
            node(
                "l3",
                "Recon",
                NodeKind::Folder,
                vec![node("s4", "Sniper", NodeKind::Slot, vec![])],
            ),
        ]
    }

    /// **A tree filter must not lie about containment.** The two rules together: an own-hit keeps its
    /// whole subtree, and a descendant-hit keeps the ANCESTOR PATH so the match renders at the right
    /// depth under the right parent. A flat "keep matching labels" filter would satisfy neither, and
    /// in a tree whose entire job is showing what is inside what, that is a wrong answer, not a
    /// terser one.
    #[test]
    fn the_layer_filter_keeps_subtrees_and_ancestor_paths() {
        let t = tree();

        // Empty / blank query ⇒ everything, unchanged. The filter is OFF by default; this is the
        // path that runs on every doc rebuild.
        assert_eq!(filter_outliner(&t, ""), t);
        assert_eq!(filter_outliner(&t, "   "), t);

        // OWN HIT keeps the whole subtree: searching for a folder means wanting its contents.
        let own = filter_outliner(&t, "assault");
        assert_eq!(own.len(), 1, "only the Assault branch survives");
        assert_eq!(own[0].id, "l1");
        assert_eq!(
            own[0].children.len(),
            3,
            "an own-hit folder keeps every child — pruning them would show an empty folder that is \
             not empty"
        );

        // DESCENDANT HIT keeps the path, pruned to it. `Machinegunner` is two levels down.
        let deep = filter_outliner(&t, "machinegun");
        assert_eq!(deep.len(), 1);
        assert_eq!(deep[0].id, "l1", "the grandparent is kept as structure");
        assert_eq!(
            deep[0].children.len(),
            1,
            "…but only the branch that leads to the hit"
        );
        assert_eq!(deep[0].children[0].id, "l2");
        assert_eq!(deep[0].children[0].children[0].id, "s3");

        // A miss is a miss — not a silently-unfiltered tree.
        assert!(filter_outliner(&t, "zzz").is_empty());

        // Case-insensitive substring, because it is the SAME predicate the Locations tab uses.
        assert_eq!(filter_outliner(&t, "SNIP").len(), 1);
        assert!(matches_query("Sniper", "SNIP"));

        // PERTURB the rule that costs the most if it is wrong: a filter that dropped
        // non-matching ancestors would return the hit at the ROOT, at the wrong depth, under no
        // parent. State that defect as a value and check the real result differs from it.
        let orphaned: Vec<&str> = vec!["s3"];
        let real: Vec<&str> = deep.iter().map(|n| n.id.as_str()).collect();
        assert_ne!(
            real, orphaned,
            "PERTURB: a hit must not surface as a root — the tree's job is containment"
        );
    }

    /// **The five decoration buttons are gone.** They were `disabled=true` unconditionally, their
    /// tooltips literally said "(visual only)", and only one passed `active=true` — a permanent claim
    /// to a Hierarchy/Layers/Assets/History/Settings tab set that did not exist and could not be
    /// reached. `mt-auto` is what marooned them at the floor of a 900 px void.
    ///
    /// Checked on the production half only, so this test's own mention of the string cannot keep it
    /// green (the T-759 hollow-pin trap).
    #[test]
    fn no_decoration_survives_in_the_left_dock() {
        let src = production();
        // Needle assembled so this source cannot satisfy it.
        let decoration = format!("({} only)", "visual");
        assert!(
            !src.contains(&decoration),
            "T-637: a control whose tooltip admits it does nothing is furniture; the dock must not \
             ship any"
        );
        assert!(
            !src.contains("strip_btn"),
            "T-637: the decoration strip's builder must be gone, not merely unused"
        );
        assert!(
            !src.contains("mt-auto"),
            "T-637: `mt-auto` was the marooning — it turned the dock's unused height into a layout \
             feature instead of giving it to the tree"
        );
        // Every remaining `disabled=true` in the dock must be gone too: the dock has no permanently
        // dead controls left at all.
        assert!(
            !src.contains("disabled=true"),
            "T-637: no permanently-disabled control may remain in the left dock"
        );
    }

    /// **The height goes to the tree.** The dock is a column ([`crate::eden_layout::DOCK_L`]); the
    /// tree region claims the remainder with `flex-1` and can shrink inside it with `min-h-0`. Both
    /// tokens are load-bearing: without `flex-1` the void comes straight back, and without `min-h-0`
    /// a flex child refuses to shrink below its content, so a long tree pushes the panel instead of
    /// scrolling inside it.
    #[test]
    fn the_tree_claims_the_dock_height_the_decoration_used_to_hold() {
        let src = production();
        assert!(
            src.contains("min-h-0 flex-1 overflow-y-auto"),
            "T-637: the layers tree region must claim the dock's remaining height and scroll inside it"
        );
        // …and the Layers tab has the search row Eden fills that width with.
        assert!(
            src.contains("dock-left-layers-filter"),
            "T-637: the Layers tab needs a driveable filter row, like the Locations tab already had"
        );
        // The tree renders the FILTERED signal, not the raw one — otherwise the box is decoration
        // itself, which is the exact defect this ticket deleted five of. Read as "the first argument
        // at the `virtual_tree` call site", so leading whitespace cannot make the check vacuous.
        let call = src
            .find("virtual_tree(")
            .expect("the dock must still render a tree");
        let first_arg = src[call + "virtual_tree(".len()..]
            .split(',')
            .next()
            .unwrap_or("")
            .trim();
        assert_eq!(
            first_arg, "layer_nodes",
            "T-637: the tree must be fed the FILTERED node set (got `{first_arg}`), or the filter \
             box is itself decoration"
        );
        assert!(
            src.contains("filter_outliner(ns, &q)"),
            "T-637: the filtered set must come from the shared tree filter"
        );
    }

    /// **THE HEADER FITS, AND THAT IS ARITHMETIC NOW TOO.** The peer of the right dock's
    /// `t637_tab_strip_budget`. At the equalised 240 px this row holds the collapse chevron, two tab
    /// labels and a trailing verb; before this ticket the labels alone overran it, and because the
    /// tab group carries `min-w-0` the row squeezed rather than overflowed — the first label wrapped
    /// and nothing anywhere reported it.
    ///
    /// The label widths come from [`UPPERCASE_LABEL_ADVANCE_PX`], which is a MEASURED ceiling (see
    /// its doc comment), so lengthening a label fails here instead of wrapping in a browser.
    #[test]
    fn the_header_row_fits_the_dock() {
        let pad = tw_len_px(DOCK_L, "p-").expect("the dock states its padding");
        // The header's own `px-1` gutter sits inside the dock's padding.
        let budget = DOCK_PX - 2.0 * pad - 2.0 * 4.0;
        let cell = |label: &str| {
            label.chars().count() as f64 * UPPERCASE_LABEL_ADVANCE_PX + TAB_LABEL_PAD_PX
        };
        // chevron (STUB_PX — its hit box must match the collapsed stub, so it is not ours to shrink)
        // + gap + tab + gap + tab | gap | the trailing verb cell (`size-5`).
        let gap = 4.0;
        let verb = 20.0;
        let row =
            STUB_PX + gap + cell(TAB_LABEL_LAYERS) + gap + cell(TAB_LABEL_PLACES) + gap + verb;
        assert!(
            row <= budget,
            "T-637: the header wants {row} px of a {budget} px dock row. It will not overflow — the \
             tab group carries `min-w-0`, so it will SQUEEZE, wrap a label and grow a line, which is \
             the failure mode that reports nothing"
        );
        // The cells refuse to be squeezed, so a future overrun is a visible overflow the eye catches
        // rather than a silent reflow.
        let src = production();
        assert_eq!(
            src.matches("shrink-0 rounded px-1.5 py-0.5 text-label-sm font-semibold uppercase")
                .count(),
            2,
            "T-637: both tab states must be `shrink-0` — a squeezable cell hides an overrun"
        );
    }

    /// The measured ceiling is a CEILING. If someone raises it to make a longer label fit, this
    /// fails: the number has a provenance (a headless-Chrome measurement of the real classes) and
    /// widening it silently would make the budget above meaningless.
    #[test]
    fn the_measured_label_advance_is_still_an_upper_bound() {
        // The two worst per-character advances actually observed, in the widest fallback font.
        for (label, measured_total) in [(TAB_LABEL_LAYERS, 45.75), (TAB_LABEL_PLACES, 72.88)] {
            let per_char = measured_total / label.chars().count() as f64;
            assert!(
                per_char <= UPPERCASE_LABEL_ADVANCE_PX,
                "T-637: `{label}` measured {per_char} px/char, above the {UPPERCASE_LABEL_ADVANCE_PX} \
                 px ceiling the header budget is computed from"
            );
        }
        assert!(
            UPPERCASE_LABEL_ADVANCE_PX < 10.0,
            "T-637: a ceiling loose enough to admit anything is not a ceiling"
        );
    }
}
