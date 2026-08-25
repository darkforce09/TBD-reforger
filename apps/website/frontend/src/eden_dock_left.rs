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
use crate::core::ui::MaterialIcon;
use crate::eden_tree::virtual_tree;
use crate::outliner::OutlinerNode;

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

    // ── T-697 — the same box, now searching the DOCUMENT ─────────────────────────────────────────
    // `nodes` is read for its DEPENDENCY, not its value: it is the mirror `editor_ops::refresh_docks`
    // pushes at every mutation site (place, move, delete, undo, redo, restore), so tracking it is how
    // this search re-runs when the document changes without inventing a second change signal. The
    // query is the other input. Both reads happen before any early return, or a blank query would
    // untrack the doc and the list would go stale the moment it re-armed.
    let doc_hits = RwSignal::new(Vec::<DocHit>::new());
    Effect::new(move |_| {
        let _tick = nodes.with(Vec::len);
        let q = layer_query.get();
        doc_hits.set(search_document(&document_rows(), &q));
    });

    // ── T-697 — the selection filter ─────────────────────────────────────────────────────────────
    // Recomputed from the live selection at every selection change. `selection_facets` returns the
    // empty list whenever nothing can actually be narrowed, so an empty vector here is the whole of
    // "this selection is homogeneous" and the row simply does not render.
    let sel_facets = RwSignal::new(Vec::<SelectionFacet>::new());
    Effect::new(move |_| {
        let n = selected.with(Vec::len);
        if n < 2 {
            sel_facets.set(Vec::new());
            return;
        }
        sel_facets.set(selection_facets(&selection_rows()));
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
    // T-812 — rename session id ONLY. The mid-edit draft must NOT live here: the bookmark-list
    // closure tracks `renaming`, and writing the draft through that signal remounts the input on
    // every keystroke. T-785's on_load then focus()+select()s the remounted node, so each typed
    // character replaces the whole draft (wave200 F2: "Ridge OP Two" → committed "o").
    let renaming = RwSignal::new(Option::<String>::None);
    // Draft text for the open rename session. The list render does not track this signal.
    let rename_draft = RwSignal::new(String::new());
    // Latch so Escape abandons without the subsequent blur committing.
    let rename_abandon = RwSignal::new(false);
    // T-812 — ADD open latch (bool). Draft is separate so typing does not remount the naming input
    // (same remount class as rename). `true` = the "name this view" box is armed.
    let adding = RwSignal::new(false);
    let add_draft = RwSignal::new(String::new());

    // Persist + re-render in one place, so no verb can mutate the collection without writing it.
    let commit_bookmarks = move |next: Bookmarks| {
        save_bookmarks(&next);
        bookmarks.set(next);
    };

    // The one-shot index load. T-762 — read the already-parsed LabelHost towns via
    // `world_assets::named_locations()`; do not re-fetch locations.json.
    let arm_places = move || {
        if places_armed.get_untracked() {
            return;
        }
        places_armed.set(true);
        places.set(load_named_places());
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
                    // T-812 / wave200 F7 — bare `autofocus` on this reactively-inserted node does not
                    // focus (focused_on_mount:false; 'g' flipped GRID). NodeRef + on_load focus+select,
                    // and the draft lives in `add_draft` so the open latch is not rewritten per char.
                    {move || {
                        if !adding.get() {
                            return None;
                        }
                        let add_ref = NodeRef::<leptos::html::Input>::new();
                        add_ref.on_load(|el: web_sys::HtmlInputElement| {
                            let _ = el.focus();
                            el.select();
                        });
                        Some(view! {
                            <input
                                type="text"
                                node_ref=add_ref
                                data-testid="dock-left-bookmark-name"
                                aria-label="Bookmark name"
                                // Uncontrolled after mount: a reactive prop:value can land AFTER
                                // on_load and clear the select-all; initial value= is enough.
                                value=add_draft.get_untracked()
                                class="mx-1 my-0.5 rounded border border-primary/60 bg-black/30 px-1.5 py-1 text-label-sm text-on-surface"
                                on:input=move |ev| add_draft.set(event_target_value(&ev))
                                on:blur=move |_| adding.set(false)
                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                    match ev.key().as_str() {
                                        "Enter" => {
                                            ev.prevent_default();
                                            let name = add_draft.get_untracked();
                                            if let Some((x, y, zoom)) = live_camera() {
                                                let mut next = bookmarks.get_untracked();
                                                if next.add(&name, x, y, zoom) {
                                                    commit_bookmarks(next);
                                                }
                                            }
                                            adding.set(false);
                                        }
                                        "Escape" => {
                                            ev.stop_propagation();
                                            adding.set(false);
                                        }
                                        _ => {}
                                    }
                                }
                            />
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
                                    let is_editing = editing.as_ref() == Some(&key);
                                    if is_editing {
                                        // T-785 — `autofocus` alone does NOT focus this input (reactive
                                        // insert). T-812 — the draft must NOT round-trip through the
                                        // list-tracked `renaming` signal: that remounted the input on
                                        // every keystroke, and on_load's focus+select then ate the
                                        // typed name down to one character (wave200 F2). Draft lives
                                        // in `rename_draft`; `renaming` holds only the row id. Select
                                        // runs once on session mount, never again mid-edit.
                                        let rename_ref = NodeRef::<leptos::html::Input>::new();
                                        rename_ref
                                            .on_load(|el: web_sys::HtmlInputElement| {
                                                let _ = el.focus();
                                                el.select();
                                            });
                                        return view! {
                                            <input
                                                type="text"
                                                node_ref=rename_ref
                                                data-testid="dock-left-bookmark-rename"
                                                aria-label="Rename bookmark"
                                                // Uncontrolled after mount (see ADD note): keeps
                                                // on_load select-all so the first keystroke replaces.
                                                value=rename_draft.get_untracked()
                                                class="mx-1 my-0.5 w-[calc(100%-0.5rem)] rounded border border-primary/60 bg-black/30 px-1.5 py-1 text-label-sm text-on-surface"
                                                on:input=move |ev| rename_draft.set(event_target_value(&ev))
                                                on:blur=move |_| {
                                                    if rename_abandon.get_untracked() {
                                                        rename_abandon.set(false);
                                                        renaming.set(None);
                                                        return;
                                                    }
                                                    if let Some(from) = renaming.get_untracked() {
                                                        let to = rename_draft.get_untracked();
                                                        let mut next = bookmarks.get_untracked();
                                                        if next.rename(&from, &to) {
                                                            commit_bookmarks(next);
                                                        }
                                                        renaming.set(None);
                                                    }
                                                }
                                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                    match ev.key().as_str() {
                                                        "Enter" => {
                                                            ev.prevent_default();
                                                            if let Some(from) = renaming.get_untracked()
                                                            {
                                                                let to = rename_draft.get_untracked();
                                                                let mut next = bookmarks.get_untracked();
                                                                if next.rename(&from, &to) {
                                                                    commit_bookmarks(next);
                                                                }
                                                            }
                                                            renaming.set(None);
                                                        }
                                                        "Escape" => {
                                                            ev.stop_propagation();
                                                            rename_abandon.set(true);
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
                                                    rename_abandon.set(false);
                                                    rename_draft.set(rename_key.clone());
                                                    renaming.set(Some(rename_key.clone()));
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

    // ── T-697 — the hit list ─────────────────────────────────────────────────────────────────────
    // One row per matching entity, above the tree, only while the box has something in it. A row is
    // a BUTTON when [`hit_is_routable`] — the REGISTERED route probe, i.e. the very resolution the
    // row's click runs — and INERT TEXT otherwise, carrying `unselectable_reason` as its title.
    //
    // **Wave 129 (RV-1): the affordance asks the click's own resolver, per ROW, never a kind list.**
    // This branch used to read `DocKind::is_selectable`, a hardcoded `Slot | Vehicle` set written
    // when the T-655 router really did resolve only those two. The router has since grown a Zone arm
    // (T-754) and an Entity arm (wave 129 F1), so zone and object hits rendered `aria-disabled` over
    // a click that WOULD have selected — the same affordance/click divergence as T-754 with the
    // polarity flipped, and just as much a lie to the author. There is exactly ONE decision now and
    // the click is on the other end of it.
    let hits_body = move || {
        let q = layer_query.get();
        if q.trim().is_empty() {
            return ().into_any();
        }
        let hits = doc_hits.get();
        if hits.is_empty() {
            // Half-typed and unreadable queries are NOT failed searches, and T-084 already owns the
            // three sentences that tell them apart — reused verbatim rather than re-worded here.
            let msg = crate::asset_catalog::search_empty_message(&q, "entities in this mission");
            return view! {
                <p class="mt-1 px-1 text-label-sm text-outline" data-testid="dock-left-search-empty">
                    {msg}
                </p>
            }
            .into_any();
        }
        let total = hits.len();
        let shown = total.min(MAX_DOC_HITS);
        view! {
            <section class="mt-1 flex shrink-0 flex-col" data-testid="dock-left-search-results">
                <h3 class="px-1 text-label-sm font-semibold uppercase text-outline">
                    {if shown == total {
                        format!("Found {total}")
                    } else {
                        format!("Found {total} — showing {shown}")
                    }}
                </h3>
                <div class="max-h-40 overflow-y-auto">
                    {hits
                        .into_iter()
                        .take(MAX_DOC_HITS)
                        .map(|hit| {
                            let routable = hit_is_routable(&hit);
                            let kind = hit.entity.kind;
                            let id = hit.entity.id.clone();
                            let badge = kind.noun();
                            let label = hit.entity.label.clone();
                            let matched = if hit.field == "faction" {
                                hit.entity.faction.clone()
                            } else {
                                hit.entity
                                    .text
                                    .iter()
                                    .find(|(f, _)| *f == hit.field)
                                    .map_or_else(String::new, |(_, v)| v.clone())
                            };
                            let body = view! {
                                <MaterialIcon
                                    name=kind.icon()
                                    class="block shrink-0 text-sm text-outline"
                                />
                                <span class="min-w-0 flex-1 truncate text-left text-label-sm">
                                    {label.clone()}
                                </span>
                                <span class="shrink-0 text-label-sm lowercase text-outline">
                                    {badge}
                                </span>
                            };
                            if routable {
                                let title = format!(
                                    "Select this {badge} — matched {} \"{matched}\" ({id})",
                                    hit.field,
                                );
                                let click_id = id.clone();
                                view! {
                                    <button
                                        type="button"
                                        title=title
                                        data-testid="dock-left-search-hit"
                                        class="flex w-full cursor-pointer items-center gap-1 rounded px-1 py-0.5 text-on-surface hover:bg-white/5"
                                        on:click=move |ev: web_sys::MouseEvent| {
                                            ev.stop_propagation();
                                            crate::validation_panel::route_select_by_subject_id(
                                                &click_id,
                                            );
                                        }
                                    >
                                        {body}
                                    </button>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <div
                                        aria-disabled="true"
                                        title=unselectable_reason(kind)
                                        data-testid="dock-left-search-hit-inert"
                                        class="flex w-full items-center gap-1 rounded px-1 py-0.5 text-outline"
                                    >
                                        {body}
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                        .collect_view()}
                </div>
            </section>
        }
        .into_any()
    };

    // ── T-697 — the selection filter's chips ─────────────────────────────────────────────────────
    // One chip per way the selection can ACTUALLY be narrowed (see `selection_facets`: a facet that
    // would keep everything is never emitted). Chips wrap rather than truncate, so this row cannot
    // overrun the 240 px column however many factions the selection straddles.
    let facets_row = move || {
        let facets = sel_facets.get();
        if facets.is_empty() {
            return ().into_any();
        }
        view! {
            <section class="mt-1 flex shrink-0 flex-col" data-testid="dock-left-selection-filter">
                <h3 class="px-1 text-label-sm font-semibold uppercase text-outline">
                    "Filter selection"
                </h3>
                <div class="flex flex-wrap gap-1 px-1 py-0.5">
                    {facets
                        .into_iter()
                        .map(|f| {
                            let n = f.ids.len();
                            let title = format!(
                                "Keep only the {n} selected by {}: {}",
                                f.axis.to_lowercase(),
                                f.label,
                            );
                            let ids = f.ids.clone();
                            view! {
                                <button
                                    type="button"
                                    title=title
                                    data-testid="dock-left-selection-facet"
                                    class="shrink-0 cursor-pointer rounded border border-outline-variant/30 px-1.5 py-0.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                                    on:click=move |ev: web_sys::MouseEvent| {
                                        ev.stop_propagation();
                                        apply_selection(ids.clone());
                                    }
                                >
                                    {format!("{} ({n})", f.label)}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </section>
        }
        .into_any()
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
                                        add_draft.set(bookmarks.with_untracked(default_bookmark_name));
                                        adding.set(true);
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
                // T-803 (O-9) — the DROP-TARGET affordance: a persistent, on-screen statement of the
                // layer the next placement (unit OR comment) will land in. The hover tooltip on the
                // active Folder row was the only indication before; an armed placement gave the author
                // nothing cursor-adjacent to read, so they placed into a guess. This strip names the
                // destination `ensure_layer` will actually resolve — the active layer if one is set,
                // else the first top-level layer (the fallback the operator saw as "it doesn't place
                // in the root"). On the Layers tab only: the destination is a layer concept, and the
                // Locations tab has no drop target. `data-testid` + the "Placing into" text are the
                // scripted acceptance hook. NB the ROW background half of O-9 (a) lives on the tree
                // rows in `eden_tree.rs`, not here.
                {move || {
                    (tab.get() == LeftTab::Layers)
                        .then(|| {
                            let dest = active_layer.with(|a| {
                                a.as_deref()
                                    .and_then(|id| nodes.with(|ns| find_layer_label(ns, id)))
                            });
                            // Fall back to `ensure_layer`'s destination so the strip never lies: the
                            // active pointer, else the first layer, else (empty doc) the layer a
                            // placement would mint on the spot.
                            let (name, muted) = match dest {
                                Some(label) => (label, false),
                                None => match nodes.with(|ns| first_folder_label(ns)) {
                                    Some(label) => (label, true),
                                    None => ("a new layer".to_string(), true),
                                },
                            };
                            // Muted = the author has not chosen; still name the real destination but
                            // read it as a hint, distinct from a deliberately-armed target.
                            let name_cls = if muted {
                                "min-w-0 flex-1 truncate font-medium text-on-surface-variant"
                            } else {
                                "min-w-0 flex-1 truncate font-medium text-primary"
                            };
                            view! {
                                <div
                                    class="mt-1 flex shrink-0 items-center gap-1 rounded bg-black/20 px-1.5 py-0.5 text-label-sm text-outline"
                                    data-testid="dock-left-drop-target"
                                    title="Placements (units and comments) file into this layer. Click a layer row to change it."
                                >
                                    <MaterialIcon
                                        name="place"
                                        class="block shrink-0 text-sm leading-none"
                                    />
                                    <span class="shrink-0">"Placing into:"</span>
                                    <span class=name_cls>{name}</span>
                                </div>
                            }
                        })
                }}
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
                                    aria-label="Search the mission and filter editor layers"
                                    // T-697 — the box searches the DOCUMENT now, not just the tree,
                                    // so the placeholder advertises the grammar it shares with the
                                    // asset palette rather than naming one of its two surfaces.
                                    placeholder="Search mission — name, class:, mod:"
                                    prop:value=move || layer_query.get()
                                    class="mt-1 w-full shrink-0 rounded border border-outline-variant/30 bg-black/20 px-1.5 py-0.5 text-label-sm text-on-surface outline-none placeholder:text-outline focus:border-primary/60"
                                    on:input=move |ev| layer_query.set(event_target_value(&ev))
                                />
                            }
                        })
                }}
                // T-697 — the selection filter, then the document hits, then the tree. The chips sit
                // above the results because they act on what is ALREADY selected (a state the author
                // arrived with), while the hit list is the answer to what they are typing now.
                {move || (tab.get() == LeftTab::Layers).then(facets_row)}
                {move || (tab.get() == LeftTab::Layers).then(hits_body)}
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
// parsed once at boot by `world_assets::labels::LabelHost::init`. This dock reads them through
// `world_assets::named_locations()` (T-762) — the already-parsed `LabelHost` towns — and does not
// re-fetch or re-parse the file.
//
// **THE CAMERA.** Flying to a place must not be a SECOND camera mover. The one mover is
// `RenderEngine::set_view` followed by `on_camera_changed` and a viewport flush — the path
// `editor_ops::center_on_selection` (Space), the validation panel's finding jump, and the
// initial view all take. T-762 promotes that sequence as `world_assets::fly_to` on the same
// `RENDER_CTX` seam as `camera_snapshot` / `apply_grid`. [`fly_to`] here only resolves the
// optional zoom (bookmark carries one; a named location keeps the live zoom) and forwards.
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
///
/// T-697 — it is now T-084's grammar, through the one matcher [`query_hits`], rather than a hand-run
/// `to_lowercase().contains()`. The plain behaviour this function was written for is UNCHANGED (an
/// empty query matches everything; a literal is a case-insensitive substring — those are exactly
/// `SearchPattern::All` and `SearchPattern::Plain` against the `Label` field), and `*`, `?` and
/// `/…/` now work here too. The point is not the extra patterns: it is that the layers tree, the
/// bookmarks list, the locations index and the document search below cannot drift into four ideas of
/// what the box means. A bookmark and a location have no class name and no faction, so `class:` and
/// `mod:` match nothing against them — the honest answer, since those rows carry no such datum.
#[must_use]
pub fn matches_query(name: &str, query: &str) -> bool {
    query_hits(query, name, "", "")
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

// ── T-803 (O-9) — name the drop target the author is placing into ─────────────────────────────────
//
// Which layer receives the next placement was set by clicking an outliner Folder row
// (`editor_ops::set_active_layer`) and its ONLY indication was a hover tooltip — no persistent,
// on-screen statement of the destination. The author placed into a guess; comments compounded it
// (both units and comments resolve their layer through `editor_ops::ensure_layer`). These two helpers
// resolve the destination the SAME way `ensure_layer` does, so the header strip below names the layer
// a placement will ACTUALLY land in, not a hopeful one:
//
//   * `find_layer_label` — the active layer's label, if the active id still points at a live Folder
//     (the untracked stale-pointer guard in `ensure_layer` mirrors this: a deleted/undone-away id is
//     not the destination).
//   * `first_folder_label` — the FALLBACK destination when nothing is active: `ensure_layer` files
//     into `rows.first()`, i.e. the first top-level Folder — NOT the "root"/Unfiled bucket. Naming it
//     is the fix for the operator's "it doesn't place in the root" surprise: the destination was
//     always a real layer, just an unstated one.

/// T-803 — the label of the Folder whose id is `id`, searched depth-first through the outliner tree
/// (`Unfiled`/`Faction`/`Squad`/`Slot`/`Comment` kinds are never drop targets, so only `Folder` nodes
/// answer). `None` when the id is absent — a stale active pointer, exactly the case `ensure_layer`
/// clears before falling back.
#[must_use]
pub fn find_layer_label(nodes: &[OutlinerNode], id: &str) -> Option<String> {
    for n in nodes {
        if n.kind == crate::outliner::NodeKind::Folder && n.id == id {
            return Some(n.label.clone());
        }
        if let Some(found) = find_layer_label(&n.children, id) {
            return Some(found);
        }
    }
    None
}

/// T-803 — the label of the first top-level `Folder`, mirroring `ensure_layer`'s `rows.first()`
/// fallback destination. Skips the virtual `Unfiled` root and any ORBAT headers, which are not doc
/// layers and never receive a placement. `None` only when the doc has no layer at all (the empty
/// mission — `ensure_layer` mints `DEFAULT_LAYER` in that case, so the strip below says "a new layer").
#[must_use]
pub fn first_folder_label(nodes: &[OutlinerNode]) -> Option<String> {
    nodes
        .iter()
        .find(|n| n.kind == crate::outliner::NodeKind::Folder)
        .map(|n| n.label.clone())
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

/// T-696 / T-762 — FLY TO `(x, y)`, optionally at `zoom` (a bookmark carries one; a named location
/// does not, and keeps the current zoom).
///
/// This is NOT a second camera mover. It resolves zoom then forwards to `world_assets::fly_to`,
/// which runs `set_view` → `on_camera_changed` → `flush_viewport` on the registered `RENDER_CTX`
/// (same sequence as `editor_ops::center_on_selection` / the T-166 smoke hook body). A no-op
/// before the engine mounts.
#[allow(unused_variables)]
pub fn fly_to(x: f64, y: f64, zoom: Option<f64>) {
    #[cfg(target_arch = "wasm32")]
    {
        let Some(z) = zoom.or_else(|| live_camera().map(|(_, _, z)| z)) else {
            return; // no engine yet — nothing to fly.
        };
        crate::world_assets::fly_to(x, y, z);
    }
}

/// T-696 / T-762 — map the boot-parsed `world_assets::named_locations()` index into dock rows.
/// Empty before the engine / label host mounts; the index is an accelerator, so an empty list is
/// not an error state.
fn load_named_places() -> Vec<NamedPlace> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut out: Vec<NamedPlace> = crate::world_assets::named_locations()
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        Vec::new()
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// T-697 — DOCUMENT SEARCH + THE SELECTION FILTER (3den E4 / 3DEN-TOOL-011)
// ═════════════════════════════════════════════════════════════════════════════════════════════════
//
// TBD could search the CATALOGUE (what you may place) and could not search the DOCUMENT (what you
// HAVE placed). At a WOG-corpus median of 137 placed entities per mission, find-by-name is not a
// nicety, and the gap was total: `filter_catalog` had exactly one caller (`eden_dock_right`), and
// the only reader of a placed vehicle / zone / trigger / marker / object was the panel that renders
// that one kind. Nothing anywhere asked a question of the whole document.
//
// **ONE GRAMMAR, NOT TWO — T-084's, reused.** The query language here is not new and must never
// become new: `parse_search_query` / `SearchField` / `SearchPattern` / `filter_catalog` /
// `search_empty_message` all come from `asset_catalog`, so `class:`, `mod:`, `*`, `?` and `/…/` mean
// in the document exactly what they mean in the palette, and the half-typed and broken-pattern
// empty states are literally the same sentences.
//
// The one thing T-084 does not expose is its MATCHER: `SearchPattern::hits` is private to
// `asset_catalog`, and that file is not this slice's to widen. So [`query_hits`] evaluates a query
// by handing `filter_catalog` a two-node PROJECTION of one candidate — a depth-0 folder whose label
// is the candidate's GROUP (its faction) holding one leaf whose `label` is the text being searched
// and whose `id` is its Enfusion class name. That is not a trick, it is the same shape the
// catalogue has, and it makes all three fields land on the right datum for free:
//
//   * `Label` (the default) — substring over the leaf's text, and a folder self-match keeps the
//     subtree, so a bare `BLUFOR` returns every BLUFOR entity exactly as it returns a whole addon
//     folder in the palette;
//   * `class:` — LEAF-ONLY prefix over the full `resourceName` or its `classname_tail`;
//   * `mod:` — DEPTH-0 only, i.e. the faction group.
//
// The cost is one `parse_search_query` per candidate string rather than one per query. That is a few
// hundred short-string parses per keystroke at the corpus median and is the price of not forking the
// grammar; a shared matcher in `asset_catalog` would remove it and is the seam to promote.
//
// **THE REGEX ARM IS NOT LOAD-BEARING (T-764).** T-084's `/…/` engine has a known stack-depth defect
// on very deep patterns. Nothing here requires it: `Plain` and `Glob` carry the feature, `Regex` is
// one arm of a pattern enum this file never constructs, and no default, placeholder or empty-state
// message pushes an author toward it. Not fixed here — it is a queued ticket and another slice's
// file.
//
// **RESULTS SELECT, OR THEY SAY THEY CANNOT — wog.md 14.6, filed three times: T-754, wave 129, and
// wave 129 RV-1 on this very surface.** A hit row routes through
// `validation_panel::route_select_by_subject_id`, the ONE shipped click-to-select router (T-655), and
// [`hit_is_routable`] asks that same router — through the REGISTERED probe
// `validation_panel::subject_id_routes` — whether THIS row's id would resolve. A row it says no to is
// rendered as INERT TEXT (no button, no pointer cursor, `aria-disabled`, and a title that says in
// words why); a row it says yes to is a button. One decision, both ends of it.
//
// The rule is an IFF and it is violated in BOTH directions. T-754 was the loud half — an affordance
// over a dead click. RV-1 was the quiet half: this file froze the router's reach into a
// `Slot | Vehicle` constant, the router then grew a `Zone` arm (T-754) and an `Entity` arm (wave 129
// F1), and zone/object hits went on rendering `aria-disabled` over a click that WOULD have selected,
// under a title asserting a limit the code no longer had. A kind list cannot track a router; only
// asking the router can.
//
// **WHERE IT LIVES, AND WHY THERE IS NO THIRD TAB.** T-637 measured the header as a width budget and
// `the_header_row_fits_the_dock` adds it up: at [`crate::eden_layout::DOCK_PX`] 240 with `p-2`
// gutters the row has 216 px and already spends 207.5 on the chevron, "Layers", "Locations" and the
// trailing verb. A third tab is ~50 px against 8.5 px of headroom — it does not fit, and because the
// tab group carries `min-w-0` it would not overflow, it would SQUEEZE and wrap silently. So document
// search is the LAYERS TAB's existing filter box, promoted: one box, T-084's grammar, two result
// surfaces that cannot disagree because [`matches_query`] (the tree's predicate, and the Locations
// tab's) is now the same [`query_hits`] call the document search is. `the_search_rows_fit_the_dock`
// below does the arithmetic for the two rows this ticket adds.
//
// A slot can therefore appear both in the filtered tree and in the hit list. That is deliberate, not
// a duplicate: the tree answers WHERE it lives (which layer, which parent), the list answers WHAT
// matched (which text attribute, and on a vehicle/marker/zone/trigger the tree has never held any
// row at all).

/// T-697 — what a matched document row IS. Drives the row glyph, the badge noun, and the noun that
/// [`unselectable_reason`] names. It does **not** decide clickability — [`hit_is_routable`] asks the
/// router that, per row (wave 129 RV-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocKind {
    /// A `slots` row (an ORBAT player/AI slot).
    Slot,
    /// A `vehiclesById` row.
    Vehicle,
    /// A `entitiesById` row — a placed world object (T-254).
    Object,
    /// A `factionsById[].briefing.markers[]` row (T-069).
    Marker,
    /// A `zones` row (T-582).
    Zone,
    /// A `triggers` row.
    Trigger,
    /// A `commentsById` row — the editor-only annotation (T-651).
    Comment,
    /// An `editorLayersById` folder.
    Layer,
}

impl DocKind {
    /// The badge noun, singular.
    #[must_use]
    pub fn noun(self) -> &'static str {
        match self {
            DocKind::Slot => "slot",
            DocKind::Vehicle => "vehicle",
            DocKind::Object => "object",
            DocKind::Marker => "marker",
            DocKind::Zone => "zone",
            DocKind::Trigger => "trigger",
            DocKind::Comment => "comment",
            DocKind::Layer => "layer",
        }
    }

    /// The row glyph (Material Symbols name), matching the icon each kind's own panel already uses.
    #[must_use]
    pub fn icon(self) -> &'static str {
        match self {
            DocKind::Slot => "person",
            DocKind::Vehicle => "directions_car",
            DocKind::Object => "category",
            DocKind::Marker => "place",
            DocKind::Zone => "crop_square",
            DocKind::Trigger => "bolt",
            DocKind::Comment => "sticky_note_2",
            DocKind::Layer => "folder",
        }
    }
}

/// **WOULD A CLICK ON THIS HIT SELECT ANYTHING? — the T-754 rule, asked of the click's own router.**
///
/// Wave 129 (RV-1), the peer of `validation_panel::finding_is_routable` and
/// `eden_settings::owner_is_routable`. [`crate::validation_panel::subject_id_routes`] is the
/// REGISTERED route probe — an `Rc` of the same resolution `route_select_by_subject_id` runs,
/// narrowed by `mission_editor::route_availability` — so the affordance and the click cannot answer
/// differently. Asking it per ROW rather than per KIND is the whole of the fix.
///
/// This replaced `DocKind::is_selectable`, a hardcoded `Slot | Vehicle` list. That list was true of
/// T-655's router and false of the one that ships: T-754 added a `Zone` arm and wave-129 F1 added an
/// `Entity` (placed object) arm, so zone and object hits were painted INERT over a click that would
/// have worked, while the row's title asserted a router limit that no longer existed. **Do not
/// re-derive this from a kind list, and do not fall back to `mission_editor::route_target` when no
/// probe is registered** — no probe means no router to click into, and `false` is the honest answer.
#[must_use]
pub fn hit_is_routable(hit: &DocHit) -> bool {
    crate::validation_panel::subject_id_routes(&hit.entity.id)
}

/// T-697 — why a hit row is inert, in words the author can act on. Rendered as the row's `title`
/// (and its `aria-description`) so the answer is available exactly where the click would have been.
///
/// Wave 129 (RV-1) rewrote this sentence. It used to read "resolves slots and vehicles only", which
/// stopped being true when the router grew its zone and entity arms — an inert row was explaining
/// itself with a false statement about the code. It now says what [`hit_is_routable`] actually
/// found: nothing to route to for this row, right now.
#[must_use]
pub fn unselectable_reason(kind: DocKind) -> String {
    format!(
        "Found, but not selectable from here: the editor's click-to-select router resolves no \
         selection for this {} right now, so a click would do nothing. Open it from its own panel.",
        kind.noun()
    )
}

/// T-697 — one placed thing in the document, projected for search. Built by
/// `editor_ops::document_entities` (wasm, where the doc handles live) and consumed by the pure
/// functions here, which is what makes the whole search natively testable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocEntity {
    /// The doc id — what the click-to-select router is handed.
    pub id: String,
    pub kind: DocKind,
    /// The row's display name (already fallen back: never empty).
    pub label: String,
    /// The Enfusion `resourceName` / object alias this row spawns as, empty when it has none. This
    /// is the datum `class:` matches, and only this one.
    pub class_name: String,
    /// The side this row belongs to (`BLUFOR` / `OPFOR` / `INDFOR`, or a library faction's key),
    /// empty when the kind carries none. The datum `mod:` matches, and the selection filter's
    /// faction axis.
    pub faction: String,
    /// **THE TEXT ATTRIBUTES** — `(field name, value)`, in the order the author thinks of them. The
    /// search runs over EACH of these separately rather than over a concatenation, so a glob stays
    /// whole-string per attribute (`Alpha-?` matches a callsign, not a callsign glued to a rank) and
    /// so a hit can report WHICH attribute it came from. Never empty: every row carries at least its
    /// id, because searching for an id an error message quoted is a real thing authors do.
    pub text: Vec<(&'static str, String)>,
}

/// T-697 — one search hit: the row, and the text attribute that matched it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocHit {
    pub entity: DocEntity,
    /// The `text` field name that matched, or `"faction"` when only the faction/folder matched
    /// (T-776) — shown on the row so a hit is never mysterious.
    pub field: &'static str,
}

/// T-697 — how many hit rows the dock renders. The count reported to the author is the FULL one (see
/// the results header); this only bounds the DOM. A 240 px column cannot show 2,000 rows usefully and
/// mounting them would cost more than the search does.
pub const MAX_DOC_HITS: usize = 200;

/// T-697 — the hit row's leading glyph box: a Material Symbol at `text-sm` (14 px), and a symbol
/// glyph is 1 em square.
const HIT_ICON_PX: f64 = 14.0;
/// T-697 — the hit row's `gap-1` (4 px), of which the row has two: icon│label│badge.
const HIT_GAP_PX: f64 = 4.0;
/// T-697 — the hit row's own `px-1` gutter (4 px each side).
const HIT_ROW_PAD_PX: f64 = 8.0;
/// T-697 — the width a vertical scrollbar claims from a scrolling list. Chromium's classic scrollbar
/// is 15 px; overlay scrollbars take 0. Budget for the classic one — the pin must not pass only on
/// the machine whose scrollbars happen to be free.
const LIST_SCROLLBAR_PX: f64 = 15.0;
/// T-697 — the least width the truncating hit LABEL may be left with and still be a label. Below
/// this the row degrades into an ellipsis with a badge beside it, which is furniture: it would name
/// nothing the author could recognise, and a search result that cannot be read is not a result.
const HIT_MIN_LABEL_PX: f64 = 80.0;

/// T-697 — **the one matcher.** Evaluate T-084's grammar against a single candidate.
///
/// `text` is the string being searched, `class_name` the row's Enfusion class (what `class:`
/// matches), `group` its faction (what `mod:` matches, and what a plain query matches as a
/// containing folder). See the section header for why this is a `filter_catalog` call over a
/// two-node projection rather than a matcher of its own.
#[must_use]
pub fn query_hits(query: &str, text: &str, class_name: &str, group: &str) -> bool {
    let leaf = crate::asset_catalog::CatalogNode {
        id: class_name.to_string(),
        label: text.to_string(),
        default_expanded: false,
        children: Vec::new(),
        // A `payload` is what makes a node a LEAF to `filter_catalog` (module rule: "a leaf is
        // `payload.is_some()`"), which is what puts it in the `class:` field's leaf-only path. The
        // values are inert here — this projection never reaches a palette.
        payload: Some(crate::asset_catalog::PlacePayload {
            asset_id: class_name.to_string(),
            role: String::new(),
        }),
    };
    let root = crate::asset_catalog::CatalogNode {
        id: String::new(),
        label: group.to_string(),
        default_expanded: false,
        children: vec![leaf],
        payload: None,
    };
    !crate::asset_catalog::filter_catalog(std::slice::from_ref(&root), query).is_empty()
}

/// T-697 — search the whole document. Returns one hit per MATCHING ENTITY (not per matching
/// attribute), carrying the FIRST text attribute that matched, in `rows` order — or `"faction"`
/// when the entity matched only via its faction/folder group (T-776).
///
/// A blank query returns NO hits rather than every row: an untouched filter box is not a request to
/// list the mission, and answering it with 137 rows would bury the tree under the box that opened
/// them. Half-typed (`class:`) and unreadable (`/[/`) queries return none too — `filter_catalog`
/// already draws that line, and [`crate::asset_catalog::search_empty_message`] is what says which of
/// the three empty answers this is.
#[must_use]
pub fn search_document(rows: &[DocEntity], query: &str) -> Vec<DocHit> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    rows.iter()
        .filter_map(|e| {
            // Prefer an attribute that hits WITHOUT the faction/folder contributing. A plain
            // `BLUFOR` matches every entity of that side via the group node (deliberate subtree
            // semantics) — but that is not a `name` hit. T-776: reporting the first text
            // attribute here claimed `matched name "…"` when only the faction matched.
            if let Some((field, _)) = e
                .text
                .iter()
                .find(|(_, v)| query_hits(query, v, &e.class_name, ""))
            {
                return Some(DocHit {
                    entity: e.clone(),
                    field,
                });
            }
            if query_hits(query, "", &e.class_name, &e.faction) {
                return Some(DocHit {
                    entity: e.clone(),
                    field: "faction",
                });
            }
            None
        })
        .collect()
}
/// T-697 — one way to narrow the live selection, and the exact ids it would leave selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionFacet {
    /// `"Type"` or `"Faction"` — which axis of the ticket's "by type or faction" this is.
    pub axis: &'static str,
    /// The chip's label (`"vehicle"`, `"BLUFOR"`).
    pub label: String,
    /// The ids the selection becomes. Always a PROPER, non-empty subset of the input (see
    /// [`selection_facets`]).
    pub ids: Vec<String>,
}

/// T-697 — **the selection filter.** Every way the given selection can actually be narrowed, by type
/// then by faction, types in [`DocKind`] order and factions alphabetical.
///
/// A facet is emitted **only when it is a proper subset**. A chip that would keep everything selected
/// narrows nothing, and rendering it would be the T-754 mistake in a second costume — a control that
/// looks like it acts and does not. So a homogeneous selection (six BLUFOR slots) yields NO chips and
/// the dock says so, rather than offering "slot (6)" and "BLUFOR (6)" as no-ops.
///
/// Rows with an empty `faction` are grouped under one explicit "no faction" chip rather than being
/// dropped: "the ones that belong to nobody" is a real thing to narrow to, and silently omitting them
/// would make the chip counts fail to sum to the selection.
#[must_use]
pub fn selection_facets(rows: &[DocEntity]) -> Vec<SelectionFacet> {
    let total = rows.len();
    if total < 2 {
        return Vec::new();
    }
    let mut out: Vec<SelectionFacet> = Vec::new();
    let mut kinds: Vec<DocKind> = rows.iter().map(|e| e.kind).collect();
    kinds.sort_unstable();
    kinds.dedup();
    for k in kinds {
        let ids: Vec<String> = rows
            .iter()
            .filter(|e| e.kind == k)
            .map(|e| e.id.clone())
            .collect();
        if ids.len() < total {
            out.push(SelectionFacet {
                axis: "Type",
                label: k.noun().to_string(),
                ids,
            });
        }
    }
    let mut factions: Vec<&str> = rows.iter().map(|e| e.faction.as_str()).collect();
    factions.sort_unstable();
    factions.dedup();
    for f in factions {
        let ids: Vec<String> = rows
            .iter()
            .filter(|e| e.faction == f)
            .map(|e| e.id.clone())
            .collect();
        if ids.len() < total {
            out.push(SelectionFacet {
                axis: "Faction",
                label: if f.is_empty() {
                    "no faction".to_string()
                } else {
                    f.to_string()
                },
                ids,
            });
        }
    }
    out
}

/// T-697 — apply a narrowed selection through the ONE selection seam, and report whether it took.
/// A no-op (and `false`) off wasm and before the editor mounts, like every other `editor_ops` reach
/// in this file.
#[allow(unused_variables)]
pub fn apply_selection(ids: Vec<String>) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        crate::editor_ops::set_selection_ids(ids) > 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// T-697 — the live document, projected for search. Empty off wasm / before the editor mounts.
#[must_use]
pub fn document_rows() -> Vec<DocEntity> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::editor_ops::document_entities()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Vec::new()
    }
}

/// T-697 — the live SELECTION, projected the same way, so the filter chips are computed from the
/// same rows the search is. Empty off wasm / before the editor mounts.
#[must_use]
pub fn selection_rows() -> Vec<DocEntity> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::editor_ops::selection_entities()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        default_bookmark_name, filter_bookmarks, filter_places, matches_query, sort_places,
        Bookmarks, LeftTab, NamedPlace, BOOKMARKS_KEY, BOOKMARKS_VERSION,
    };

    const SRC: &str = include_str!("eden_dock_left.rs");

    /// T-759 — **the haystack a POSITIVE source pin is allowed to read.** `SRC` is the WHOLE file,
    /// test module included, so a bare `SRC.contains(...)` is satisfied by the assertion that
    /// spells the needle. Every positive needle below therefore reads the file's PRODUCTION half
    /// through `class_r_scrub`, the same scrubber the `t697_document_search` module three tests
    /// down already uses on this same file — its first pass cuts everything from the first
    /// `#[cfg(test)]` onward, so a needle written in a test can no longer satisfy itself. It also
    /// drops comments, so section-header prose cannot keep a pin green.
    ///
    /// This form keeps string literals — a `data-testid` or a URL is code that ships, and pinning
    /// one is not the defect that pinning a comment is.
    ///
    /// The NEGATIVE needle in `the_index_and_the_fly_to_reuse_the_shipped_paths` deliberately stays
    /// on raw `SRC`: for "this must NOT appear", the widest unscrubbed haystack is the strongest
    /// one, and scrubbing could only ever hide a hit.
    fn live_src() -> String {
        crate::arsenal::class_r_scrub::live_source(SRC)
    }

    /// The same production half with string/char literals blanked as well — for needles that mean
    /// "this is real CODE", where the same text sitting in a literal is precisely the decoy.
    fn live_rust() -> String {
        crate::arsenal::class_r_scrub::live_code(SRC)
    }

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
        // T-759: the default is CODE, so it is read from the literal-blanked production half —
        // the same sentence quoted in a doc comment or a string cannot stand in for it.
        let code = live_rust();
        assert!(
            code.contains("let tab = RwSignal::new(LeftTab::Layers)"),
            "the dock must still open on the Editor Layers tree"
        );
        // T-759: the testids are string literals that SHIP, so literals are kept — but the test
        // module is cut, so this list cannot be its own evidence.
        let src = live_src();
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
            assert!(src.contains(needle), "missing driveable testid {needle}");
        }
    }

    /// **T-812 — bookmark rename + ADD: draft decoupled from list remount; NodeRef focus on mount.**
    ///
    /// Wave200 F2: T-785's rename on_load focus+select was correct for the unfocused insert, but the
    /// draft still lived inside the list-tracked `renaming` signal — every keystroke remounted the
    /// input and select() ate the name down to one character. The pin now requires `rename_draft`
    /// (list does not track it) and `renaming: Option<String>` (id only). Select runs on mount only.
    ///
    /// Wave200 F7: the ADD ("name this view") input had bare autofocus on a reactive insert —
    /// focused_on_mount:false, 'g' flipped GRID. Same NodeRef + on_load focus+select, with
    /// `add_draft` decoupled from the `adding` open latch.
    ///
    /// A source pin because this file's view is `#[cfg(target_arch = "wasm32")]` and there is no
    /// wasm-bindgen-test harness. Mechanism tokens are CODE in the literal-blanked production half.
    #[test]
    fn bookmark_rename_and_add_decouple_draft_and_focus_on_mount() {
        let code = live_rust();
        let src = live_src();
        assert!(
            code.contains("let rename_draft = RwSignal::new(String::new())"),
            "rename draft must be its own signal so the list render does not track mid-edit text"
        );
        assert!(
            code.contains("let renaming = RwSignal::new(Option::<String>::None)"),
            "renaming must hold only the row id (Option<String>), not the draft pair"
        );
        assert!(
            !code.contains("Option::<(String, String)>"),
            "the (id, draft) pair shape must be gone — that was the F2 remount trap"
        );
        assert!(
            src.contains("node_ref=rename_ref") && src.contains("node_ref=add_ref"),
            "both rename and ADD inputs must bind a NodeRef"
        );
        assert!(
            code.contains("let add_draft = RwSignal::new(String::new())"),
            "ADD draft must be decoupled from the open latch"
        );
        assert!(
            code.contains(".on_load(") && code.contains(".focus()") && code.contains(".select()"),
            "on_load must call focus() and select() on the mounted inputs"
        );
        assert!(
            src.contains("value=rename_draft.get_untracked()")
                && src.contains("value=add_draft.get_untracked()"),
            "both inputs must seed from local draft via value= (uncontrolled after mount)"
        );
        assert!(
            !src.contains("prop:value=move || rename_draft.get()")
                && !src.contains("prop:value=move || add_draft.get()"),
            "reactive prop:value on these inputs clears on_load select-all — banned"
        );
    }

    /// T-696 / T-762 — source pins for the wasm-only halves: the index reads the boot-parsed
    /// `world_assets::named_locations` seam (not a second fetch), and fly-to rides
    /// `world_assets::fly_to` (not the T-166 `__editorCamSet` smoke hook).
    #[test]
    fn the_index_and_the_fly_to_reuse_the_shipped_paths() {
        // T-759: positives read the scrubbed PRODUCTION half via class_r_scrub.
        let code = live_rust();
        assert!(
            code.contains("named_locations"),
            "the index must read world_assets::named_locations, not re-fetch locations.json"
        );
        assert!(
            code.contains("world_assets::fly_to"),
            "fly-to must call the world_assets::fly_to RENDER_CTX seam"
        );
        // Delete-prod RED: production must not couple to the smoke-hook name.
        assert!(
            !code.contains("__editorCamSet"),
            "fly-to must not invoke the T-166 __editorCamSet smoke hook"
        );
        // The NEGATIVE stays on the whole unscrubbed file on purpose: "there is no second mover"
        // is only as strong as the text it searched, and scrubbing could only hide a hit. The
        // needle is still split so this line itself is not one (the personnel.rs `production_src`
        // idiom).
        let second_mover = format!("{}{}", "set_view", "(");
        assert!(
            !SRC.contains(&second_mover),
            "there must be no SECOND camera mover in this dock"
        );
        assert!(
            code.contains("camera_snapshot"),
            "a new bookmark must record the LIVE camera, not a guess"
        );
    }

    /// T-762 / wave 131 F1 — dock wiring alone is hollow if `world_assets::fly_to` /
    /// `named_locations` bodies are gutted. Pin the wasm-only `mod.rs` via `include_str!` so
    /// HOST `cargo test` still goes RED when RENDER_CTX / towns() disappear. Needles split so
    /// this assertion line cannot satisfy itself.
    #[test]
    fn fly_to_and_named_locations_bodies_are_live() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let src = live_code(include_str!("world_assets/mod.rs"));
        let fly = only_body(&src, "pub fn fly_to");
        let render = format!("{}{}", "RENDER", "_CTX");
        let set_view = format!("{}{}", "set_view", "(");
        let on_cam = format!("{}{}", "on_camera_changed", "(");
        let flush = format!("{}{}", "flush_viewport", "(");
        assert!(
            fly.contains(&render),
            "T-762: fly_to must reach RENDER_CTX (not a gutted no-op)"
        );
        assert!(
            fly.contains(&set_view),
            "T-762: fly_to must call set_view on the live engine"
        );
        assert!(
            fly.contains(&on_cam),
            "T-762: fly_to must call on_camera_changed after set_view"
        );
        assert!(
            fly.contains(&flush),
            "T-762: fly_to must flush_viewport so residency catches up"
        );
        let named = only_body(&src, "pub fn named_locations");
        let towns = format!("{}{}", "towns", "()");
        assert!(
            named.contains(&towns),
            "T-762: named_locations must read LabelHost towns(), not return an empty Vec"
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
/// **NOTE ON THE PIN IDIOM.** Every source needle below is checked against the PRODUCTION half only
/// (`split("#[cfg(test)]").next()`), so a needle written here cannot satisfy itself. The T-696 pins
/// in the module above once did `contains()` against the WHOLE file, needle and assertion together —
/// T-759 fixed that; they now read `class_r_scrub`'s scrubbed production half.
#[cfg(test)]
mod t637_density {
    use super::{
        filter_outliner, find_layer_label, first_folder_label, matches_query, TAB_LABEL_LAYERS,
        TAB_LABEL_PAD_PX, TAB_LABEL_PLACES, UPPERCASE_LABEL_ADVANCE_PX,
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

    /// T-803 (O-9) — **the drop-target resolvers name the layer `ensure_layer` actually files into.**
    /// `find_layer_label` answers only for a live `Folder` id (a `Slot`/nested `Folder` id resolves
    /// through the recursion; a stray or non-Folder id gives `None`, the stale-pointer case
    /// `ensure_layer` clears before it falls back). `first_folder_label` is that fallback — the first
    /// top-level layer, which is where a placement lands when nothing is active, and the reason the
    /// operator saw "it doesn't place in the root": the root/Unfiled bucket is never the destination.
    #[test]
    fn the_drop_target_resolvers_name_the_real_destination() {
        let t = tree();

        // The active layer, by id, at any depth: top-level and nested both answer with their label.
        assert_eq!(find_layer_label(&t, "l1").as_deref(), Some("Assault"));
        assert_eq!(
            find_layer_label(&t, "l2").as_deref(),
            Some("Support"),
            "a nested folder is a valid drop target; the walk must reach it"
        );
        assert_eq!(find_layer_label(&t, "l3").as_deref(), Some("Recon"));

        // A SLOT id is not a layer, and a stray id is nobody: both are the `None` that sends
        // `ensure_layer` to its fallback. If `find_layer_label` answered for a slot, the strip would
        // name a destination that cannot receive a placement.
        assert_eq!(
            find_layer_label(&t, "s1"),
            None,
            "a slot id is not a drop target — only Folder kinds answer"
        );
        assert_eq!(find_layer_label(&t, "ghost"), None);

        // The fallback destination is the FIRST top-level layer — the same `rows.first()`
        // `ensure_layer` uses when nothing is active. Naming it is the fix for the "root" surprise.
        assert_eq!(first_folder_label(&t).as_deref(), Some("Assault"));

        // Empty doc ⇒ no layer to name; the strip says "a new layer" (what `ensure_layer` mints).
        assert_eq!(first_folder_label(&[]), None);

        // PERTURB the rule the operator's complaint turns on: were the fallback the ROOT/Unfiled
        // bucket instead of the first real layer, the strip would promise a destination placements
        // never reach. State that wrong answer and assert the real one differs from it.
        let unfiled = node("__unfiled__", "Unfiled", NodeKind::Unfiled, vec![]);
        let mut with_unfiled = vec![unfiled];
        with_unfiled.extend(tree());
        assert_eq!(
            first_folder_label(&with_unfiled).as_deref(),
            Some("Assault"),
            "PERTURB: the fallback must skip the virtual Unfiled root — it is not a doc layer and \
             receives no placement"
        );
    }

    /// T-803 (O-9) — **the persistent drop-target affordance ships in the dock.** The active layer's
    /// only indication was a hover tooltip; this pins the on-screen statement (the `data-testid` hook
    /// the scripted acceptance clicks for, the "Placing into:" copy, and that it reads BOTH
    /// `active_layer` and the resolver so it names the real destination, not a static string).
    /// Checked on the scrubbed production half, so this test's own mention cannot keep it green.
    #[test]
    fn the_drop_target_affordance_ships() {
        let src = production();
        assert!(
            src.contains("dock-left-drop-target"),
            "T-803: the drop-target statement needs a stable test hook for the scripted acceptance"
        );
        assert!(
            src.contains("Placing into:"),
            "T-803: the affordance must NAME the destination on screen, not only in a hover tooltip"
        );
        assert!(
            src.contains("find_layer_label"),
            "T-803: the strip must resolve the ACTIVE layer's label, or it cannot name the target"
        );
        assert!(
            src.contains("first_folder_label"),
            "T-803: the strip must fall back to `ensure_layer`'s first-layer destination, or it \
             would lie about where a placement lands when nothing is active"
        );
    }
}

/// T-697 — document search and the selection filter.
///
/// **NOTE ON THE PIN IDIOM.** Every source needle below is checked against `class_r_scrub`'s scrubbed
/// PRODUCTION text (`live_code` / `live_source`), whose first pass cuts the test module outright. A
/// pin that can be satisfied by itself is not a pin. The T-696 module above used to do exactly that
/// — `SRC = include_str!(whole file)`, so its needles matched their own assertions and would have
/// survived the deletion of the code they pinned; T-759 pointed those pins at these same helpers.
#[cfg(test)]
mod t697_document_search {
    use super::{
        hit_is_routable, matches_query, search_document, selection_facets, unselectable_reason,
        DocEntity, DocHit, DocKind, HIT_GAP_PX, HIT_ICON_PX, HIT_MIN_LABEL_PX, HIT_ROW_PAD_PX,
        LIST_SCROLLBAR_PX, MAX_DOC_HITS, UPPERCASE_LABEL_ADVANCE_PX,
    };
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};
    use crate::eden_layout::{tw_len_px, DOCK_L, DOCK_PX};
    use crate::mission_editor::route_target;
    use crate::validation_panel::register_route_probe;

    /// The dock's own production text — comments, test modules and unreachable arms removed.
    fn dock_code() -> String {
        live_code(include_str!("eden_dock_left.rs"))
    }
    /// The same, with string literals KEPT: for pins about copy and `data-testid`s that ship.
    fn dock_source() -> String {
        live_source(include_str!("eden_dock_left.rs"))
    }
    /// The document index's production text (`editor_ops.rs` carries no test module of its own).
    fn ops_code() -> String {
        live_code(include_str!("editor_ops.rs"))
    }

    fn entity(id: &str, kind: DocKind, label: &str, faction: &str) -> DocEntity {
        DocEntity {
            id: id.to_string(),
            kind,
            label: label.to_string(),
            class_name: String::new(),
            faction: faction.to_string(),
            text: vec![("label", label.to_string()), ("id", id.to_string())],
        }
    }

    /// A realistic small mission: two BLUFOR slots (one with a callsign the role does not contain),
    /// an OPFOR vehicle with a class name and no authored text, and a zone.
    fn mission() -> Vec<DocEntity> {
        vec![
            DocEntity {
                id: "slot-1".into(),
                kind: DocKind::Slot,
                label: "Rifleman".into(),
                class_name: "{26A9756790131354}Prefabs/Characters/Character_US_Rifleman.et".into(),
                faction: "BLUFOR".into(),
                text: vec![
                    ("role", "Rifleman".into()),
                    ("callsign", "Alpha-1".into()),
                    ("class", "Character_US_Rifleman".into()),
                    ("id", "slot-1".into()),
                ],
            },
            DocEntity {
                id: "slot-2".into(),
                kind: DocKind::Slot,
                label: "Medic".into(),
                class_name: String::new(),
                faction: "BLUFOR".into(),
                text: vec![("role", "Medic".into()), ("id", "slot-2".into())],
            },
            DocEntity {
                id: "veh-1".into(),
                kind: DocKind::Vehicle,
                label: "UAZ469".into(),
                class_name: "{ABCD}Prefabs/Vehicles/UAZ469.et".into(),
                faction: "OPFOR".into(),
                text: vec![("class", "UAZ469".into()), ("id", "veh-1".into())],
            },
            DocEntity {
                id: "zone-1".into(),
                kind: DocKind::Zone,
                label: "Objective Alpha".into(),
                class_name: String::new(),
                faction: "OPFOR".into(),
                text: vec![("label", "Objective Alpha".into()), ("id", "zone-1".into())],
            },
        ]
    }

    fn ids(hits: &[DocHit]) -> Vec<&str> {
        hits.iter().map(|h| h.entity.id.as_str()).collect()
    }

    /// **THE DOCUMENT IS SEARCHED, AND THE CATALOGUE ALREADY WAS.** The ticket's whole existence:
    /// `filter_catalog` had exactly one caller before this ticket (the right dock's palette), and a
    /// placed vehicle / zone / trigger / marker / object was readable by its own panel and by
    /// nothing else. These are the questions that had no answer.
    #[test]
    fn the_placed_document_is_searchable_by_text() {
        let m = mission();
        // A plain label search reaches a slot's role and a zone's label.
        assert_eq!(ids(&search_document(&m, "rifle")), ["slot-1"]);
        assert_eq!(ids(&search_document(&m, "objective")), ["zone-1"]);
        // …and, the point of the ticket, a VEHICLE — a kind no tree in this editor has ever held.
        assert_eq!(ids(&search_document(&m, "uaz")), ["veh-1"]);
    }

    /// **EVERY TEXT ATTRIBUTE, NOT JUST THE DISPLAY LABEL — and the hit says which one.** A slot's
    /// callsign is not in its role, so a search that only saw the tree label would miss it. The
    /// reported field is what stops a hit from being mysterious ("why did THAT match?").
    #[test]
    fn the_search_covers_every_text_attribute_and_names_the_one_that_matched() {
        let m = mission();
        let hits = search_document(&m, "alpha");
        // `Alpha-1` is slot-1's callsign; `Objective Alpha` is the zone's label.
        assert_eq!(ids(&hits), ["slot-1", "zone-1"]);
        assert_eq!(hits[0].field, "callsign");
        assert_eq!(hits[1].field, "label");
        // An id is a text attribute too: authors paste ids out of validation findings.
        assert_eq!(ids(&search_document(&m, "veh-1")), ["veh-1"]);
        assert_eq!(search_document(&m, "veh-1")[0].field, "id");
    }

    /// **ONE GRAMMAR — T-084's, not a second one.** The four pattern kinds and the three fields all
    /// behave in the document exactly as they behave in the palette, because they ARE the palette's:
    /// `class:` prefixes the resource name or its tail, `mod:` takes the faction group, `*`/`?` glob
    /// whole-string per attribute, `/…/` is an unanchored regex.
    /// T-776 — a plain faction hit must name `faction`, not the entity's first text attribute.
    /// `BLUFOR` returns every BLUFOR entity via folder self-match (deliberate); lying that the
    /// *name* matched is the honesty gap wave-119 NIT-4 named. Delete the faction branch in
    /// [`search_document`] and this goes red.
    #[test]
    fn a_faction_only_hit_names_faction_not_the_first_text_attribute() {
        let e = entity("slot-1", DocKind::Slot, "Alpha 1-1", "BLUFOR");
        assert!(
            !e.text
                .iter()
                .any(|(_, v)| v.to_lowercase().contains("blufor")),
            "precondition: no text attribute contains the faction token"
        );
        let hits = search_document(std::slice::from_ref(&e), "BLUFOR");
        assert_eq!(ids(&hits), ["slot-1"]);
        assert_eq!(
            hits[0].field, "faction",
            "T-776: a faction-only hit must report field=faction, not `{}`",
            hits[0].field
        );
        // And a real name hit still names the attribute — faction must not steal the credit.
        let by_name = search_document(std::slice::from_ref(&e), "Alpha");
        assert_eq!(by_name[0].field, "label");
    }

    #[test]
    fn the_query_grammar_is_t084s() {
        let m = mission();
        // `class:` — leaf-only, prefix, full resource name OR the classname tail (T-646/T-084).
        assert_eq!(
            ids(&search_document(&m, "class:Character_US_Ri")),
            ["slot-1"]
        );
        assert_eq!(ids(&search_document(&m, "class:{ABCD}Prefabs")), ["veh-1"]);
        // A bare label search must NOT behave like `class:` — `class:` stays a prefix.
        assert!(search_document(&m, "class:Rifleman").is_empty());
        // `mod:` — the depth-0 group, which in a document is the faction.
        assert_eq!(ids(&search_document(&m, "mod:OPFOR")), ["veh-1", "zone-1"]);
        // Glob, whole-string, per attribute.
        assert_eq!(ids(&search_document(&m, "Alpha-?")), ["slot-1"]);
        assert_eq!(ids(&search_document(&m, "Rifle*")), ["slot-1"]);
        // Regex, unanchored. NOT load-bearing (T-764's stack-depth defect lives on this arm) — it is
        // here because it comes free with the shared grammar, and nothing steers an author to it.
        assert_eq!(ids(&search_document(&m, "/^medic$/")), ["slot-2"]);
        // And the grammar is literally the palette's, not a copy of it.
        let src = dock_code();
        assert!(
            src.contains("asset_catalog::filter_catalog(")
                && src.contains("asset_catalog::search_empty_message("),
            "T-697: the document search must run T-084's matcher and T-084's empty states, not its own"
        );
        for reinvention in [
            "fn parse_search_pattern",
            "enum SearchPattern",
            "enum SearchField",
        ] {
            assert!(
                !src.contains(reinvention),
                "T-697: `{reinvention}` here would be a SECOND query language in one editor"
            );
        }
    }

    /// **ONE BOX, ONE MEANING.** The layers tree, the bookmarks list, the locations index and the
    /// document search all go through [`query_hits`], so they cannot drift into four ideas of what
    /// the filter box means. The plain behaviour T-696 wrote `matches_query` for is unchanged.
    #[test]
    fn one_predicate_serves_every_list_in_this_dock() {
        assert!(matches_query("Montignac", ""), "empty query matches all");
        assert!(matches_query("Montignac", "   "), "blank query matches all");
        assert!(matches_query("Montignac", "montignac"), "case-insensitive");
        assert!(matches_query("Montignac", "TIGN"), "substring, not prefix");
        assert!(!matches_query("Montignac", "levie"));
        // The grammar rides along for free.
        assert!(matches_query("Montignac", "Mont*"));
        assert!(
            !matches_query("Montignac", "class:Mont"),
            "no class name to match"
        );
        let dock = dock_code();
        assert!(
            only_body(&dock, "fn matches_query").contains("query_hits("),
            "T-697: `matches_query` must be the one matcher, or the tree and the search disagree"
        );
    }

    /// **A HALF-TYPED QUERY IS NOT A FAILED SEARCH**, and a blank one is not a request to list the
    /// whole mission. T-084 already draws both lines; this reuses its sentences rather than writing
    /// a fourth set.
    #[test]
    fn blank_and_half_typed_queries_find_nothing_and_say_which() {
        let m = mission();
        assert!(
            search_document(&m, "").is_empty(),
            "a blank box is not a query"
        );
        assert!(search_document(&m, "   ").is_empty());
        assert!(
            search_document(&m, "class:").is_empty(),
            "half-typed operator"
        );
        assert!(search_document(&m, "/[/").is_empty(), "unreadable regex");
        let msg =
            |q: &str| crate::asset_catalog::search_empty_message(q, "entities in this mission");
        assert!(msg("class:").contains("class:"), "guidance, not `no match`");
        assert!(msg("/[/").contains("could not be read"));
        assert!(msg("nosuchthing").contains("No entities in this mission match"));
    }

    /// **RESULTS SELECT, OR THEY SAY THEY CANNOT — wog.md 14.6 / T-754 / wave 129 RV-1.**
    ///
    /// A CORRESPONDENCE pin, and it is the correspondence that is checked, not a list: for every
    /// `DocKind` it computes the CLICK's own answer (`mission_editor::route_target` over a document
    /// with one id per kind) and requires [`super::hit_is_routable`] — which the view's
    /// button-vs-inert branch is — to equal it. Both directions, with a non-vacuity assert that this
    /// run saw at least one LIVE row and at least one INERT one.
    ///
    /// **Why the old pin could not have caught RV-1.** It restated `DocKind::is_selectable`'s own
    /// `Slot | Vehicle` constant back at it. When the router grew a `Zone` arm (T-754) and an
    /// `Entity` arm (wave 129 F1) the constant went stale, zone and object hits kept rendering
    /// `aria-disabled` over a click that would have selected, and the pin stayed green throughout —
    /// because it was checking the kind list against itself. A guard that repeats its subject is not
    /// a guard.
    ///
    /// Perturbation RED: restore the hardcoded list — decide the row from
    /// `matches!(kind, DocKind::Slot | DocKind::Vehicle)` instead of asking the registered probe.
    #[test]
    fn a_hit_row_is_a_live_affordance_iff_the_click_would_select() {
        // One id per kind, in the document the shipped resolver reads. Slot ids live in the SoA,
        // which is not in this root, so `is_slot` supplies them exactly as the router's caller does.
        // The last four maps are in the document and owned by NO selection surface — the router has
        // no arm for a briefing marker, a trigger, a comment or an editor layer.
        let root = serde_json::json!({
            "vehiclesById": { "veh-1": { "position": { "x": 10.0, "y": 20.0 } } },
            "entitiesById": { "obj-1": { "position": { "x": 30.0, "y": 40.0 } } },
            "zonesById": {
                "zone-1": { "shape": { "circle": { "x": 5.0, "z": 6.0, "r": 50.0 } } }
            },
            "factionsById": { "BLUFOR": { "briefing": { "markers": [{ "id": "mark-1" }] } } },
            "triggersById": { "trg-1": {} },
            "commentsById": { "cmt-1": {} },
            "editorLayersById": { "lay-1": {} }
        });
        fn is_slot(id: &str) -> bool {
            id == "slot-1"
        }
        // The probe is registered the way `mission_editor` registers it at mount: the SAME
        // resolution the click runs, asked as a question.
        let probe_root = root.clone();
        register_route_probe(std::rc::Rc::new(move |id: &str| {
            route_target(&probe_root, id, &is_slot).is_some()
        }));

        let mut live: Vec<DocKind> = Vec::new();
        let mut inert: Vec<DocKind> = Vec::new();
        for (id, kind) in [
            ("slot-1", DocKind::Slot),
            ("veh-1", DocKind::Vehicle),
            ("obj-1", DocKind::Object),
            ("zone-1", DocKind::Zone),
            ("mark-1", DocKind::Marker),
            ("trg-1", DocKind::Trigger),
            ("cmt-1", DocKind::Comment),
            ("lay-1", DocKind::Layer),
        ] {
            let hit = DocHit {
                entity: entity(id, kind, "Row", "BLUFOR"),
                field: "label",
            };
            // The ORACLE — what the click would actually do — computed independently of the view.
            let would_select = route_target(&root, id, &is_slot).is_some();
            assert_eq!(
                hit_is_routable(&hit),
                would_select,
                "RV-1: the {} row's affordance must EQUAL what a click on it would do (the router \
                 says {would_select}). Painting inert over a live click is the same lie as painting \
                 live over a dead one",
                kind.noun()
            );
            if would_select {
                live.push(kind);
            } else {
                inert.push(kind);
            }
        }

        // NOT VACUOUS: this pin is worth nothing unless it saw the affordance both ON and OFF.
        assert!(
            !live.is_empty() && !inert.is_empty(),
            "the correspondence must be exercised in both directions (saw live {live:?} / inert \
             {inert:?})"
        );
        // What the router reaches TODAY — reported by the resolver, not asserted into the view. The
        // day an arm is added or removed this line moves and the affordance moves with it, together.
        assert_eq!(
            live,
            vec![
                DocKind::Slot,
                DocKind::Vehicle,
                DocKind::Object,
                DocKind::Zone
            ],
            "T-655 + T-754 (zones) + wave-129 F1 (placed objects) are the shipped router's arms"
        );
        assert_eq!(
            inert,
            vec![
                DocKind::Marker,
                DocKind::Trigger,
                DocKind::Comment,
                DocKind::Layer
            ],
            "T-754: the router has no arm for these, so their rows must stay INERT — rendering one \
             as clickable is the defect this programme has filed three times"
        );
        for kind in &inert {
            let why = unselectable_reason(*kind);
            assert!(
                why.contains(kind.noun()),
                "an inert row must say WHY, naming the kind it is about"
            );
            assert!(
                !why.contains("slots and vehicles"),
                "RV-1: the reason may not assert a router limit the router does not have. It read \
                 `resolves slots and vehicles only` for the whole of the zone and entity widenings"
            );
        }

        // THE OTHER AXIS — same document, different mount state (F6/F7). The resolver refuses while
        // the owning panel is unmounted (or before the editor mounts, or on the host build), and the
        // row must follow the RESOLVER, not the document.
        register_route_probe(std::rc::Rc::new(|_: &str| false));
        let zone = DocHit {
            entity: entity("zone-1", DocKind::Zone, "Objective Alpha", "OPFOR"),
            field: "label",
        };
        assert!(
            !hit_is_routable(&zone),
            "F6/F7: the resolver refuses this subject, so the row is inert — a fallback to \
             `route_target` here IS the dead click"
        );
        assert!(
            route_target(&root, "zone-1", &is_slot).is_some(),
            "the document resolves `zone-1` in BOTH phases; only the probe's answer moved, which is \
             exactly why the affordance may not be decided from the document"
        );

        // SOURCE SIDE — one decision, and the click is on the other end of it.
        let code = dock_code();
        assert!(
            code.contains("validation_panel::route_select_by_subject_id("),
            "T-655/T-697: a hit must select through the ONE registered router"
        );
        assert!(
            !code.contains("editor_ops::select_slot("),
            "T-697: a second click-to-select path is how the two drift apart"
        );
        assert!(
            only_body(&code, &format!("fn hit{}", "_is_routable"))
                .contains(&format!("subject_id{}", "_routes")),
            "RV-1: clickability must be the REGISTERED probe's answer — the one the click runs"
        );
        // NEGATIVES, over the whole of this file's LIVE half (the test module, which legitimately
        // calls the router to state the FACT the affordance is checked against, is cut first).
        assert_eq!(
            code.matches(&format!("is{}", "_selectable(")).count(),
            0,
            "RV-1: the hardcoded kind list is gone. It is a second copy of the router's reach, and \
             a copy is a thing that can go stale — this one did, silently, for two widenings"
        );
        assert_eq!(
            code.matches(&format!("route{}", "_target(")).count(),
            0,
            "RV-1: no live code in this view may resolve the router itself — that is a second \
             availability decision, and the click's is the one that counts"
        );
        // The inert branch exists and is not a disabled-looking button.
        let src = dock_source();
        assert!(
            src.contains("aria-disabled") && src.contains("unselectable_reason(kind)"),
            "T-754: an unselectable hit must render as inert text carrying its reason"
        );
        assert!(
            src.contains("dock-left-search-hit") && src.contains("dock-left-search-hit-inert"),
            "T-697: both row shapes must be driveable from a gate"
        );
    }

    /// **THE SELECTION FILTER ONLY OFFERS NARROWINGS THAT NARROW.** A chip that would keep the whole
    /// selection selected is the T-754 mistake in a second costume, so a homogeneous selection yields
    /// no chips at all rather than a row of no-ops.
    #[test]
    fn the_selection_filter_offers_only_proper_subsets() {
        let homogeneous = vec![
            entity("a", DocKind::Slot, "One", "BLUFOR"),
            entity("b", DocKind::Slot, "Two", "BLUFOR"),
            entity("c", DocKind::Slot, "Three", "BLUFOR"),
        ];
        assert!(
            selection_facets(&homogeneous).is_empty(),
            "nothing to narrow by ⇒ no chips, not chips that do nothing"
        );
        assert!(
            selection_facets(&homogeneous[..1]).is_empty(),
            "one row is not a selection to filter"
        );
        assert!(selection_facets(&[]).is_empty());

        let mixed = vec![
            entity("a", DocKind::Slot, "One", "BLUFOR"),
            entity("b", DocKind::Slot, "Two", "OPFOR"),
            entity("c", DocKind::Vehicle, "Truck", "OPFOR"),
        ];
        let facets = selection_facets(&mixed);
        let total = mixed.len();
        for f in &facets {
            assert!(
                !f.ids.is_empty() && f.ids.len() < total,
                "{f:?} narrows nothing"
            );
            for id in &f.ids {
                assert!(
                    mixed.iter().any(|e| &e.id == id),
                    "a chip must not invent an id"
                );
            }
        }
        // BOTH axes the ticket names, and the counts are real.
        let by = |axis: &str, label: &str| {
            facets
                .iter()
                .find(|f| f.axis == axis && f.label == label)
                .unwrap_or_else(|| panic!("missing {axis} chip {label}"))
        };
        assert_eq!(by("Type", "slot").ids, ["a", "b"]);
        assert_eq!(by("Type", "vehicle").ids, ["c"]);
        assert_eq!(by("Faction", "BLUFOR").ids, ["a"]);
        assert_eq!(by("Faction", "OPFOR").ids, ["b", "c"]);
    }

    /// Rows with no faction get their OWN chip rather than being dropped — "the ones that belong to
    /// nobody" is a real narrowing, and dropping them would make the chip counts fail to sum.
    #[test]
    fn the_faction_axis_keeps_the_unfactioned() {
        let rows = vec![
            entity("a", DocKind::Comment, "Note", ""),
            entity("b", DocKind::Slot, "One", "BLUFOR"),
        ];
        let facets = selection_facets(&rows);
        let none = facets
            .iter()
            .find(|f| f.axis == "Faction" && f.label == "no faction")
            .expect("the unfactioned must be reachable");
        assert_eq!(none.ids, ["a"]);
        let sum: usize = facets
            .iter()
            .filter(|f| f.axis == "Faction")
            .map(|f| f.ids.len())
            .sum();
        assert_eq!(
            sum,
            rows.len(),
            "the faction chips must partition the selection"
        );
    }

    /// **NARROWING A SELECTION IS NOT A DOCUMENT EDIT** (T-642's line). It goes through
    /// `set_slot_selection` — the selection-only tail a folder click takes — and must never reach the
    /// history, or every filter chip would cost the author a Ctrl+Z.
    #[test]
    fn narrowing_the_selection_is_not_undoable() {
        let ops = ops_code();
        let body = only_body(&ops, "pub fn set_selection_ids");
        assert!(
            body.contains("set_slot_selection(ids)"),
            "T-697: the narrow must reuse the shipped selection-only tail"
        );
        for banned in [
            "after_local_edit",
            "remove_slots",
            "add_slot",
            "mission_history::",
        ] {
            assert!(
                !body.contains(banned),
                "T-642/T-697: narrowing a selection must not be a document edit, found {banned}"
            );
        }
        let dock = dock_code();
        let apply = only_body(&dock, "pub fn apply_selection");
        assert!(
            apply.contains("editor_ops::set_selection_ids("),
            "T-697: the chip must apply through the one seam"
        );
    }

    /// **EVERY PLACED COLLECTION IS INDEXED, OR THE SEARCH LIES.** Eight collections an author can
    /// place into; a ninth arriving without a case in `document_entities` would be silently
    /// unfindable, which is the failure the ticket is about.
    #[test]
    fn the_index_covers_every_placeable_collection() {
        let ops = ops_code();
        let body = only_body(&ops, "pub fn document_entities");
        for kind in [
            "DocKind::Slot",
            "DocKind::Vehicle",
            "DocKind::Object",
            "DocKind::Marker",
            "DocKind::Zone",
            "DocKind::Trigger",
            "DocKind::Comment",
            "DocKind::Layer",
        ] {
            assert!(
                body.contains(kind),
                "T-697: `{kind}` is not indexed — it cannot be found"
            );
        }
        // Read-only: the index must not open a transaction on the way past.
        for banned in ["after_local_edit", "core.add_", "core.set_", "core.remove_"] {
            assert!(
                !body.contains(banned),
                "T-697: the document index is a READ, found {banned}"
            );
        }
        // The selection projection is DERIVED from the index, so the two cannot disagree about an
        // entity's kind or faction.
        assert!(
            only_body(&ops, "pub fn selection_entities").contains("document_entities()"),
            "T-697: the selection filter must read the same rows the search does"
        );
    }

    /// **THE TWO NEW ROWS FIT 240 px, AND THAT IS ARITHMETIC — the T-637 rule.** The dock is width
    /// budgeted and this is the third ticket in it this run; eyeballing is what produced the silent
    /// squeeze T-637 had to go and measure. The header is NOT touched (there is no third tab — it
    /// does not fit; see the section header), so what is added up here is the two body rows.
    #[test]
    fn the_search_rows_fit_the_dock() {
        let pad = tw_len_px(DOCK_L, "p-").expect("the dock states its padding");
        // A row spans the dock's inner width, less whatever the scrolling list's scrollbar claims.
        let budget = DOCK_PX - 2.0 * pad - LIST_SCROLLBAR_PX;

        // ── the hit row: [px-1] icon │gap│ label(flex, truncates) │gap│ badge ───────────────────
        // The badge is the widest kind noun; the measured UPPERCASE ceiling is a safe bound for a
        // `lowercase` cell, which is narrower per character in every font in the stack.
        let widest_noun = [
            DocKind::Slot,
            DocKind::Vehicle,
            DocKind::Object,
            DocKind::Marker,
            DocKind::Zone,
            DocKind::Trigger,
            DocKind::Comment,
            DocKind::Layer,
        ]
        .into_iter()
        .map(|k| k.noun().chars().count())
        .max()
        .expect("eight kinds");
        let badge = widest_noun as f64 * UPPERCASE_LABEL_ADVANCE_PX;
        let furniture = HIT_ROW_PAD_PX + HIT_ICON_PX + 2.0 * HIT_GAP_PX + badge;
        let label = budget - furniture;
        assert!(
            label >= HIT_MIN_LABEL_PX,
            "T-697: the hit row's furniture wants {furniture} px of a {budget} px row, leaving \
             {label} px for the name — under the {HIT_MIN_LABEL_PX} px floor a result stops being \
             readable and the list becomes badges beside ellipses"
        );

        // ── the facet chips: they WRAP, so only a single chip has to fit on its own ─────────────
        let chip = |label: &str| {
            // `px-1.5` (12 px) + the text, worst case a three-digit count.
            12.0 + (label.chars().count() + " (999)".len()) as f64 * UPPERCASE_LABEL_ADVANCE_PX
        };
        let widest_chip = chip("vehicle");
        assert!(
            widest_chip <= budget - HIT_ROW_PAD_PX,
            "T-697: a `{}` chip wants {widest_chip} px of a {} px row",
            "vehicle",
            budget - HIT_ROW_PAD_PX
        );
        assert!(
            dock_source().contains("flex flex-wrap gap-1"),
            "T-697: the chips must WRAP — a selection straddling many factions must grow a line, \
             not overrun the column or squeeze its neighbours"
        );

        // The DOM is bounded too: a 2,000-hit query renders 200 rows and says so.
        assert!(
            MAX_DOC_HITS <= 400,
            "a 240 px column cannot usefully mount more"
        );
        assert!(
            dock_source().contains("Found {total} — showing {shown}"),
            "T-697: a truncated list must report the FULL count, or the number is a lie"
        );
    }

    /// The tree keeps its own filter and its own input (T-637), fed the FILTERED node set. The
    /// document search is ADDED beside it, not swapped for it — a search that emptied the tree it
    /// sits above would have deleted a shipped feature to add one.
    #[test]
    fn the_document_search_does_not_replace_the_layer_filter() {
        let src = dock_source();
        assert!(
            src.contains("filter_outliner(ns, &q)") && src.contains("virtual_tree("),
            "T-637: the layers tree and its filter must survive this ticket"
        );
        assert!(
            src.contains("dock-left-layers-filter") && src.contains("dock-left-search-results"),
            "T-697: one box, two surfaces — both must be driveable"
        );
    }
}
