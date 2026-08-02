//! T-661 — the shared dock-tree rendering (guides, windowed list, one-row draw), split from
//! `eden_chrome.rs`.
//!
//! `virtual_tree` is the windowed outliner both docks draw with; `guide_spans` / `chevron_or_spacer`
//! and the row-class recipes are shared by the outliner (`single_row`) and the palette
//! (`eden_dock_right::palette_rows`). Not cfg-gated (the doc-driving `on:click` bodies are wasm-gated
//! inside their closures).
#![allow(dead_code)]
use leptos::prelude::*;

use crate::outliner::{
    flatten_visible, FlatRow, LayerRow, NodeKind, OutlinerNode, VIRTUAL_SLOT_THRESHOLD,
};
use crate::ui::{badge_class, MaterialIcon};

/* ───────────────────────── T-666 — folder-click selection rules ───────────────────────── */

// SEL-LAYER-CHILDREN-001 / SEL-LAYER-DESC-001 / SEL-GROUP-ICON-001.
//
// These are the "which slots does clicking a folder select?" rules, and they read the
// **unfiltered document** — a folder's `entity_ids` as carried on the `LayerRow`
// (`editor_ops::layer_rows` → `small_maps_json()` → `editorLayersById`). That source is
// deliberately NOT `MissionDocCore::materialize()`: materialize FILTERS out the slots of a hidden
// layer (store.rs:465-473 — `continue` before any column is pushed for a slot on a hidden/inherited
// layer), which is the very lane T-715 is a pending defect on (hidden layers' slots vanish from the
// docks because `slot_rows()` feeds from the filtered SoA). The SPEC of "what this folder contains"
// is the doc's `entityIds`, not what the current view happens to show — so folder-click selects what
// the DOC says the layer holds, unfiltered, and a hidden layer still selects its slots. This does
// not touch dimming and does not fix (or regress) T-715.
//
// `select_layer_children` = the folder's DIRECT slot children (SEL-LAYER-CHILDREN-001).
// `select_layer_descendants` = every slot in the folder's whole subtree (SEL-LAYER-DESC-001,
// the modifier / second-affordance variant), walking `parentId` down through child folders.

/// A folder's DIRECT slot children — the ids listed in its own `entityIds`, in doc order.
/// Unknown `id` → empty. Reads the unfiltered doc source (see the module note above).
#[must_use]
pub(crate) fn layer_direct_slot_children(layers: &[LayerRow], id: &str) -> Vec<String> {
    layers
        .iter()
        .find(|l| l.id == id)
        .map(|l| l.entity_ids.clone())
        .unwrap_or_default()
}

/// Every slot in a folder's subtree — its own `entityIds` plus, recursively, those of every
/// descendant folder (`parentId` chain). Order is this folder's slots first, then each child
/// folder's subtree in `layers` order; a cycle-guard (`seen`) mirrors `build_outliner`'s
/// belt-and-braces against a malformed `parentId`. Reads the unfiltered doc source.
#[must_use]
pub(crate) fn layer_descendant_slots(layers: &[LayerRow], id: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    fn walk<'a>(
        layers: &'a [LayerRow],
        id: &str,
        seen: &mut std::collections::HashSet<&'a str>,
        out: &mut Vec<String>,
    ) {
        let Some(layer) = layers.iter().find(|l| l.id == id) else {
            return;
        };
        if !seen.insert(layer.id.as_str()) {
            return;
        }
        out.extend(layer.entity_ids.iter().cloned());
        for child in layers
            .iter()
            .filter(|l| l.parent_id.as_deref() == Some(layer.id.as_str()))
        {
            walk(layers, &child.id, seen, out);
        }
    }
    walk(layers, id, &mut seen, &mut out);
    out
}

/// SEL-GROUP-ICON-001 — does this folder DIRECTLY contain any slots (vs only sub-folders)?
/// Drives the distinct folder glyph: a folder holding slots reads differently from a pure
/// grouping folder. "Directly" = its own `entityIds` is non-empty (a folder whose only content is
/// sub-folders that hold slots is still a grouping folder at this level).
#[must_use]
pub(crate) fn folder_holds_slots(layers: &[LayerRow], id: &str) -> bool {
    layers
        .iter()
        .find(|l| l.id == id)
        .is_some_and(|l| !l.entity_ids.is_empty())
}

/// SEL-GROUP-ICON-001 (render side) — collect the ids of every Folder node that DIRECTLY holds at
/// least one Slot child, walking the built `OutlinerNode` tree. The windowed renderer draws from a
/// flat `FlatRow` slice with no per-row "holds slots" bit and `FlatRow` lives in `outliner.rs`
/// (not owned here), so the distinction is precomputed from the tree the dock already has and
/// looked up by id in [`single_row`]. Same "direct children only" rule as [`folder_holds_slots`]:
/// a slot filed straight in this folder counts; one filed in a sub-folder does not.
#[must_use]
fn folders_holding_slots(nodes: &[OutlinerNode]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    fn walk(nodes: &[OutlinerNode], out: &mut std::collections::HashSet<String>) {
        for n in nodes {
            if n.kind == NodeKind::Folder && n.children.iter().any(|c| c.kind == NodeKind::Slot) {
                out.insert(n.id.clone());
            }
            walk(&n.children, out);
        }
    }
    walk(nodes, &mut out);
    out
}

/// A tree row's shared recipe; depth renders as leading guide-line spans (see `guide_spans`).
pub(crate) const ROW: &str = "relative flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface";
pub(crate) const ROW_ACTIVE: &str = "relative flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm transition-colors bg-primary/20 text-primary";
/// T-177 A2 — the palette-leaf variant of [`ROW`]: adds `cursor-grab` (→ `cursor-grabbing` while
/// pressed) so hovering a placeable role advertises the drag affordance. Folders keep `cursor-pointer`
/// and outliner slots keep the plain [`ROW`] default (only palette leaves are drag-to-place).
pub(crate) const PALETTE_LEAF: &str = "relative flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface cursor-grab active:cursor-grabbing";

/// Hierarchy guide lines — continuous YouTube spines (T-178 A3/A4; supersedes T-177 L-hooks).
/// `ancestors` / `guide_ids` both have `len == depth`. Continuous `w-px` stems + mid-row stub;
/// click toggles the column owner (`guide_ids[k]`).
pub(crate) fn guide_spans(
    ancestors: &[bool],
    guide_ids: &[String],
    collapsed: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    let depth = ancestors.len();
    if depth == 0 {
        return ().into_any();
    }
    debug_assert_eq!(guide_ids.len(), depth);
    let col_left = |k: usize| format!("left:calc(0.375rem + {:.3}rem)", (k as f64) * 0.75 + 0.375);
    let mut lines: Vec<AnyView> = Vec::new();
    let make_toggle = |id: String, collapsed: RwSignal<std::collections::HashSet<String>>| {
        move |ev: web_sys::MouseEvent| {
            ev.stop_propagation();
            collapsed.update(|c| {
                if !c.remove(&id) {
                    c.insert(id.clone());
                }
            });
        }
    };
    // Ancestor spines: full-height hairline where the branch continues.
    for (k, cont) in ancestors.iter().enumerate().take(depth.saturating_sub(1)) {
        if *cont {
            let id = guide_ids.get(k).cloned().unwrap_or_default();
            let left = col_left(k);
            let on_click = make_toggle(id.clone(), collapsed);
            lines.push(
                view! {
                    <span
                        role="button"
                        tabindex="-1"
                        data-guide-toggle=id.clone()
                        aria-label=format!("Toggle {id}")
                        class="absolute inset-y-0 w-px cursor-pointer bg-white/25"
                        style=left
                        on:click=on_click
                    ></span>
                }
                .into_any(),
            );
        }
    }
    let last = depth - 1;
    let id = guide_ids.get(last).cloned().unwrap_or_default();
    let left = col_left(last);
    // Continuous stem: full height if sibling continues, else top-half only (last child).
    if ancestors[last] {
        let on_click = make_toggle(id.clone(), collapsed);
        lines.push(
            view! {
                <span
                    role="button"
                    tabindex="-1"
                    data-guide-toggle=id.clone()
                    aria-label=format!("Toggle {id}")
                    class="absolute inset-y-0 w-px cursor-pointer bg-white/25"
                    style=left.clone()
                    on:click=on_click
                ></span>
            }
            .into_any(),
        );
    } else {
        let on_click = make_toggle(id.clone(), collapsed);
        lines.push(
            view! {
                <span
                    role="button"
                    tabindex="-1"
                    data-guide-toggle=id.clone()
                    aria-label=format!("Toggle {id}")
                    class="absolute top-0 h-1/2 w-px cursor-pointer bg-white/25"
                    style=left.clone()
                    on:click=on_click
                ></span>
            }
            .into_any(),
        );
    }
    // Mid-row horizontal stub into the row content.
    let on_click = make_toggle(id.clone(), collapsed);
    lines.push(
        view! {
            <span
                role="button"
                tabindex="-1"
                data-guide-toggle=id.clone()
                aria-label=format!("Toggle {id}")
                class="absolute top-1/2 h-px w-2 cursor-pointer bg-white/25"
                style=left
                on:click=on_click
            ></span>
        }
        .into_any(),
    );
    let spacers = (0..depth)
        .map(|_| view! { <span class="w-3 shrink-0"></span> })
        .collect::<Vec<_>>();
    view! { {lines}{spacers} }.into_any()
}

/// Chevron toggle for container rows (`expand_more` open / `chevron_right` closed) — a
/// `role="button"` span so it can nest inside the row `<button>`; leaves get an alignment
/// spacer. Clicking toggles the id in `collapsed` without firing the row action.
pub(crate) fn chevron_or_spacer(
    has_children: bool,
    open: bool,
    id: &str,
    collapsed: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    if !has_children {
        return view! { <span class="size-4 shrink-0"></span> }.into_any();
    }
    let cid = id.to_string();
    let icon = if open { "expand_more" } else { "chevron_right" };
    view! {
        <span
            role="button"
            tabindex="-1"
            aria-expanded=if open { "true" } else { "false" }
            class="flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
            on:click=move |ev| {
                ev.stop_propagation();
                collapsed
                    .update(|c| {
                        if !c.remove(&cid) {
                            c.insert(cid.clone());
                        }
                    });
            }
        >
            <MaterialIcon name=icon class="block text-sm" />
        </span>
    }
    .into_any()
}

/// T-169 — window geometry. `ROW_H` is the flow height of one row (`px-1.5 py-1 text-label-sm`);
/// the spacers use it to reserve the off-screen rows. `OVERSCAN` renders a few rows past the
/// viewport each way so a fast scroll never flashes blank.
const ROW_H: f64 = 24.0;
const CONTAINER_H: f64 = 420.0;
const OVERSCAN: usize = 6;

/// T-665 — the eye + lock toggle glyphs for a Folder row (the two per-row controls the ticket
/// wires). Each is a `role="button"` span (like the chevron) so it nests inside the row `<button>`
/// and `stop_propagation`s, so clicking a glyph flips the flag WITHOUT firing the row's
/// make-active-layer action. The glyph shows this layer's OWN flag (filled = on); when an ANCESTOR
/// carries the flag (inherited, `*_effective && !own`) the icon renders muted and inert, because the
/// flag lives on the parent — you toggle it there, and this row only reflects the inherited state.
/// Trailing on the row (pushed right by a spacer in [`single_row`]).
fn layer_flag_toggles(
    id: &str,
    hidden: bool,
    locked: bool,
    hidden_effective: bool,
    locked_effective: bool,
) -> AnyView {
    // Inherited-only (ancestor set it): show muted + inert; own toggles remain live.
    let hidden_inherited = hidden_effective && !hidden;
    let locked_inherited = locked_effective && !locked;

    let eye_id = id.to_string();
    let eye_icon = if hidden {
        "visibility_off"
    } else {
        "visibility"
    };
    let eye_class = if hidden_inherited {
        "flex size-4 shrink-0 items-center justify-center rounded text-outline/50"
    } else if hidden {
        "flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-primary transition-colors hover:bg-white/10"
    } else {
        "flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
    };
    let eye_label = if hidden { "Show layer" } else { "Hide layer" };
    let eye = view! {
        <span
            role="button"
            tabindex="-1"
            data-layer-hidden=if hidden { "true" } else { "false" }
            aria-label=eye_label
            title=eye_label
            class=eye_class
            on:click=move |ev: web_sys::MouseEvent| {
                ev.stop_propagation();
                if !hidden_inherited {
                    #[cfg(target_arch = "wasm32")]
                    crate::editor_ops::set_layer_hidden(&eye_id, !hidden);
                    #[cfg(not(target_arch = "wasm32"))]
                    let _ = &eye_id;
                }
            }
        >
            <MaterialIcon name=eye_icon class="block text-sm" filled=hidden />
        </span>
    };

    let lock_id = id.to_string();
    let lock_icon = if locked { "lock" } else { "lock_open" };
    let lock_class = if locked_inherited {
        "flex size-4 shrink-0 items-center justify-center rounded text-outline/50"
    } else if locked {
        "flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-tactical-yellow transition-colors hover:bg-white/10"
    } else {
        "flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
    };
    let lock_label = if locked {
        "Unlock transforms"
    } else {
        "Lock transforms"
    };
    let lock = view! {
        <span
            role="button"
            tabindex="-1"
            data-layer-locked=if locked { "true" } else { "false" }
            aria-label=lock_label
            title=lock_label
            class=lock_class
            on:click=move |ev: web_sys::MouseEvent| {
                ev.stop_propagation();
                if !locked_inherited {
                    #[cfg(target_arch = "wasm32")]
                    crate::editor_ops::set_layer_locked(&lock_id, !locked);
                    #[cfg(not(target_arch = "wasm32"))]
                    let _ = &lock_id;
                }
            }
        >
            <MaterialIcon name=lock_icon class="block text-sm" filled=locked />
        </span>
    };

    view! {
        <span class="ml-auto flex shrink-0 items-center gap-0.5 pl-1">
            {eye}
            {lock}
        </span>
    }
    .into_any()
}

/// T-666 — per-tree authoring context threaded into [`single_row`]. Grouped into one struct so the
/// row signature stays readable: the editor-layers tree passes `authoring = true` (create/rename/
/// delete/reparent/refile controls live), the ORBAT tree passes `false` (its rows are inert / its
/// own refile is the `orbat_refile` latch). `holds_slots` is the SEL-GROUP-ICON-001 set; `rename`
/// holds the `(id, live text)` of the row being inline-renamed, `None` when nothing is being edited.
#[derive(Clone, Copy)]
struct RowAuthoring {
    /// Enable the Outliner layer-authoring affordances on this tree (editor-layers only).
    enabled: bool,
    /// SEL-GROUP-ICON-001 — folder ids that DIRECTLY hold a slot (distinct glyph).
    holds_slots: StoredValue<std::collections::HashSet<String>>,
    /// The `(layer id, live text)` of the folder being inline-renamed; `None` = no edit in flight.
    rename: RwSignal<Option<(String, String)>>,
}

/// T-666 — the hover row actions on a Folder row: **rename** (arms the inline input) and **delete**
/// (LAYER-DEL-001, behind a confirm). Two `role="button"` spans (like the chevron / flag toggles)
/// so they nest inside the row `<button>` and `stop_propagation` — clicking one never fires the
/// row's select/drop action. Hidden until the row is hovered (`opacity-0 group-hover:opacity-100`).
fn folder_row_actions(
    id: &str,
    label: &str,
    rename: RwSignal<Option<(String, String)>>,
) -> AnyView {
    let rename_id = id.to_string();
    let rename_seed = label.to_string();
    let rename_btn = view! {
        <span
            role="button"
            tabindex="-1"
            aria-label="Rename layer"
            title="Rename layer"
            class="flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-outline opacity-0 transition-opacity hover:bg-white/10 hover:text-on-surface group-hover:opacity-100"
            on:click=move |ev: web_sys::MouseEvent| {
                ev.stop_propagation();
                rename.set(Some((rename_id.clone(), rename_seed.clone())));
            }
        >
            <MaterialIcon name="edit" class="block text-sm" />
        </span>
    };

    // LAYER-DEL-001 — destructive: `remove_editor_layer` deletes the WHOLE subtree (child folders +
    // every slot filed under them). The confirm text says so, because it is not recoverable except
    // by undo. `stop_propagation` first so the click never doubles as a select/drop.
    let del_id = id.to_string();
    let del_label = label.to_string();
    let delete_btn = view! {
        <span
            role="button"
            tabindex="-1"
            aria-label="Delete layer"
            title="Delete layer and everything in it"
            class="flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-outline opacity-0 transition-opacity hover:bg-white/10 hover:text-error group-hover:opacity-100"
            on:click=move |ev: web_sys::MouseEvent| {
                ev.stop_propagation();
                #[cfg(target_arch = "wasm32")]
                {
                    let msg = format!(
                        "Delete \u{201c}{del_label}\u{201d} and everything in it?\n\nThis removes the layer, all folders nested inside it, and every unit filed in any of them. You can undo this.",
                    );
                    let ok = web_sys::window()
                        .and_then(|w| w.confirm_with_message(&msg).ok())
                        .unwrap_or(false);
                    if ok {
                        let _ = crate::editor_ops::delete_layer(&del_id);
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&del_id, &del_label);
            }
        >
            <MaterialIcon name="delete" class="block text-sm" />
        </span>
    };

    // No `ml-auto` here: the Folder row places this AFTER `layer_flag_toggles` (which carries the
    // `ml-auto` that pushes the whole trailing cluster right), so a second `ml-auto` would fight it.
    view! {
        <span class="flex shrink-0 items-center gap-0.5 pl-1">
            {rename_btn}
            {delete_btn}
        </span>
    }
    .into_any()
}

/// Render ONE flattened outliner row (no recursion — the windowed list draws a flat slice).
/// Header kinds (Unfiled / Faction) are inert; Squad is a refile drop target when `orbat_refile`;
/// Folder → active-layer + folder-click selection (T-666); Slot → select + dbl-click→Attributes
/// (SEL-ORBAT-DBL-001).
fn single_row(
    row: &FlatRow,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    collapsed: RwSignal<std::collections::HashSet<String>>,
    // T-180.6 — when true, slot pointerdown arms refile; squad pointerup completes it.
    orbat_refile: bool,
    // T-666 — layer-authoring context (create/rename/delete/reparent/refile + group icon).
    authoring: RowAuthoring,
) -> AnyView {
    let label = row.label.clone();
    let aria = row.label.clone();
    let id = row.id.clone();
    let is_leader = row.is_leader;
    // T-177/T-178 — per-row guide continuation + click-to-toggle owners.
    let ancestors: &[bool] = &row.ancestors;
    let guide_ids: &[String] = &row.guide_ids;
    // Static per build — a chevron toggle bumps `collapsed`, which re-flattens + re-renders
    // the slice (the virtual_tree Effect tracks it), so open state never goes stale.
    let open = !collapsed.with_untracked(|c| c.contains(&row.id));
    let toggle = chevron_or_spacer(row.has_children, open, &row.id, collapsed);
    let sl_badge = if is_leader {
        view! {
            <span class=badge_class("primary") data-sl-badge="true">"SL"</span>
        }
        .into_any()
    } else {
        ().into_any()
    };
    match row.kind {
        NodeKind::Unfiled => view! {
            <div class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm text-outline">
                {guide_spans(ancestors, guide_ids, collapsed)}
                {toggle}
                <MaterialIcon name="inbox" class="block text-sm" />
                <span>{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Faction => view! {
            <div class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                {guide_spans(ancestors, guide_ids, collapsed)}
                {toggle}
                <MaterialIcon name="flag" class="block text-sm" />
                <span class="truncate">{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Squad => {
            let dest = id.clone();
            if orbat_refile {
                view! {
                    <div
                        class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm text-on-surface-variant"
                        title="Drop a slot here to refile into this squad"
                        on:pointerup=move |ev| {
                            ev.stop_propagation();
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::complete_refile_onto_squad(dest.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &dest;
                        }
                    >
                        {guide_spans(ancestors, guide_ids, collapsed)}
                        {toggle}
                        <MaterialIcon name="groups" class="block text-sm" />
                        <span class="truncate">{label}</span>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm text-on-surface-variant">
                        {guide_spans(ancestors, guide_ids, collapsed)}
                        {toggle}
                        <MaterialIcon name="groups" class="block text-sm" />
                        <span class="truncate">{label}</span>
                    </div>
                }
                .into_any()
            }
        }
        NodeKind::Folder => {
            let is_active = {
                let id = id.clone();
                move || active_layer.get().as_deref() == Some(id.as_str())
            };
            // SEL-GROUP-ICON-001 — a folder that DIRECTLY holds slots gets a distinct glyph
            // (`folder_special`) from a pure grouping folder (`folder`/`folder_open`), so the tree
            // reads "this bucket has units" vs "this is just structure" at a glance.
            let holds = authoring.holds_slots.with_value(|h| h.contains(&id));
            let folder_icon = if holds {
                "folder_special"
            } else if open {
                "folder_open"
            } else {
                "folder"
            };
            // T-665 — dim a folder that is effectively hidden (own or inherited); the eye/lock
            // toggles ride at the row's trailing edge.
            let dim = if row.hidden_effective { " opacity-40" } else { "" };
            let flag_toggles = layer_flag_toggles(
                &id,
                row.hidden,
                row.locked,
                row.hidden_effective,
                row.locked_effective,
            );
            // T-666 — is THIS folder being inline-renamed?
            let editing = {
                let id = id.clone();
                let rename = authoring.rename;
                move || rename.with(|r| r.as_ref().is_some_and(|(rid, _)| rid == &id))
            };
            if authoring.enabled && editing() {
                // Inline-rename input (armed on create, or via the row's rename action). Enter /
                // blur commits through `rename_layer`; Escape cancels. Stops propagation so typing
                // never reaches the row's click/drag handlers.
                let id_input = id.clone();
                let rename = authoring.rename;
                let commit = {
                    let id = id.clone();
                    move |text: String| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = crate::editor_ops::rename_layer(&id, &text);
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = (&id, &text);
                        rename.set(None);
                    }
                };
                let commit_key = commit.clone();
                let commit_blur = commit.clone();
                return view! {
                    <div class=move || format!("{}{dim}", if is_active() { ROW_ACTIVE } else { ROW })>
                        {guide_spans(ancestors, guide_ids, collapsed)}
                        {toggle}
                        <MaterialIcon name=folder_icon class="block text-sm" />
                        <input
                            r#type="text"
                            class="min-w-0 flex-1 rounded bg-black/30 px-1 text-label-sm text-on-surface outline-none ring-1 ring-primary/60"
                            prop:value=move || rename.with(|r| r.as_ref().map_or_else(String::new, |(_, t)| t.clone()))
                            autofocus=true
                            on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()
                            on:pointerdown=|ev: web_sys::PointerEvent| ev.stop_propagation()
                            on:input=move |ev| {
                                let v = event_target_value(&ev);
                                let rid = id_input.clone();
                                rename.update(|r| { *r = Some((rid, v)); });
                            }
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                match ev.key().as_str() {
                                    "Enter" => {
                                        ev.prevent_default();
                                        let text = rename.with(|r| r.as_ref().map_or_else(String::new, |(_, t)| t.clone()));
                                        commit_key(text);
                                    }
                                    "Escape" => {
                                        ev.prevent_default();
                                        rename.set(None);
                                    }
                                    _ => {}
                                }
                            }
                            on:blur=move |_| {
                                let text = rename.with(|r| r.as_ref().map_or_else(String::new, |(_, t)| t.clone()));
                                commit_blur(text);
                            }
                        />
                    </div>
                }
                .into_any();
            }
            // T-666 — folder click: select the folder's DIRECT slot children
            // (SEL-LAYER-CHILDREN-001) AND keep the T-661 "make this the drop target" behavior. A
            // modifier (Alt or Shift) selects ALL descendants instead (SEL-LAYER-DESC-001 — the
            // "second affordance"). Both read the UNFILTERED doc (see `editor_ops` selectors).
            let id_click = id.clone();
            let authoring_on = authoring.enabled;
            let click = move |ev: web_sys::MouseEvent| {
                #[cfg(target_arch = "wasm32")]
                {
                    crate::editor_ops::set_active_layer(Some(id_click.clone()));
                    if authoring_on {
                        if ev.alt_key() || ev.shift_key() {
                            crate::editor_ops::select_layer_descendants(&id_click);
                        } else {
                            crate::editor_ops::select_layer_children(&id_click);
                        }
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&id_click, &ev, authoring_on);
            };
            // T-666 — pointer-drag reparent: arm this folder on pointerdown; a drop onto another
            // folder reparents it, and dropping onto the header root-dropzone reparents to root.
            // Same pointer idiom as ORBAT refile (the Leptos frontend has no HTML5-DnD lane).
            let id_down = id.clone();
            let id_up = id.clone();
            let authoring_dnd = authoring.enabled;
            // Hover row actions (rename / delete) — T-666. `group`/`group-hover` reveal them.
            let row_actions = if authoring.enabled {
                folder_row_actions(&id, &label, authoring.rename)
            } else {
                ().into_any()
            };
            let base = move || {
                let g = if authoring_on { " group" } else { "" };
                format!("{}{dim}{g}", if is_active() { ROW_ACTIVE } else { ROW })
            };
            view! {
                <button
                    type="button"
                    aria-label=aria
                    title=if authoring_on { "Click: drop target + select units · Alt-click: select subtree" } else { "Make this the drop target" }
                    class=base
                    on:click=click
                    on:pointerdown=move |_| {
                        if authoring_dnd {
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::begin_layer_drag(id_down.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_down;
                        }
                    }
                    on:pointerup=move |ev: web_sys::PointerEvent| {
                        if authoring_dnd {
                            ev.stop_propagation();
                            #[cfg(target_arch = "wasm32")]
                            {
                                let _ = crate::editor_ops::complete_layer_drop_onto_folder(id_up.clone());
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_up;
                        }
                    }
                >
                    {guide_spans(ancestors, guide_ids, collapsed)}
                    {toggle}
                    <MaterialIcon name=folder_icon class="block text-sm" />
                    <span class="truncate">{label}</span>
                    {flag_toggles}
                    {row_actions}
                </button>
            }
            .into_any()
        }
        NodeKind::Slot => {
            let is_sel = {
                let id = id.clone();
                move || selected.get().iter().any(|s| s == &id)
            };
            let id_dbl = id.clone();
            let id_refile = id.clone();
            let id_layer_refile = id.clone();
            let authoring_slot = authoring.enabled;
            // T-665 — a slot on a hidden layer (or hidden ancestor) renders dimmed; one on a locked
            // layer shows a trailing lock hint (the store still refuses its move — this is the
            // visible surface of that refusal in the outliner).
            let dim = if row.hidden_effective { " opacity-40" } else { "" };
            let lock_hint = if row.locked_effective {
                view! {
                    <MaterialIcon
                        name="lock"
                        class="ml-auto block shrink-0 pl-1 text-sm text-outline"
                    />
                }
                .into_any()
            } else {
                ().into_any()
            };
            view! {
                <button
                    type="button"
                    aria-label=aria
                    class=move || format!("{}{dim}", if is_sel() { ROW_ACTIVE } else { ROW })
                    on:click=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor_ops::select_slot(id.clone());
                    }
                    // T-159.26 A1 — outliner activate (native dblclick) opens Attributes,
                    // the SEL-ORBAT-DBL-001 contract.
                    on:dblclick=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor_ops::open_attributes(id_dbl.clone());
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &id_dbl;
                    }
                    on:pointerdown=move |_| {
                        if orbat_refile {
                            // T-180.6 — ORBAT tree: arm refile onto a squad.
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::begin_refile(id_refile.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_refile;
                        } else if authoring_slot {
                            // T-666 — Editor-Layers tree: arm refile of this slot into a folder
                            // (a folder-row `pointerup` completes it via `move_slot_to_layer`).
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::begin_layer_slot_drag(id_layer_refile.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_layer_refile;
                        }
                    }
                >
                    {guide_spans(ancestors, guide_ids, collapsed)}
                    {toggle}
                    <MaterialIcon name="person" class="block text-sm" />
                    <span class="truncate">{label}</span>
                    {sl_badge}
                    {lock_hint}
                </button>
            }
            .into_any()
        }
    }
}

/// T-169 — publish `window.__outlinerStats[key] = {total, rendered, threshold}` for the gate.
#[cfg(target_arch = "wasm32")]
fn set_outliner_stats(key: &str, total: usize, rendered: usize) {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else { return };
    let stats = match js_sys::Reflect::get(&win, &JsValue::from_str("__outlinerStats")) {
        Ok(v) if v.is_object() => v,
        _ => {
            let o = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__outlinerStats"), &o);
            o.into()
        }
    };
    let entry = js_sys::Object::new();
    let set = |k: &str, n: usize| {
        let _ = js_sys::Reflect::set(&entry, &JsValue::from_str(k), &JsValue::from_f64(n as f64));
    };
    set("total", total);
    set("rendered", rendered);
    set("threshold", VIRTUAL_SLOT_THRESHOLD);
    let _ = js_sys::Reflect::set(&stats, &JsValue::from_str(key), &entry);
}
#[cfg(not(target_arch = "wasm32"))]
fn set_outliner_stats(_key: &str, _total: usize, _rendered: usize) {}

/// T-169 — render a dock tree, windowed above [`VIRTUAL_SLOT_THRESHOLD`]. Below it the whole
/// flattened list renders eagerly; above it a fixed-height scroll container draws only the visible
/// slice (+ overscan) between two spacer divs, so a mission-scale tree never builds N DOM rows.
/// `stats_key` names this tree in `window.__outlinerStats`.
pub(crate) fn virtual_tree(
    nodes: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    stats_key: &'static str,
    empty_msg: &'static str,
    // T-180.6 — enable ORBAT slot→squad pointer-refile in this tree.
    orbat_refile: bool,
    // T-666 — enable Outliner layer authoring on this tree (Editor Layers dock only): folder-click
    // selection, inline rename, hover delete, pointer-drag reparent/refile, group icon.
    authoring: bool,
) -> AnyView {
    // Per-tree collapse state (T-172 B6). Starts EMPTY = fully expanded, exactly the pre-collapse
    // render — the T-169 windowing smoke's totals depend on the default-expanded boot state.
    let collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    // T-666 — the inline-rename buffer for THIS tree: `(layer id, live text)` of the folder being
    // renamed, `None` when nothing is being edited. Persists across re-renders (an RwSignal).
    let rename = RwSignal::new(None::<(String, String)>);
    // SEL-GROUP-ICON-001 — the "folder directly holds a slot" set, recomputed with the flatten.
    let holds_slots = StoredValue::new(std::collections::HashSet::<String>::new());
    let authoring_ctx = RowAuthoring {
        enabled: authoring,
        holds_slots,
        rename,
    };
    // Flatten once per doc/collapse change (O(n), like the mutation itself); the scroll path only
    // re-slices. Created ONCE per mount (this fn is called outside any reactive closure), so the
    // Effect never leaks — it re-runs on `nodes`/`collapsed` change, and the render `move ||`
    // re-slices on `rev`/scroll.
    let flat = StoredValue::new(Vec::<FlatRow>::new());
    let rev = RwSignal::new(0u64);
    Effect::new(move |_| {
        let ns = nodes.get();
        if authoring {
            holds_slots.set_value(folders_holding_slots(&ns));
            // LAYER-CREATE-001 — a create just happened → open that new folder's inline rename.
            // Consumed once (the ops latch clears on read), so a later flatten won't re-arm it.
            #[cfg(target_arch = "wasm32")]
            if let Some(new_id) = crate::editor_ops::take_rename_armed() {
                // Seed the buffer with the just-minted "New Layer N" name so a blur with no typing
                // keeps it (rename rejects a blank), and the caret lands on real text to overwrite.
                let seed = ns
                    .iter()
                    .find(|n| n.id == new_id)
                    .map_or_else(String::new, |n| n.label.clone());
                rename.set(Some((new_id, seed)));
            }
        }
        let f = collapsed.with(|c| flatten_visible(&ns, c));
        flat.set_value(f);
        rev.update(|r| *r = r.wrapping_add(1));
    });
    let scroll_top = RwSignal::new(0.0_f64);
    (move || {
        rev.track(); // re-render the slice when the tree changes
        let st = scroll_top.get();
        flat.with_value(|f| {
            let total = f.len();
            if total == 0 {
                set_outliner_stats(stats_key, 0, 0);
                return view! { <p class="text-label-sm text-outline">{empty_msg}</p> }.into_any();
            }
            if total <= VIRTUAL_SLOT_THRESHOLD {
                set_outliner_stats(stats_key, total, total);
                return view! {
                    <div>
                        {f
                            .iter()
                            .map(|r| single_row(r, selected, active_layer, collapsed, orbat_refile, authoring_ctx))
                            .collect::<Vec<_>>()}
                    </div>
                }
                .into_any();
            }
            let per_screen = (CONTAINER_H / ROW_H).ceil() as usize;
            let start = ((st / ROW_H).floor() as usize).saturating_sub(OVERSCAN);
            let end = (start + per_screen + 2 * OVERSCAN).min(total);
            set_outliner_stats(stats_key, total, end - start);
            let top = start as f64 * ROW_H;
            let bottom = (total - end) as f64 * ROW_H;
            let rows: Vec<AnyView> = f[start..end]
                .iter()
                .map(|r| single_row(r, selected, active_layer, collapsed, orbat_refile, authoring_ctx))
                .collect();
            view! {
                <div
                    class="overflow-y-auto"
                    style=format!("height:{CONTAINER_H}px")
                    on:scroll=move |ev| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                                scroll_top.set(el.scroll_top() as f64);
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                >
                    <div style=format!("height:{top}px")></div>
                    {rows}
                    <div style=format!("height:{bottom}px")></div>
                </div>
            }
            .into_any()
        })
    })
    .into_any()
}

#[cfg(test)]
mod tests {
    //! T-666 — the folder-click SELECTION RULES + the group-icon rule, native (this module is not
    //! wasm-gated, so `cargo test -p website-frontend` runs it). These pin the pure logic the
    //! `editor_ops` selectors call (`layer_direct_slot_children` / `layer_descendant_slots` /
    //! `folders_holding_slots`) — the part that decides WHICH slots a folder click selects.
    //!
    //! The ops WRAPPERS (`create_layer` / `rename_layer` / `delete_layer` / `reparent_layer` /
    //! `refile_slot_to_layer`) and the DOCK CONTROLS are `#![cfg(target_arch = "wasm32")]`, so they
    //! cannot run under this native harness. Two things stand in for them, both real gates, not
    //! prose: (1) the wrappers are thin pass-throughs onto the SHIPPED, ALREADY-TESTED core
    //! mutators — `add_editor_layer` / `rename_editor_layer` / `remove_editor_layer` (subtree +
    //! reseed) / `reparent_editor_layer` (cycle-guarded) / `move_slot_to_layer` — whose semantics
    //! AND undo-in-one-step are pinned in `map-engine-core`'s store.rs tests (e.g.
    //! `remove_editor_layer_reseeds_when_subtree_is_all_layers`), and each core mutator commits a
    //! single transaction (one undo step); (2) the [`source_pins`] tests below read the source and
    //! assert every control + tail is wired (create + refresh_docks tail, delete-confirm text,
    //! inline rename, root dropzone, the unfiltered-doc selection source).

    use super::*;
    use crate::outliner::{build_outliner, LayerRow, NodeKind, SlotRow};

    fn slot(id: &str) -> SlotRow {
        SlotRow {
            id: id.to_string(),
            role: "Rifleman".to_string(),
        }
    }
    fn layer(id: &str, parent: Option<&str>, ents: &[&str]) -> LayerRow {
        LayerRow {
            id: id.to_string(),
            name: format!("{id}-name"),
            parent_id: parent.map(str::to_string),
            entity_ids: ents.iter().map(|s| (*s).to_string()).collect(),
            hidden: false,
            locked: false,
        }
    }

    /// A three-level fixture: root(a1,a2) → child(b1) → grandchild(c1).
    fn nested() -> Vec<LayerRow> {
        vec![
            layer("root", None, &["a1", "a2"]),
            layer("child", Some("root"), &["b1"]),
            layer("grand", Some("child"), &["c1"]),
        ]
    }

    // ── SEL-LAYER-CHILDREN-001 ────────────────────────────────────────────────────────────────

    #[test]
    fn direct_children_are_own_entity_ids_only() {
        let layers = nested();
        // The folder's DIRECT slot children = its own `entityIds`, in order — NOT the subtree.
        assert_eq!(
            layer_direct_slot_children(&layers, "root"),
            vec!["a1", "a2"]
        );
        assert_eq!(layer_direct_slot_children(&layers, "child"), vec!["b1"]);
        assert_eq!(layer_direct_slot_children(&layers, "grand"), vec!["c1"]);
    }

    #[test]
    fn direct_children_unknown_layer_is_empty() {
        assert!(layer_direct_slot_children(&nested(), "nope").is_empty());
    }

    // ── SEL-LAYER-DESC-001 ────────────────────────────────────────────────────────────────────

    #[test]
    fn descendants_walk_the_whole_subtree() {
        let layers = nested();
        // root's descendants = root's own slots + child's + grandchild's (recursion is the point).
        assert_eq!(
            layer_descendant_slots(&layers, "root"),
            vec!["a1", "a2", "b1", "c1"]
        );
        // child's subtree stops above root but still reaches the grandchild.
        assert_eq!(layer_descendant_slots(&layers, "child"), vec!["b1", "c1"]);
        // a leaf folder's subtree is just itself.
        assert_eq!(layer_descendant_slots(&layers, "grand"), vec!["c1"]);
    }

    /// FIRED-ONCE (perturb / fail / restore): the descendant walk MUST recurse, and this is the
    /// rule the destructive delete (`remove_editor_layer` subtree) and SEL-LAYER-DESC-001 both
    /// stand on. To prove the assertion has teeth, PERTURB the fixture to break the parent chain
    /// (grandchild reparented off the subtree), assert the walk then MISSES `c1` (the FAIL the
    /// test would catch if the logic ever stopped recursing), then RESTORE the correct chain and
    /// assert `c1` is back. If `layer_descendant_slots` were a direct-children-only lookup, the
    /// FIRST (perturbed) and correct cases would be identical and this test could not tell them
    /// apart — so the negative arm is what makes the recursion gate real.
    #[test]
    fn descendants_recursion_gate_fires() {
        // Correct chain: grandchild reachable → c1 present.
        let ok = layer_descendant_slots(&nested(), "root");
        assert!(
            ok.contains(&"c1".to_string()),
            "baseline reaches grandchild"
        );

        // PERTURB: detach `grand` from the subtree (parent → an unrelated root).
        let mut perturbed = nested();
        perturbed.push(layer("other", None, &[]));
        for l in &mut perturbed {
            if l.id == "grand" {
                l.parent_id = Some("other".to_string());
            }
        }
        let broken = layer_descendant_slots(&perturbed, "root");
        assert!(
            !broken.contains(&"c1".to_string()),
            "PERTURBED: with the chain cut, the subtree walk must NOT reach the grandchild's slot \
             — this is the failure the recursion gate exists to prevent"
        );
        // still reaches the intact level.
        assert!(broken.contains(&"b1".to_string()), "child level intact");

        // RESTORE: the intact fixture reaches the grandchild again.
        let restored = layer_descendant_slots(&nested(), "root");
        assert!(
            restored.contains(&"c1".to_string()),
            "RESTORE: grandchild back"
        );
    }

    #[test]
    fn descendants_cycle_guarded() {
        // A malformed parentId cycle (root ↔ child) must terminate, not hang.
        let layers = vec![
            layer("root", Some("child"), &["a1"]),
            layer("child", Some("root"), &["b1"]),
        ];
        let got = layer_descendant_slots(&layers, "root");
        assert!(got.contains(&"a1".to_string()) && got.contains(&"b1".to_string()));
    }

    // ── The selection reads the UNFILTERED doc (the T-715 non-regression contract) ────────────

    #[test]
    fn selection_reads_unfiltered_doc_hidden_layer_still_selects() {
        // A HIDDEN layer's slots are dropped by `materialize()` (and so by `slot_rows`), which is
        // the T-715 defect lane. The selection helpers read `LayerRow.entity_ids` (from
        // `small_maps_json`, unfiltered), so a hidden folder STILL selects its slots — folder-click
        // selects what the DOC contains, not what the filtered view shows.
        let mut layers = nested();
        for l in &mut layers {
            if l.id == "child" {
                l.hidden = true; // child (and its grandchild) would vanish from the render SoA
            }
        }
        // Direct + descendant selection are unaffected by the hidden flag.
        assert_eq!(layer_direct_slot_children(&layers, "child"), vec!["b1"]);
        assert_eq!(
            layer_descendant_slots(&layers, "root"),
            vec!["a1", "a2", "b1", "c1"],
            "a hidden sub-layer's slots are still part of what the parent folder contains"
        );
    }

    // ── SEL-GROUP-ICON-001 ────────────────────────────────────────────────────────────────────

    #[test]
    fn group_icon_distinguishes_slot_holders_from_grouping_folders() {
        // `parent` groups only sub-folders; `leaf` directly holds a slot.
        let layers = vec![
            layer("parent", None, &[]),
            layer("leaf", Some("parent"), &["s1"]),
        ];
        assert!(
            !folder_holds_slots(&layers, "parent"),
            "pure grouping folder"
        );
        assert!(folder_holds_slots(&layers, "leaf"), "directly holds a slot");

        // The render-side set (built from the OutlinerNode tree) agrees: only `leaf` is flagged.
        let tree = build_outliner(&layers, &[slot("s1")]);
        let holders = folders_holding_slots(&tree);
        assert!(holders.contains("leaf"));
        assert!(!holders.contains("parent"));
    }

    #[test]
    fn group_icon_set_walks_nested_folders() {
        // Nested render set must find a slot-holder at any depth (walk, not top-level only).
        let layers = nested(); // root & child & grand all hold ≥1 slot
        let tree = build_outliner(&layers, &[slot("a1"), slot("a2"), slot("b1"), slot("c1")]);
        let holders = folders_holding_slots(&tree);
        for id in ["root", "child", "grand"] {
            assert!(holders.contains(id), "{id} directly holds a slot");
        }
    }

    // ── Source-inspection pins for the wasm-only controls + the ops tails ─────────────────────

    mod source_pins {
        //! These read the SOURCE of the three owned files and assert each control is wired, since
        //! the controls are `#![cfg(target_arch = "wasm32")]` and cannot be exercised natively.
        //! A pin fails loudly if a rename drops a call the ticket requires.

        const OPS: &str = include_str!("editor_ops.rs");
        const TREE: &str = include_str!("eden_tree.rs");
        const DOCK: &str = include_str!("eden_dock_left.rs");

        /// Every layer-authoring wrapper rides `after_local_edit()` — the tail that calls
        /// `refresh_docks()` (via `refresh_signals`). Pin the pairing so a wrapper can't ship
        /// mutating the core without refreshing the docks.
        #[test]
        fn wrappers_call_core_then_after_local_edit() {
            for (wrapper, core_call) in [
                ("pub fn create_layer", "add_editor_layer"),
                ("pub fn rename_layer", "rename_editor_layer"),
                ("pub fn delete_layer", "remove_editor_layer"),
                ("pub fn reparent_layer", "reparent_editor_layer"),
                ("pub fn refile_slot_to_layer", "move_slot_to_layer"),
            ] {
                assert!(OPS.contains(wrapper), "missing wrapper {wrapper}");
                assert!(
                    OPS.contains(core_call),
                    "{wrapper} must ride the shipped core mutator {core_call}"
                );
            }
            // The single refresh tail every wrapper funnels through.
            assert!(OPS.contains("mission_history::after_local_edit"));
        }

        /// The selection helpers read the UNFILTERED doc (`layer_rows` → `entity_ids`), never
        /// `materialize()` — the T-715 non-regression contract, stated in-code.
        #[test]
        fn selection_uses_layer_rows_not_materialize() {
            assert!(OPS.contains("select_layer_children"));
            assert!(OPS.contains("select_layer_descendants"));
            assert!(
                OPS.contains("layer_direct_slot_children")
                    && OPS.contains("layer_descendant_slots"),
                "selectors must call the unfiltered-doc helpers"
            );
            // The selectors feed off `layer_rows(core)` (small_maps_json / editorLayersById),
            // which carries every slot regardless of hidden state — unlike `slot_rows`/materialize.
            assert!(OPS.contains("layer_rows(core)"));
        }

        /// LAYER-CREATE-001 — the "+" create button and the root dropzone live in the dock header.
        #[test]
        fn dock_has_create_button_and_root_dropzone() {
            assert!(
                DOCK.contains("create_layer"),
                "the + button calls create_layer"
            );
            assert!(DOCK.contains("New layer"), "the + button is labelled");
            assert!(
                DOCK.contains("complete_layer_drop_onto_root"),
                "the header is a root dropzone"
            );
            assert!(
                DOCK.contains("cancel_layer_drag"),
                "a stray drag is cleared"
            );
            // The authoring tree is enabled (last virtual_tree arg true on this dock).
            assert!(DOCK.contains("virtual_tree"));
        }

        /// LAYER-DEL-001 — delete is destructive-subtree and the confirm text SAYS SO.
        #[test]
        fn delete_confirm_names_the_subtree() {
            assert!(
                TREE.contains("delete_layer"),
                "delete action calls delete_layer"
            );
            assert!(
                TREE.contains("confirm_with_message"),
                "delete is behind a confirm"
            );
            // The confirm text must warn it removes nested folders + every unit filed under them.
            assert!(
                TREE.contains("folders nested inside it") && TREE.contains("every unit filed"),
                "the confirm must state the whole subtree is destroyed"
            );
        }

        /// Inline rename + the two folder-click selection modifiers are wired in the tree.
        #[test]
        fn tree_has_inline_rename_and_dual_selection() {
            assert!(
                TREE.contains("rename_layer"),
                "inline rename commits via rename_layer"
            );
            assert!(
                TREE.contains("select_layer_children") && TREE.contains("select_layer_descendants"),
                "folder click selects children; a modifier selects descendants"
            );
            assert!(
                TREE.contains("alt_key()") && TREE.contains("shift_key()"),
                "the descendant modifier is Alt/Shift"
            );
            // Pointer-drag reparent/refile (the current TreeView-DnD idiom).
            assert!(TREE.contains("begin_layer_drag") && TREE.contains("begin_layer_slot_drag"));
            assert!(TREE.contains("complete_layer_drop_onto_folder"));
            // SEL-GROUP-ICON-001 — the distinct slot-holder glyph.
            assert!(TREE.contains("folder_special"));
        }
    }
}
