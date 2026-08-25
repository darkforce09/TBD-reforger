//! T-661 — the shared dock-tree rendering (guides, windowed list, one-row draw), split from
//! `eden_chrome.rs`.
//!
//! `virtual_tree` is the windowed outliner both docks draw with; `guide_spans` / `chevron_or_spacer`
//! and the row-class recipes are shared by the outliner (`single_row`) and the palette
//! (`eden_dock_right::palette_rows`). Not cfg-gated (the doc-driving `on:click` bodies are wasm-gated
//! inside their closures).
#![allow(dead_code)]
use leptos::prelude::*;

use crate::core::ui::MaterialIcon;
use crate::editor::panels::outliner::{
    flatten_visible, FlatRow, LayerRow, NodeKind, OutlinerNode, VIRTUAL_SLOT_THRESHOLD,
};

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

// T-668 — the tree rows speak the one state vocabulary (`eden_layout`): an idle row wears
// [`crate::editor::layout::HOVER_FILL`] (`transition-colors hover:bg-white/10 hover:text-on-surface`),
// a selected/active row wears [`crate::editor::layout::TOGGLED_PLATE`] (`bg-primary/20 text-primary
// border-t border-background/60`). These consts are the pre-merged literals of `base + recipe` — the
// same "the recipe can't be `cn`'d into a `const`" idiom `eden_layout`'s STRIP/DOCK_* use — and
// `t668_tree_rows_speak_the_vocabulary` pins that each literal still carries its recipe's tokens, so
// a hand-edit that dropped the top border (making a selected row indistinguishable from a hovered
// one) fails there. The border is the load-bearing half: before T-668 `ROW_ACTIVE` had none, so a
// selected row and a hovered row differed only by tint, not by construction.

// ── T-637 — ONE row geometry, and it is EXPLICIT ─────────────────────────────────────────────────
//
// The pitch used to be implicit: `px-1.5 py-1` around a 16 px `text-label-sm` line box, which is 24
// and which `ROW_H` restated as a magic `24.0`. Two things were wrong with that.
//
//   (1) **It was not actually one number.** `ROW_ACTIVE` carries a `border-t` that `ROW` does not,
//       and under `box-sizing: border-box` an auto-height row grows by that border — so a SELECTED
//       row was 25 px in a tree whose virtual spacers reserved 24. The drift was invisible at eight
//       rows and a creeping scroll-position error at eight hundred. Stating `h-4` fixes the height
//       INSIDE the border box, so every recipe is the same height by construction.
//   (2) **24 px is not a dense tree.** Eden's outliner runs at a 15.8 px pitch and that is how it
//       fits a mission's worth of structure in 240 px; ours showed a title, one row, and 900 px of
//       nothing. `h-4` is 16.
//
// The four ad-hoc row kinds (Unfiled/Faction/Squad/Comment) used to inline their own copy of the
// geometry, so "the tree's row height" lived in seven string literals. They all compose [`ROW_GEOM`]
// now, and `t637_one_dense_row_geometry` pins that `ROW_H` IS what the shared class says.

/// T-637 — the geometry EVERY tree row shares, and the single place the 16 px pitch is stated.
/// `h-4` is [`ROW_H`]; `items-center` centres the 16 px chevron/glyph cells inside it.
pub(crate) const ROW_GEOM: &str =
    "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm";

/// A tree row's shared recipe (idle): [`ROW_GEOM`] + [`crate::editor::layout::HOVER_FILL`]. Depth
/// renders as leading guide-line spans (see `guide_spans`).
pub(crate) const ROW: &str = "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface";
/// A tree row's SELECTED/active recipe: [`ROW_GEOM`] + [`crate::editor::layout::TOGGLED_PLATE`] (the
/// lighter primary plate PLUS the 1px dark top border that makes it distinct-by-construction from a
/// hovered [`ROW`]). The border is inside the `h-4` box, so this row is not a pixel taller than [`ROW`].
pub(crate) const ROW_ACTIVE: &str = "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm bg-primary/20 text-primary border-t border-background/60";
/// T-803 (fold a) — a FOLDER row that is the **active drop target** (the layer the next placement /
/// comment lands in, `editor_ops::active_layer`). This is a DIFFERENT STATE from selection and must
/// read differently (state-vocabulary rule): selection wears [`ROW_ACTIVE`] (the primary plate + dark
/// top border); the drop target wears the *tertiary* plate + an INSET RING (not a top border). The two
/// share no distinguishing token — different hue (`tertiary` vs `primary`) AND a ring vs a border — so
/// a selected slot and the drop-target folder never read as the same thing, and `is_active` (drop
/// target) is never confused with `is_sel` (selection). The ring is a box-shadow, so like [`ROW`] it
/// adds no height: this recipe is still `h-4` and survives windowing at the same pitch. A small
/// `my_location` chip (see the Folder row view) rides this state as the non-colour half of the cue.
/// `t803_drop_target_reads_differently` pins the two folder sites onto this const and the
/// class-distinctness (neither string contains the other's distinguishing token).
pub(crate) const ROW_DROP_TARGET: &str = "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm bg-tertiary/15 text-tertiary ring-1 ring-inset ring-tertiary/50";
/// T-177 A2 — the palette-leaf variant of [`ROW`]: adds `cursor-grab` (→ `cursor-grabbing` while
/// pressed) so hovering a placeable role advertises the drag affordance. Folders keep `cursor-pointer`
/// and outliner slots keep the plain [`ROW`] default (only palette leaves are drag-to-place). Same
/// [`crate::editor::layout::HOVER_FILL`] as [`ROW`].
pub(crate) const PALETTE_LEAF: &str = "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface cursor-grab active:cursor-grabbing";
/// T-637 — the non-interactive row kinds (Squad / Comment headers): [`ROW_GEOM`] at the muted rest
/// weight. They are `<div>`s, not buttons, but they occupy the same 16 px pitch — a group header that
/// was a different height from its children is what made the tree read as ragged.
pub(crate) const ROW_STATIC: &str =
    "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm text-on-surface-variant";
/// T-637 — the "Unfiled" pseudo-root's row: [`ROW_GEOM`] at the faintest weight (it is a virtual
/// bucket, not a doc layer).
pub(crate) const ROW_UNFILED: &str =
    "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm text-outline";
/// T-637 — an ORBAT faction header: [`ROW_GEOM`] plus the small-caps treatment that marks a section.
pub(crate) const ROW_FACTION: &str = "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant";
/// T-637 — the ORBAT squad-leader badge, sized for the dense row. `ui::badge_class` is the page-level
/// pill (`px-2 py-0.5` ⇒ 22 px with its border) and it burst out of a 16 px row; this is the same
/// primary tint at `h-3` with `leading-none`, so the badge sits INSIDE the row instead of setting its
/// height.
pub(crate) const ROW_BADGE: &str = "inline-flex h-3 shrink-0 items-center rounded border border-primary/30 bg-primary/10 px-1 text-label-sm leading-none text-primary";

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
            <MaterialIcon name=icon class="block text-sm leading-none" />
        </span>
    }
    .into_any()
}

/// T-169 — window geometry. `ROW_H` is the flow height of one row; the spacers use it to reserve the
/// off-screen rows, so if it disagrees with what the row class actually renders the scrollbar lies
/// and a fast scroll lands on the wrong row. `OVERSCAN` renders a few rows past the viewport each way
/// so a fast scroll never flashes blank.
///
/// T-637 — 24 → 16, and it is no longer a magic number: it is [`ROW_GEOM`]'s `h-4` read back through
/// the Tailwind spacing scale, pinned by `t637_one_dense_row_geometry`. Denser is the point (Eden's
/// outliner runs at ~15.8 px), but the pin is the durable half — the two used to be able to drift.
const ROW_H: f64 = 16.0;
/// T-769 — fallback height used only until the live scroller's `clientHeight` is measured (and on
/// native, where there is no layout). The windowed scroller itself is `h-full min-h-0` inside the
/// dock's `flex-1` tree region: a fixed 420 px budget left ~538 px of void on large missions after
/// T-637 handed that region to the tree. The wave-118 claim that `h-full` would silently stop the
/// T-169 windowing gate from testing was false — `smoke_virtual_outliner` also pins a rendered cap
/// (and now pins the measured-height formula), so the two move together here.
const CONTAINER_H_FALLBACK: f64 = 420.0;
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
                    crate::editor::state::operations::set_layer_hidden(&eye_id, !hidden);
                    #[cfg(not(target_arch = "wasm32"))]
                    let _ = &eye_id;
                }
            }
        >
            <MaterialIcon name=eye_icon class="block text-sm leading-none" filled=hidden />
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
                    crate::editor::state::operations::set_layer_locked(&lock_id, !locked);
                    #[cfg(not(target_arch = "wasm32"))]
                    let _ = &lock_id;
                }
            }
        >
            <MaterialIcon name=lock_icon class="block text-sm leading-none" filled=locked />
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
/// own refile is the `orbat_refile` latch). `holds_slots` is the SEL-GROUP-ICON-001 set.
///
/// T-811 — rename state is TWO signals, not one `(id, draft)` tuple: the tree list may track
/// [`RowAuthoring::renaming`] (which folder is open) so the input mounts/unmounts, but it must
/// NEVER read [`RowAuthoring::rename_draft`]. A draft round-trip through the list-tracked signal
/// remounts the input on every keystroke (wave200 F2 / the T-812 remount trap); the NodeRef
/// `on_load` focus+select then re-selects the whole field and each next char replaces the name.
#[derive(Clone, Copy)]
struct RowAuthoring {
    /// Enable the Outliner layer-authoring affordances on this tree (editor-layers only).
    enabled: bool,
    /// SEL-GROUP-ICON-001 — folder ids that DIRECTLY hold a slot (distinct glyph).
    holds_slots: StoredValue<std::collections::HashSet<String>>,
    /// Which folder id is being inline-renamed; `None` = no edit in flight. List-safe.
    renaming: RwSignal<Option<String>>,
    /// Live draft text for the open rename. Input-only — do not read from the list render.
    rename_draft: RwSignal<String>,
}

/// T-666 — the hover row actions on a Folder row: **rename** (arms the inline input) and **delete**
/// (LAYER-DEL-001, behind a confirm). Two `role="button"` spans (like the chevron / flag toggles)
/// so they nest inside the row `<button>` and `stop_propagation` — clicking one never fires the
/// row's select/drop action. Hidden until the row is hovered (`opacity-0 group-hover:opacity-100`).
fn folder_row_actions(
    id: &str,
    label: &str,
    renaming: RwSignal<Option<String>>,
    rename_draft: RwSignal<String>,
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
                // Seed draft BEFORE arming `renaming` so the first list paint already has text.
                rename_draft.set(rename_seed.clone());
                renaming.set(Some(rename_id.clone()));
            }
        >
            <MaterialIcon name="edit" class="block text-sm leading-none" />
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
                        let _ = crate::editor::state::operations::delete_layer(&del_id);
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&del_id, &del_label);
            }
        >
            <MaterialIcon name="delete" class="block text-sm leading-none" />
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

/* ═════════ T-784 — WHICH OUTLINER ROWS ROUTE, AND WHO ANSWERS THAT ═══════════════════════════════
 *
 * THE AFFORDANCE INVARIANT (wave 129, and it has been violated in both directions): **a row is
 * clickable IFF clicking it does something**, and where the click is the shipped click-to-select
 * router the ONE answer is `validation_panel::subject_id_routes` — an `Rc` of the very resolution
 * `route_select_by_subject_id` runs. A kind list cannot track a router (T-754 painted an affordance
 * over a dead click; wave-129 RV-1 painted `aria-disabled` over a click that WOULD have worked),
 * and a fallback for "no probe registered" is not a safety net, it is the bug — no probe means no
 * router to click into, and `false` is the honest answer.
 *
 * The Comment row is the row this ticket makes live. It was `ROW_STATIC`: drag-into-folder and
 * dblclick-to-edit worked, but NO click path reached the selection, so the T-781 composition lane
 * had no reachable entrance. It is now the `eden_settings`/`eden_dock_left` shape — routable ⇒ a
 * real `<button>`, refused ⇒ a non-focusable `aria-disabled` element that says why — and the
 * boolean it branches on is the probe's, not a kind test.
 */

/// **The id a row hands to the click-to-select ROUTER**, or `None` for a row whose click is a
/// different verb entirely.
///
/// The match is over the tree's OWN dispatch — which rows delegate their click to the router — and
/// never over which kinds the router can resolve. That second question is the router's alone and is
/// asked, per id, by [`row_routes`]. Keeping the two apart is the whole of the wave-129 rule: this
/// function may be read as a list, but it is not a list *of the router's reach*, which is the list
/// that goes stale.
///
///   * `Comment` — routes. Its id is a `commentsById` key and `mission_editor::route_target` grew
///     the arm that resolves it (T-784); the selection it lands in is the one `save_composition`
///     reads, which is what makes a note composable with entities.
///   * `Slot` — does NOT route. A slot row selects through the Outliner's own
///     `editor_ops::select_slot`, which needs no router, cannot fail, and deliberately does not move
///     the camera (React: "selecting a slot selects it globally, no auto camera move"). Routing it
///     would make the core selection path depend on a probe registered at mount and fly the camera
///     on every outliner click — two regressions to buy a uniformity nothing needs.
///   * `Folder` — does NOT route. Its click makes it the drop target and selects its CHILDREN
///     (T-666); its own id is not a selection subject.
///   * `Unfiled` / `Faction` / `Squad` — containers and headers, not entities. Nothing to route.
///
/// Exhaustive on purpose: a new `NodeKind` cannot compile until this file states which side it is
/// on, so a row kind can never default into "silently unclickable".
#[must_use]
pub(crate) fn row_router_subject(kind: NodeKind, id: &str) -> Option<&str> {
    match kind {
        NodeKind::Comment => Some(id),
        NodeKind::Slot
        | NodeKind::Folder
        | NodeKind::Unfiled
        | NodeKind::Faction
        | NodeKind::Squad => None,
    }
}

/// **Would clicking this row select the thing it names?** — the router's own resolution, asked
/// before the affordance is drawn.
///
/// The peer of `eden_dock_left::hit_is_routable`, `eden_settings::owner_is_routable` and
/// `validation_panel::finding_is_routable`, and the same `Rc` behind all four. No probe registered
/// (the native shell, pre-mount) ⇒ `false`, and that is correct rather than pessimistic.
#[must_use]
pub(crate) fn row_routes(kind: NodeKind, id: &str) -> bool {
    row_router_subject(kind, id)
        .is_some_and(crate::editor::panels::validation_panel::subject_id_routes)
}

/// Why an unroutable row is inert, in words, rendered as its `title` — the answer available exactly
/// where the click would have been. It names the refusal it actually got, never a kind ban: the
/// dock-left row's old "resolves slots and vehicles only" sentence became a lie the moment the
/// router grew an arm, and this one cannot.
#[must_use]
pub(crate) fn inert_row_reason() -> &'static str {
    "Not selectable right now — the editor's click-to-select router resolves nothing for this row"
}

/// T-651 (`PLACE-COMMENT-001`) / **T-784 — the COMMENT row, and IT SELECTS.**
///
/// It used to be a `ROW_STATIC` `<div>` with no click path to the selection at all. T-651 reasoned
/// that a comment id is a `commentsById` key and so must not be routed into `select_slot` /
/// `open_attributes` — true, that modal reads the slot SoA and would open blank — and concluded
/// "therefore no click". That left the row unselectable, unreachable by Delete, and, once T-781 made
/// comments composable, closed the only entrance to the composition lane. The right answer was never
/// `select_slot`; it is the shipped click-to-select ROUTER, which `mission_editor::route_target`
/// now resolves comments through.
///
/// The shape follows [`row_routes`] and nothing else: routable ⇒ a real `<button>` (a keyboard user
/// can activate it, `ROW_ACTIVE` while selected), refused ⇒ the old inert element, non-focusable,
/// `aria-disabled`, carrying [`inert_row_reason`]. Both shapes keep the two affordances T-651 did
/// ship — drag-into-a-folder and dblclick-to-edit — because neither depends on the router.
///
/// Its own function, not an arm of `single_row`'s kind match, so `t784_comment_row_selects`'s shape
/// pin can examine exactly this row (`exactly one <button>` is meaningless over a match that also
/// renders the Folder and Slot buttons).
fn comment_row(
    row: &FlatRow,
    selected: RwSignal<Vec<String>>,
    collapsed: RwSignal<std::collections::HashSet<String>>,
    toggle: AnyView,
    authoring_enabled: bool,
) -> AnyView {
    let id = row.id.clone();
    let label = row.label.clone();
    let aria = row.label.clone();
    let tip = row.tooltip.clone();
    let guides = guide_spans(&row.ancestors, &row.guide_ids, collapsed);
    let routes = row_routes(row.kind, &id);
    let id_drag = id.clone();
    let id_dbl = id.clone();
    let id_click = id.clone();
    let is_sel = {
        let id = id.clone();
        move || selected.get().iter().any(|s| s == &id)
    };
    // Double-click opens the COMMENT EDITOR — a comment's Attributes. Deliberately NOT
    // `open_attributes` (SEL-ORBAT-DBL-001's target): that modal reads the slot SoA, which a comment
    // is never in, so it would open blank and write nothing.
    let on_dbl = move |_: web_sys::MouseEvent| {
        #[cfg(target_arch = "wasm32")]
        crate::editor::state::operations::open_comment_editor(id_dbl.clone());
        #[cfg(not(target_arch = "wasm32"))]
        let _ = &id_dbl;
    };
    let on_down = move |_: web_sys::PointerEvent| {
        if authoring_enabled {
            #[cfg(target_arch = "wasm32")]
            crate::editor::state::operations::begin_layer_comment_drag(id_drag.clone());
            #[cfg(not(target_arch = "wasm32"))]
            let _ = &id_drag;
        }
    };
    if routes {
        view! {
            <button
                type="button"
                title=tip
                aria-label=aria
                class=move || (if is_sel() { ROW_ACTIVE } else { ROW }).to_string()
                // THE ROUTER — the same function the dock-left search hit and the validation-panel
                // finding row click, and the same resolution `row_routes` asked above. One decision,
                // both ends of it; no second selection path was invented for this row.
                on:click=move |_| {
                    let _ = crate::editor::panels::validation_panel::route_select_by_subject_id(&id_click);
                }
                on:dblclick=on_dbl
                on:pointerdown=on_down
            >
                {guides}
                {toggle}
                <MaterialIcon name="sticky_note_2" class="block text-sm leading-none" />
                <span class="truncate">{label}</span>
            </button>
        }
        .into_any()
    } else {
        view! {
            <div
                class=ROW_STATIC
                title=inert_row_reason()
                aria-label=aria
                aria-disabled="true"
                on:dblclick=on_dbl
                on:pointerdown=on_down
            >
                {guides}
                {toggle}
                <MaterialIcon name="sticky_note_2" class="block text-sm leading-none" />
                <span class="truncate">{label}</span>
            </div>
        }
        .into_any()
    }
}

/// Render ONE flattened outliner row (no recursion — the windowed list draws a flat slice).
/// Header kinds (Unfiled / Faction) are inert; Squad is a refile drop target when `orbat_refile`;
/// Folder → active-layer + folder-click selection (T-666); Slot → select + dbl-click→Attributes
/// (SEL-ORBAT-DBL-001); Comment → the click-to-select router, iff [`row_routes`] says so (T-784).
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
            <span class=ROW_BADGE data-sl-badge="true">"SL"</span>
        }
        .into_any()
    } else {
        ().into_any()
    };
    match row.kind {
        NodeKind::Unfiled => view! {
            <div class=ROW_UNFILED>
                {guide_spans(ancestors, guide_ids, collapsed)}
                {toggle}
                <MaterialIcon name="inbox" class="block text-sm leading-none" />
                <span>{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Faction => view! {
            <div class=ROW_FACTION>
                {guide_spans(ancestors, guide_ids, collapsed)}
                {toggle}
                <MaterialIcon name="flag" class="block text-sm leading-none" />
                <span class="truncate">{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Squad => {
            let dest = id.clone();
            if orbat_refile {
                view! {
                    <div
                        class=ROW_STATIC
                        title="Drop a slot here to refile into this squad"
                        on:pointerup=move |ev| {
                            ev.stop_propagation();
                            #[cfg(target_arch = "wasm32")]
                            crate::editor::state::operations::complete_refile_onto_squad(dest.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &dest;
                        }
                    >
                        {guide_spans(ancestors, guide_ids, collapsed)}
                        {toggle}
                        <MaterialIcon name="groups" class="block text-sm leading-none" />
                        <span class="truncate">{label}</span>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div class=ROW_STATIC>
                        {guide_spans(ancestors, guide_ids, collapsed)}
                        {toggle}
                        <MaterialIcon name="groups" class="block text-sm leading-none" />
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
            let dim = if row.hidden_effective {
                " opacity-40"
            } else {
                ""
            };
            let flag_toggles = layer_flag_toggles(
                &id,
                row.hidden,
                row.locked,
                row.hidden_effective,
                row.locked_effective,
            );
            // T-666 — is THIS folder being inline-renamed?
            // T-811 — track ONLY `renaming` (the id). Reading a draft-bearing signal here would
            // re-render the whole slice on every keystroke and remount the input (F2 trap).
            let editing = {
                let id = id.clone();
                let renaming = authoring.renaming;
                move || renaming.with(|r| r.as_ref() == Some(&id))
            };
            if authoring.enabled && editing() {
                // Inline-rename input (armed on create, or via the row's rename action). Enter /
                // blur commits through `rename_layer`; Escape cancels. Stops propagation so typing
                // never reaches the row's click/drag handlers.
                //
                // T-811 — `autofocus` alone does NOT focus this input. The row is inserted by a
                // reactive `{move || …}` re-render, not present at parse time, and the browser only
                // honours the `autofocus` content attribute for elements in the initial parse /
                // first document insertion — a node created by a later reactive update is skipped
                // (same mechanism note as eden_dock_left.rs T-785). `on_load` fires once when
                // Leptos mounts the node: focus it and select the seed text so the first keystroke
                // lands in the field AND replaces the old name. Draft lives in `rename_draft`,
                // which the list render does not read — so typing cannot remount this node.
                let renaming = authoring.renaming;
                let rename_draft = authoring.rename_draft;
                let rename_ref = NodeRef::<leptos::html::Input>::new();
                rename_ref.on_load(|el: web_sys::HtmlInputElement| {
                    // Focus+select immediately, then again on a 0ms timeout so the select
                    // wins against Leptos applying `prop:value` (which clears the selection
                    // and parks the caret at the end — pencil typing would otherwise append).
                    let _ = el.focus();
                    el.select();
                    let el2 = el.clone();
                    if let Some(win) = web_sys::window() {
                        use wasm_bindgen::JsCast;
                        let cb = wasm_bindgen::closure::Closure::once(move || {
                            let _ = el2.focus();
                            el2.select();
                        });
                        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                            cb.as_ref().unchecked_ref(),
                            0,
                        );
                        cb.forget();
                    }
                });
                let commit = {
                    let id = id.clone();
                    let renaming = renaming;
                    let rename_draft = rename_draft;
                    move |text: String| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let _ = crate::editor::state::operations::rename_layer(&id, &text);
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = (&id, &text);
                        renaming.set(None);
                        rename_draft.set(String::new());
                    }
                };
                let commit_key = commit.clone();
                let commit_blur = commit.clone();
                return view! {
                    // T-803 (fold a) — the drop-target folder reads as ROW_DROP_TARGET, distinct from
                    // selection's ROW_ACTIVE (state-vocabulary rule). Same predicate (`is_active`) the
                    // normal branch uses, so the rename input never loses the target cue mid-rename.
                    <div class=move || format!("{}{dim}", if is_active() { ROW_DROP_TARGET } else { ROW })>
                        {guide_spans(ancestors, guide_ids, collapsed)}
                        {toggle}
                        <MaterialIcon name=folder_icon class="block text-sm leading-none" />
                        <input
                            r#type="text"
                            node_ref=rename_ref
                            data-testid="layer-rename-input"
                            aria-label="Rename layer input"
                            class="min-w-0 flex-1 rounded bg-black/30 px-1 text-label-sm text-on-surface outline-none ring-1 ring-primary/60"
                            prop:value=move || rename_draft.get()
                            autofocus=true
                            on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()
                            on:pointerdown=|ev: web_sys::PointerEvent| ev.stop_propagation()
                            on:input=move |ev| {
                                rename_draft.set(event_target_value(&ev));
                            }
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                match ev.key().as_str() {
                                    "Enter" => {
                                        ev.prevent_default();
                                        commit_key(rename_draft.get());
                                    }
                                    "Escape" => {
                                        ev.prevent_default();
                                        renaming.set(None);
                                        rename_draft.set(String::new());
                                    }
                                    _ => {}
                                }
                            }
                            on:blur=move |_| {
                                commit_blur(rename_draft.get());
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
                    crate::editor::state::operations::set_active_layer(Some(id_click.clone()));
                    if authoring_on {
                        if ev.alt_key() || ev.shift_key() {
                            crate::editor::state::operations::select_layer_descendants(&id_click);
                        } else {
                            crate::editor::state::operations::select_layer_children(&id_click);
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
                folder_row_actions(&id, &label, authoring.renaming, authoring.rename_draft)
            } else {
                ().into_any()
            };
            // T-803 (fold a) — the active-drop-target folder wears ROW_DROP_TARGET (tertiary plate +
            // inset ring), which reads DIFFERENTLY from a SELECTED row's ROW_ACTIVE (primary plate +
            // top border). Two states, two treatments. `is_active` is per-row reactive, so the paint
            // (and the chip below) survive scrolling in the windowed tree — the row re-renders its
            // class on every `active_layer` change regardless of scroll position.
            //
            // BOTH-STATES: a folder row's paint is governed by `is_active` (drop target) ALONE — it
            // does not read the selection signal; selection paint (ROW_ACTIVE) is worn by the SLOT
            // rows a folder click selects, not by the folder itself. So "the drop target" and "a
            // selected row" live on different elements here and can never collapse into one ambiguous
            // paint. The `my_location` chip (row view below) is the redundant, non-colour half of the
            // cue and is the tiebreaker if the two states ever share an element in future.
            let base = {
                let is_active = is_active.clone();
                move || {
                    let g = if authoring_on { " group" } else { "" };
                    format!(
                        "{}{dim}{g}",
                        if is_active() { ROW_DROP_TARGET } else { ROW }
                    )
                }
            };
            // T-803 (fold a) — the "stated in the row UI" half of fold (c): a small target glyph that
            // rides the drop-target state. Non-colour cue (a `my_location` crosshair) so the state is
            // legible without relying on the tertiary tint alone. Reactive on the same `is_active`
            // predicate as `base`, so create-layer's ops-level default (create_layer sets the new
            // layer active, editor_ops.rs:3303) lights the chip immediately, and it moves with the
            // target as the operator clicks other folders.
            let target_chip = {
                let is_active = is_active.clone();
                move || {
                    if is_active() {
                        view! {
                            <span
                                data-testid="layer-drop-target-chip"
                                title="Next placement lands here"
                                class="ml-auto inline-flex h-3 shrink-0 items-center gap-0.5 rounded border border-tertiary/40 bg-tertiary/15 px-1 text-label-sm leading-none text-tertiary"
                            >
                                <MaterialIcon name="my_location" class="block text-sm leading-none" />
                            </span>
                        }
                        .into_any()
                    } else {
                        ().into_any()
                    }
                }
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
                            crate::editor::state::operations::begin_layer_drag(id_down.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_down;
                        }
                    }
                    on:pointerup=move |ev: web_sys::PointerEvent| {
                        if authoring_dnd {
                            ev.stop_propagation();
                            #[cfg(target_arch = "wasm32")]
                            {
                                let _ = crate::editor::state::operations::complete_layer_drop_onto_folder(id_up.clone());
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_up;
                        }
                    }
                >
                    {guide_spans(ancestors, guide_ids, collapsed)}
                    {toggle}
                    <MaterialIcon name=folder_icon class="block text-sm leading-none" />
                    <span class="truncate">{label}</span>
                    {target_chip}
                    {flag_toggles}
                    {row_actions}
                </button>
            }
            .into_any()
        }
        // T-651 (`PLACE-COMMENT-001`) / T-784 — the editor-only COMMENT row. Lifted into its own
        // function so the shape pin can examine it without the rest of this kind match — see
        // [`comment_row`].
        NodeKind::Comment => comment_row(row, selected, collapsed, toggle, authoring.enabled),
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
            let dim = if row.hidden_effective {
                " opacity-40"
            } else {
                ""
            };
            let lock_hint = if row.locked_effective {
                view! {
                    <MaterialIcon
                        name="lock"
                        class="ml-auto block shrink-0 pl-1 text-sm leading-none text-outline"
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
                        crate::editor::state::operations::select_slot(id.clone());
                    }
                    // T-159.26 A1 — outliner activate (native dblclick) opens Attributes,
                    // the SEL-ORBAT-DBL-001 contract.
                    on:dblclick=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor::state::operations::open_attributes(id_dbl.clone());
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &id_dbl;
                    }
                    on:pointerdown=move |_| {
                        if orbat_refile {
                            // T-180.6 — ORBAT tree: arm refile onto a squad.
                            #[cfg(target_arch = "wasm32")]
                            crate::editor::state::operations::begin_refile(id_refile.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_refile;
                        } else if authoring_slot {
                            // T-666 — Editor-Layers tree: arm refile of this slot into a folder
                            // (a folder-row `pointerup` completes it via `move_slot_to_layer`).
                            #[cfg(target_arch = "wasm32")]
                            crate::editor::state::operations::begin_layer_slot_drag(id_layer_refile.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_layer_refile;
                        }
                    }
                >
                    {guide_spans(ancestors, guide_ids, collapsed)}
                    {toggle}
                    <MaterialIcon name="person" class="block text-sm leading-none" />
                    <span class="truncate">{label}</span>
                    {sl_badge}
                    {lock_hint}
                </button>
            }
            .into_any()
        }
    }
}

// ── T-809 (fold c, F-22) — placed vehicles belong in the LEFT outliner, beside slots/groups ────────
//
// The eye-pass fold: a placed VEHICLE is a thing on the map, and things on the map live in the left
// outliner — not on the right dock (whose Placed strip listed them only because there was nowhere
// else). This lists every map-placed vehicle under the layer/slot tree, selectable with the SAME row
// affordances a slot gets: single click selects (routed through `select_slot`, which is kind-agnostic
// — it sets the selection and the engine's tint lane for whatever id it is handed), double click
// opens Attributes. A vehicle id in `ctx.selection` is already tolerated (the wave-145 lesson: the
// selection is pruned by DOCUMENT PRESENCE, not by kind), so nothing downstream chokes on it.
//
// Only MAP-PLACED vehicles appear (`xy.is_some()`): an ORBAT-only vehicle with no map position is not
// a thing ON the map, so it does not belong in this on-the-map list (it lives in the ORBAT tree).
// Gated on `authoring` — the LEFT outliner is the only `virtual_tree` with it set — so the ORBAT tree
// and any future tree never grow a vehicles footer. wasm-only read: `editor_ops` is wasm32-only, so
// the native shell renders nothing (the same reason `placed_vehicles_panel` stubs to nothing).

/// T-809 (fold c) — the map-placed vehicles as selectable outliner rows, or nothing when this is not
/// the authoring (left-outliner) tree. See the section header.
#[cfg(target_arch = "wasm32")]
fn placed_vehicle_rows(authoring: bool, selected: RwSignal<Vec<String>>) -> AnyView {
    if !authoring {
        return ().into_any();
    }
    let rows: Vec<crate::editor::state::operations::VehicleRow> =
        crate::editor::state::operations::vehicle_rows()
            .into_iter()
            .filter(|v| v.xy.is_some()) // on-the-map vehicles only
            .collect();
    if rows.is_empty() {
        return ().into_any();
    }
    view! {
        // A small-caps section marker (ROW_FACTION idiom) sets the placed vehicles off from the layer
        // tree above without pretending to be a collapsible folder.
        <div class=ROW_FACTION aria-hidden="true">
            <span class="size-4 shrink-0"></span>
            <MaterialIcon name="directions_car" class="block text-sm leading-none" />
            <span class="truncate">"Placed vehicles"</span>
        </div>
        {rows
            .into_iter()
            .map(|v| {
                let id = v.id.clone();
                let id_click = id.clone();
                let id_dbl = id.clone();
                // Label the row by the vehicle's classname tail (`resourceName` is a GUID-headed path);
                // the outliner shows an author-legible name, not a raw prefab path.
                let label = {
                    let tail = crate::editor::arsenal::asset_catalog::classname_tail(&v.resource_name);
                    if tail.is_empty() { v.id.clone() } else { tail.to_string() }
                };
                let aria = label.clone();
                let is_sel = {
                    let id = id.clone();
                    move || selected.get().iter().any(|s| s == &id)
                };
                view! {
                    <button
                        type="button"
                        aria-label=aria
                        class=move || if is_sel() { ROW_ACTIVE } else { ROW }
                        on:click=move |_| {
                            // Same single-click contract a slot row has: select through the
                            // kind-agnostic `select_slot` (sets selection + engine tint).
                            crate::editor::state::operations::select_slot(id_click.clone());
                        }
                        on:dblclick=move |_| {
                            // SEL-ORBAT-DBL-001 — activate opens Attributes, exactly like a slot.
                            crate::editor::state::operations::open_attributes(id_dbl.clone());
                        }
                    >
                        // A leading spacer keeps these rows aligned with the tree's guide column.
                        <span class="size-4 shrink-0"></span>
                        <MaterialIcon name="directions_car" class="block text-sm leading-none" />
                        <span class="truncate">{label}</span>
                    </button>
                }
                .into_any()
            })
            .collect::<Vec<_>>()}
    }
    .into_any()
}

/// Native shell: `editor_ops` is wasm32-only, so there is no document to read — no placed-vehicle
/// rows on the native build (the `placed_vehicles_panel` stub rule).
#[cfg(not(target_arch = "wasm32"))]
fn placed_vehicle_rows(_authoring: bool, _selected: RwSignal<Vec<String>>) -> AnyView {
    ().into_any()
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
/// flattened list renders eagerly; above it an `h-full` scroll container (T-769: measured from the
/// dock's flex-1 tree region) draws only the visible slice (+ overscan) between two spacer divs, so
/// a mission-scale tree never builds N DOM rows. `stats_key` names this tree in
/// `window.__outlinerStats`.
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
    // T-666 / T-811 — inline-rename: id signal (list-tracked) + draft signal (input-only).
    let renaming = RwSignal::new(None::<String>);
    let rename_draft = RwSignal::new(String::new());
    // SEL-GROUP-ICON-001 — the "folder directly holds a slot" set, recomputed with the flatten.
    let holds_slots = StoredValue::new(std::collections::HashSet::<String>::new());
    let authoring_ctx = RowAuthoring {
        enabled: authoring,
        holds_slots,
        renaming,
        rename_draft,
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
            if let Some(new_id) = crate::editor::state::operations::take_rename_armed() {
                // Seed the buffer with the just-minted "New Layer N" name so a blur with no typing
                // keeps it (rename rejects a blank), and the caret lands on real text to overwrite.
                let seed = ns
                    .iter()
                    .find(|n| n.id == new_id)
                    .map_or_else(String::new, |n| n.label.clone());
                rename_draft.set(seed);
                renaming.set(Some(new_id));
            }
        }
        let f = collapsed.with(|c| flatten_visible(&ns, c));
        flat.set_value(f);
        rev.update(|r| *r = r.wrapping_add(1));
    });
    let scroll_top = RwSignal::new(0.0_f64);
    // T-769 — live scroller height. Starts at the historical 420 px fallback; an Effect reads
    // `clientHeight` once the `h-full` node is mounted (and again on resize/scroll) so windowing
    // tracks the flex-1 tree region instead of a nested fixed budget.
    let container_h = RwSignal::new(CONTAINER_H_FALLBACK);
    let scroller_ref = NodeRef::<leptos::html::Div>::new();
    #[cfg(target_arch = "wasm32")]
    let resize_hooked = StoredValue::new(false);
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            let Some(node) = scroller_ref.get() else {
                return;
            };
            let el: web_sys::Element = node.unchecked_into();
            let h = el.client_height() as f64;
            if h > 0.0 {
                container_h.set(h);
            }
            if !resize_hooked.get_value() {
                resize_hooked.set_value(true);
                if let Some(win) = web_sys::window() {
                    let closure = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::wrap(
                        Box::new(move |_| {
                            if let Some(node) = scroller_ref.get_untracked() {
                                let el: web_sys::Element = node.unchecked_into();
                                let h = el.client_height() as f64;
                                if h > 0.0 {
                                    container_h.set(h);
                                }
                            }
                        }),
                    );
                    let _ = win.add_event_listener_with_callback(
                        "resize",
                        closure.as_ref().unchecked_ref(),
                    );
                    closure.forget();
                }
            }
        }
    });
    (move || {
        rev.track(); // re-render the slice when the tree changes
        let st = scroll_top.get();
        let viewport_h = container_h.get();
        // T-809 (fold c) — the placed-vehicle rows ride BELOW the layer/slot tree in the LEFT
        // outliner. Built fresh in whichever branch renders (an `AnyView` moves on use, and exactly
        // one branch runs per render), inside the `rev`-tracked closure so a placement — which
        // rebuilds the outliner `nodes` and bumps `rev` — re-reads them. `placed_vehicle_rows` is
        // empty on every non-authoring tree and on the native shell, so this is inert off the left
        // dock; it is appended as a tree sibling so it never enters the windowing arithmetic.
        flat.with_value(|f| {
            let total = f.len();
            if total == 0 {
                set_outliner_stats(stats_key, 0, 0);
                return view! {
                    <div>
                        <p class="text-label-sm text-outline">{empty_msg}</p>
                        {placed_vehicle_rows(authoring, selected)}
                    </div>
                }
                .into_any();
            }
            if total <= VIRTUAL_SLOT_THRESHOLD {
                set_outliner_stats(stats_key, total, total);
                return view! {
                    <div>
                        {f
                            .iter()
                            .map(|r| single_row(r, selected, active_layer, collapsed, orbat_refile, authoring_ctx))
                            .collect::<Vec<_>>()}
                        {placed_vehicle_rows(authoring, selected)}
                    </div>
                }
                .into_any();
            }
            let per_screen = (viewport_h / ROW_H).ceil() as usize;
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
                    node_ref=scroller_ref
                    class="h-full min-h-0 overflow-y-auto"
                    data-testid="outliner-window-scroller"
                    on:scroll=move |ev| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                                let h = el.client_height() as f64;
                                if h > 0.0 {
                                    container_h.set(h);
                                }
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
                    // T-809 (fold c) — placed vehicles as a footer under the windowed tree. They ride
                    // after the bottom spacer (which pads for the un-rendered slot rows), so they sit
                    // at the true end of the list; a small addendum that need not join the windowing.
                    {placed_vehicle_rows(authoring, selected)}
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
    use crate::editor::panels::outliner::{build_outliner, LayerRow, SlotRow};

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

        const OPS: &str = include_str!("../state/operations.rs");
        const TREE: &str = include_str!("outliner_tree.rs");
        const DOCK: &str = include_str!("dock_left.rs");

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

        /// T-809 (fold c, F-22) — placed vehicles are listed in the LEFT outliner, beside slots, with
        /// the SAME row affordances a slot gets: single-click selects through the kind-agnostic
        /// `select_slot`, double-click opens Attributes via `open_attributes`. Read off `vehicle_rows`
        /// and filtered to MAP-PLACED (`xy.is_some()`); gated on `authoring` so only the left outliner
        /// grows the footer. `virtual_tree` appends the rows in every render branch (empty/eager/
        /// windowed), so the list is present regardless of tree size.
        #[test]
        fn placed_vehicles_are_listed_in_the_outliner_with_slot_affordances() {
            use crate::editor::arsenal::class_r_scrub::{live_code, live_source, only_body};
            let code = live_code(TREE);
            // Two cfg variants share the name; the wasm one (no leading `_` on the params) is the one
            // with the real body — match its unique signature so `only_body` is unambiguous.
            let body = only_body(&code, "fn placed_vehicle_rows(authoring:");
            assert!(
                body.contains("vehicle_rows()"),
                "T-809: the outliner footer reads the placed vehicles off editor_ops::vehicle_rows"
            );
            assert!(
                body.contains("xy.is_some()"),
                "T-809: only MAP-PLACED vehicles belong in the on-the-map outliner"
            );
            assert!(
                body.contains("select_slot(") && body.contains("open_attributes("),
                "T-809: a vehicle row selects (single click) and opens Attributes (dbl click) like a slot"
            );
            // Gated on `authoring` — the ORBAT tree (authoring=false) must not grow the footer.
            assert!(
                body.contains("if !authoring") || body.contains("!authoring"),
                "T-809: the footer is the left outliner's only — gated on the authoring flag"
            );
            // `virtual_tree` appends the footer in each render branch, so it survives windowing.
            let vt = only_body(&code, "fn virtual_tree(");
            let calls = vt.matches("placed_vehicle_rows(").count();
            assert!(
                calls >= 3,
                "T-809: virtual_tree must append placed vehicles in the empty, eager AND windowed \
                 branches (found {calls}); a single call would drop them under one render path"
            );
            // The row is labelled for the operator (a section marker + a vehicle glyph).
            let lit = live_source(TREE);
            let lit_body = only_body(&lit, "fn placed_vehicle_rows(authoring:");
            assert!(
                lit_body.contains("\"Placed vehicles\"") && lit_body.contains("directions_car"),
                "T-809: the outliner section names itself and carries the vehicle glyph"
            );
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

        /// T-811 — layer rename focuses via NodeRef/on_load; draft is decoupled from the
        /// list-tracked id signal (wave200 F1 / F2 remount trap).
        #[test]
        fn layer_rename_uses_noderef_onload_and_decoupled_draft() {
            // Raw TREE includes this test module, so every needle below would self-match its own
            // assertion string (the T-759 hollow-pin class); scrub to the production half.
            let tree = crate::editor::arsenal::class_r_scrub::live_source(TREE);
            assert!(
                tree.contains("NodeRef::<leptos::html::Input>::new()"),
                "the layer rename input must carry a NodeRef so it can be focused on mount"
            );
            assert!(
                tree.contains("node_ref=rename_ref"),
                "the NodeRef must be attached via node_ref=rename_ref"
            );
            assert!(
                tree.contains(".on_load(")
                    && tree.contains(".focus()")
                    && tree.contains(".select()"),
                "on_load must call focus() and select() on the mounted input"
            );
            assert!(
                tree.contains("renaming:") && tree.contains("rename_draft:"),
                "rename id and draft must be separate RowAuthoring fields"
            );
            assert!(
                tree.contains("data-testid=\"layer-rename-input\""),
                "rename input needs a stable test id for the CDP acceptance probe"
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

        /// T-803 (fold a) — the DROP-TARGET folder row reads DIFFERENTLY from a SELECTED row.
        ///
        /// The active drop target (`is_active`, the layer the next placement/comment lands in) and
        /// selection (`is_sel`) are two states, and the state-vocabulary rule says two states get two
        /// treatments. Before this fix both folder sites painted the active row with `ROW_ACTIVE` —
        /// the SAME class selection wears — so a drop-target folder and a selected row were the same
        /// paint. Both folder sites now use `ROW_DROP_TARGET`, and it shares no distinguishing token
        /// with `ROW_ACTIVE`.
        ///
        /// Source-scrubbed (`live_source` cuts the test module + comments, keeps class-name/`testid`
        /// literals) so this pin can never self-match its own assertion strings — the T-759 hollow-pin
        /// class. Scoped to `fn single_row` so the Slot arm's legitimate `if is_sel() { ROW_ACTIVE }`
        /// (selection, preserved) is in view for the non-regression check but does not pollute the
        /// `is_active`-predicate count.
        #[test]
        fn t803_drop_target_reads_differently() {
            use crate::editor::arsenal::class_r_scrub::{live_source, only_body};
            let src = live_source(TREE);
            let body = only_body(&src, "fn single_row(");

            // Both folder sites (inline-rename branch + normal `base` closure) paint the ACTIVE
            // drop-target row with ROW_DROP_TARGET, never ROW_ACTIVE. PERTURB: swap one site back to
            // `if is_active() { ROW_ACTIVE }` and this count drops to 1 → RED.
            let drop_sites = body.matches("if is_active() { ROW_DROP_TARGET }").count();
            assert_eq!(
                drop_sites, 2,
                "T-803: both folder sites (rename branch + normal branch) must paint the active \
                 drop target with ROW_DROP_TARGET — found {drop_sites} of 2"
            );
            // The old defect verbatim: the drop target wearing selection's class. Zero, exactly.
            assert!(
                !body.contains("if is_active() { ROW_ACTIVE }"),
                "T-803: a folder's drop-target state must NOT reuse selection's ROW_ACTIVE — that is \
                 the two-states-one-treatment defect this ticket closes"
            );
            // Non-regression: SELECTION still uses ROW_ACTIVE in the slot arm (`is_sel`). If this
            // vanished, the fix would have collateral-damaged the T-649/T-668 selection paint.
            assert!(
                body.contains("if is_sel() { ROW_ACTIVE }"),
                "T-803: a SELECTED slot row must still wear ROW_ACTIVE — the fix restyles the drop \
                 target only, never selection"
            );

            // The non-colour half of the cue: a target chip rides the drop-target state, with a
            // stable testid for the CDP probe and a `my_location` glyph so the state is legible
            // without relying on the tertiary tint alone.
            assert!(
                body.contains("data-testid=\"layer-drop-target-chip\"")
                    && body.contains("my_location"),
                "T-803: the drop-target row carries a testid'd target chip + glyph (non-colour cue)"
            );
        }

        /// T-803 (fold a) — `ROW_DROP_TARGET` and `ROW_ACTIVE` are DISTINCT recipes: not equal, and
        /// neither contains the other's distinguishing token. Production consts, so this reads their
        /// values directly (no scrub — same rationale as the T-668 vocabulary pin). This is the class
        /// half of the state-vocabulary rule: the two states cannot be told apart if their classes
        /// collapse to the same tokens.
        #[test]
        fn t803_drop_target_class_is_distinct_from_active() {
            use super::{ROW, ROW_ACTIVE, ROW_DROP_TARGET};
            assert_ne!(
                ROW_DROP_TARGET, ROW_ACTIVE,
                "T-803: drop-target and selection recipes must not be the same string"
            );
            // ROW_ACTIVE's distinguishing tokens (selection): the primary plate + the dark TOP BORDER.
            // None may appear in ROW_DROP_TARGET.
            for tok in ["bg-primary/20", "border-t", "border-background/60"] {
                assert!(
                    !ROW_DROP_TARGET.contains(tok),
                    "T-803: ROW_DROP_TARGET must not carry selection's token `{tok}` — a shared token \
                     is how two states start reading as one"
                );
            }
            // ROW_DROP_TARGET's distinguishing tokens (drop target): the tertiary plate + the INSET
            // RING. None may appear in ROW_ACTIVE.
            for tok in ["bg-tertiary/15", "ring-inset", "ring-tertiary/50"] {
                assert!(
                    !ROW_ACTIVE.contains(tok),
                    "T-803: ROW_ACTIVE must not carry the drop-target token `{tok}`"
                );
            }
            // The ring is a box-shadow, not a border, so — like ROW — the drop-target row adds no
            // height and survives windowing at the shared pitch (no `border-t` to grow the box).
            assert!(
                !ROW_DROP_TARGET.contains("border-t") && !ROW.contains("border-t"),
                "T-803: the drop-target ring must not reintroduce a top border (box-box height drift)"
            );
        }
    }

    /// T-668 — the shared tree-row recipes speak the one state vocabulary, so every dock/panel that
    /// consumes `ROW`/`ROW_ACTIVE` (both docks, zones, compositions, triggers) inherits it. These are
    /// production consts, so the pin reads their values directly — no scrub. The load-bearing check is
    /// `ROW_ACTIVE`'s dark top border: it is what makes a SELECTED row distinct-by-construction from a
    /// HOVERED one (before T-668 it had none, so the two differed only by tint).
    mod t668_vocabulary {
        use crate::editor::layout::{HOVER_FILL, TOGGLED_PLATE};

        /// The idle row carries the HOVER_FILL tokens (solid fill on hover, no border).
        #[test]
        fn row_carries_the_hover_fill() {
            for tok in HOVER_FILL.split_whitespace() {
                assert!(
                    super::super::ROW.contains(tok),
                    "ROW must carry HOVER_FILL token `{tok}` (the one hover fill)"
                );
            }
            assert!(
                !super::super::ROW.contains("border-t"),
                "an idle ROW must have NO top border — that is the TOGGLED cue"
            );
        }

        /// The selected row carries the TOGGLED_PLATE tokens — the lighter primary plate AND the 1px
        /// dark top border. This is the fix: distinct-by-construction from a hovered row.
        #[test]
        fn row_active_carries_the_toggled_plate() {
            for tok in TOGGLED_PLATE.split_whitespace() {
                assert!(
                    super::super::ROW_ACTIVE.contains(tok),
                    "ROW_ACTIVE must carry TOGGLED_PLATE token `{tok}` (plate + dark top border)"
                );
            }
        }

        /// Fire the distinction (perturb/fail/restore): a selected row and a hovered row differ by
        /// the top border BY CONSTRUCTION. RESTORE: `ROW_ACTIVE` has `border-t`, `ROW` does not.
        /// PERTURB: were `ROW_ACTIVE` merely the neutral hover fill (the defect), it would carry no
        /// border and this check would reject it.
        #[test]
        fn selected_and_hovered_rows_are_distinct_by_construction() {
            assert!(
                super::super::ROW_ACTIVE.contains("border-t")
                    && !super::super::ROW.contains("border-t"),
                "RESTORE: only the selected row has the top border"
            );
            // PERTURB — the defect value (a toggle wearing the bare hover fill) has no border.
            let defect = "bg-white/10";
            assert!(
                !defect.contains("border-t"),
                "PERTURB: a selected row rendered as the neutral hover fill has no distinguishing \
                 border — the check must reject it"
            );
            assert_ne!(
                super::super::ROW,
                super::super::ROW_ACTIVE,
                "idle and selected rows must not be the same string"
            );
        }

        /// The palette leaf is the idle ROW plus a grab cursor — same HOVER_FILL, no toggled plate
        /// (a leaf is dragged, never a persistent toggle).
        #[test]
        fn palette_leaf_is_hover_fill_plus_grab() {
            assert!(super::super::PALETTE_LEAF.contains("hover:bg-white/10"));
            assert!(super::super::PALETTE_LEAF.contains("cursor-grab"));
            assert!(
                !super::super::PALETTE_LEAF.contains("border-t"),
                "a palette leaf is not a toggle — no toggled-plate border"
            );
        }
    }
}

/// T-637 — **THE PITCH IS ONE NUMBER, STATED ONCE.**
///
/// `ROW_H` is not decoration: the windowed renderer reserves off-screen rows with two spacer divs
/// sized `n × ROW_H`, so if `ROW_H` and the row class disagree the scroll height is a lie and a fast
/// scroll lands on the wrong row — by `n × Δ`, which grows with the tree. Before this ticket the two
/// were connected only by a comment, AND they already disagreed: `ROW_ACTIVE` carries a `border-t`
/// that `ROW` does not, so under `box-sizing: border-box` a selected row rendered a pixel taller than
/// the 24 the spacers reserved.
///
/// Both halves are fixed here. Every row recipe states `h-4` explicitly (so the border lives inside
/// the box and no recipe can be taller than another), and `ROW_H` is that class read back through the
/// Tailwind spacing scale rather than re-typed as a literal.
#[cfg(test)]
mod t637_one_dense_row_geometry {
    use super::{
        PALETTE_LEAF, ROW, ROW_ACTIVE, ROW_BADGE, ROW_FACTION, ROW_GEOM, ROW_H, ROW_STATIC,
        ROW_UNFILED,
    };
    use crate::editor::layout::tw_len_px;

    /// Every recipe a tree row can wear is [`ROW_GEOM`] plus a paint, and the windowing constant is
    /// that geometry's stated height — not a number that merely happens to match it today.
    #[test]
    fn every_row_recipe_states_the_one_height_and_row_h_reads_it_back() {
        let geom_h = tw_len_px(ROW_GEOM, "h-").expect("ROW_GEOM must state an explicit `h-*`");
        assert!(
            (geom_h - ROW_H).abs() < f64::EPSILON,
            "T-637: ROW_H ({ROW_H}) must BE the height the shared row class renders ({geom_h}) — \
             the virtual spacers reserve n × ROW_H, so a mismatch is a scroll position that drifts \
             further wrong the longer the tree gets"
        );
        for (name, recipe) in [
            ("ROW", ROW),
            ("ROW_ACTIVE", ROW_ACTIVE),
            ("PALETTE_LEAF", PALETTE_LEAF),
            ("ROW_STATIC", ROW_STATIC),
            ("ROW_UNFILED", ROW_UNFILED),
            ("ROW_FACTION", ROW_FACTION),
        ] {
            let h = tw_len_px(recipe, "h-")
                .unwrap_or_else(|| panic!("T-637: `{name}` must state the row height explicitly"));
            assert!(
                (h - ROW_H).abs() < f64::EPSILON,
                "T-637: `{name}` renders at {h} px in a tree windowed at {ROW_H} px"
            );
            assert!(
                recipe.starts_with(ROW_GEOM),
                "T-637: `{name}` must be built from ROW_GEOM — the pitch lived in seven separate \
                 string literals before this ticket, which is how ROW_ACTIVE drifted"
            );
            // No vertical padding may creep back: it would add to `h-4` under border-box only if the
            // content overflowed, but stating both is how the two definitions start disagreeing again.
            assert!(
                !recipe.contains(" py-") && !recipe.contains(" p-"),
                "T-637: `{name}` states its height; a `py-*`/`p-*` beside it is a second opinion"
            );
        }
    }

    /// **THE DENSITY, as a number.** 24 px was the complaint — a title, one row and ~900 px of void
    /// in a 240 px dock. Eden's outliner runs at ~15.8 px and that is how it fits a mission's worth
    /// of structure in the same width. This pins the pitch change against the historical 420 px
    /// budget (17 → 26 rows). T-769 made the live scroller measured/`h-full`; the arithmetic here is
    /// still the densification magnitude, not a claim about the live container height.
    #[test]
    fn the_tree_is_dense_enough_to_be_eden_shaped() {
        assert!(
            ROW_H <= 16.0,
            "T-637: {ROW_H} px per row is not a dense tree — Eden's is ~15.8"
        );
        // Historical reference height from the pre-T-769 fixed budget — densification only.
        const DENSITY_REFERENCE_H: f64 = 420.0;
        let rows_now = (DENSITY_REFERENCE_H / ROW_H).floor();
        let rows_before = (DENSITY_REFERENCE_H / 24.0).floor();
        assert!(
            rows_now >= rows_before + 8.0,
            "T-637: the densification must buy real rows — {rows_now} visible where the pre-ticket \
             24 px pitch gave {rows_before}"
        );
    }

    /// The in-row SL badge must fit INSIDE the row rather than setting its height. `ui::badge_class`
    /// (the page-level pill) is `px-2 py-0.5` + a border ⇒ 22 px, which burst a 16 px row open; that
    /// is why the tree carries its own.
    #[test]
    fn the_leader_badge_fits_inside_the_row() {
        let badge_h = tw_len_px(ROW_BADGE, "h-").expect("the row badge must state a height");
        assert!(
            badge_h < ROW_H,
            "T-637: the SL badge is {badge_h} px inside a {ROW_H} px row — a badge that sets the \
             row height is what made the ORBAT tree ragged"
        );
        assert!(
            ROW_BADGE.contains("leading-none"),
            "T-637: without `leading-none` the badge's line box (16 px) exceeds its own `h-3` box"
        );
        assert!(
            ROW_BADGE.contains("shrink-0"),
            "T-637: the badge must not be squeezed by a long label in a 240 px dock"
        );
    }

    /// T-769 — the windowed scroller fills the flex tree region (`h-full`) and sizes its window from
    /// the measured `clientHeight`. A fixed `height:420px` coming back is the defect this pin guards.
    #[test]
    fn the_windowed_scroller_is_measured_h_full_not_a_fixed_budget() {
        use crate::editor::arsenal::class_r_scrub::{live_code, live_source};
        let raw = include_str!("outliner_tree.rs");
        let code = live_code(raw);
        let source = live_source(raw);
        assert!(
            code.contains("client_height"),
            "T-769: windowing must read the live scroller's clientHeight"
        );
        assert!(
            source.contains("h-full min-h-0 overflow-y-auto"),
            "T-769: the windowed scroller must be h-full inside the flex-1 tree region"
        );
        assert!(
            source.contains("outliner-window-scroller"),
            "T-769: the smoke measures this scroller by data-testid"
        );
        // Assembled so this pin's own prose cannot satisfy the negative check.
        let fixed = format!("height:{}px", 420);
        assert!(
            !source.contains(&fixed),
            "T-769: a fixed 420 px windowed budget must not return"
        );
        assert!(
            !code.contains("CONTAINER_H:") && !code.contains("CONTAINER_H}"),
            "T-769: CONTAINER_H must not drive windowing anymore"
        );
    }

    /// Every glyph in a row is `text-sm`, whose default line box is 20 px — 4 px taller than the row
    /// it sits in. `leading-none` collapses that line box to the glyph, which is what lets a 16 px row
    /// hold a 16 px icon cell without the icons setting the height. A single icon that forgets it
    /// re-inflates every row it appears in, so this is checked over the whole file rather than per
    /// call site.
    #[test]
    fn no_row_glyph_carries_an_uncollapsed_line_box() {
        let src = include_str!("outliner_tree.rs");
        let production = src
            .split("#[cfg(test)]")
            .next()
            .expect("the production half precedes the test modules");
        // Needles assembled so this test's own source cannot satisfy or false-fail them.
        let loose = format!("{}{}", "text-sm", "\"");
        assert!(
            !production.contains(&loose),
            "T-637: a row glyph ends its class list at `text-sm` — its 20 px line box would set the \
             height of the 16 px row it sits in. Add `leading-none`."
        );
    }
}

// ═══════ T-784 — a comment row SELECTS, and its affordance is the router's own answer ════════════
//
// The gap this closes was TOTAL, not partial: the Outliner's comment row was `ROW_STATIC` with no
// click path to `ctx.selection` at all, the map glyph had no pick, the T-697 selection filter only
// NARROWS an existing selection (so it cannot introduce a comment that was never selected), and the
// document-search router had no comment arm, so comment hits rendered inert.
//
// Two pins here. The CORRESPONDENCE pin walks every `NodeKind` and asserts, in both directions,
// that the affordance `row_routes` paints is exactly what the click `route_select_by_subject_id`
// will find — with a non-vacuity assert proving the corpus contained BOTH selectable and inert
// rows, because an all-false corpus would green a `row_routes` that always answered `false`. It
// never names which kinds "should" be selectable: that stale-list shape is what made the dock-left
// pin green while it guarded a lie (wave-129 RV-1). The SHAPE pin holds the rendered row to the
// `eden_settings` a11y contract — button when live, non-focusable `aria-disabled` when not.
#[cfg(test)]
mod t784_comment_row_selects {
    use super::{inert_row_reason, row_router_subject, row_routes};
    use crate::editor::arsenal::class_r_scrub::{live_code, live_source, only_body};
    use crate::editor::panels::outliner::NodeKind;
    use crate::editor::panels::validation_panel::{
        register_route_probe, register_select_by_id, route_select_by_subject_id,
    };

    /// A dense ordinal per `NodeKind`. **The compiler is the completeness check**: the match is
    /// exhaustive, so a new variant cannot build until it is given an ordinal, and the coverage
    /// assertion in the correspondence test then fails until [`every_node_kind`] actually contains
    /// it. Rust has no derive that enumerates a foreign enum; a bare list with no ordinal check
    /// would be precisely the stale kind list these pins exist to forbid.
    fn kind_ordinal(k: NodeKind) -> usize {
        match k {
            NodeKind::Folder => 0,
            NodeKind::Unfiled => 1,
            NodeKind::Slot => 2,
            NodeKind::Faction => 3,
            NodeKind::Squad => 4,
            NodeKind::Comment => 5,
        }
    }

    /// One more than the largest ordinal `kind_ordinal` hands out.
    const KINDS: usize = 6;

    fn every_node_kind() -> [NodeKind; KINDS] {
        [
            NodeKind::Folder,
            NodeKind::Unfiled,
            NodeKind::Slot,
            NodeKind::Faction,
            NodeKind::Squad,
            NodeKind::Comment,
        ]
    }

    /// **The affordance and the click cannot disagree — over EVERY row kind, in BOTH directions.**
    ///
    /// One resolver, registered as both seams, exactly as `mission_editor`'s mount wires them
    /// (`Rc::clone` of a single closure). It resolves ids ending `-yes` and refuses the rest, so
    /// every kind is exercised against a router that says yes AND a router that says no — the two
    /// directions the invariant has actually been violated in: an affordance over a dead click
    /// (T-754) and an inert row over a click that would have worked (wave-129 RV-1).
    ///
    /// Perturbation RED: make `row_routes` return `true` for `NodeKind::Comment` without asking the
    /// probe, or drop the `Comment` arm from `row_router_subject`.
    #[test]
    fn the_affordance_and_the_click_cannot_disagree_over_any_row_kind() {
        let resolve: std::rc::Rc<dyn Fn(&str) -> bool> =
            std::rc::Rc::new(|id: &str| id.ends_with("-yes"));
        {
            let p = std::rc::Rc::clone(&resolve);
            register_route_probe(std::rc::Rc::new(move |id: &str| p(id)));
        }
        {
            let p = std::rc::Rc::clone(&resolve);
            register_select_by_id(std::rc::Rc::new(move |id: &str| p(id)));
        }

        let mut covered = [false; KINDS];
        let mut saw_selectable = false;
        let mut saw_inert = false;
        for kind in every_node_kind() {
            covered[kind_ordinal(kind)] = true;
            for id in ["row-yes", "row-no"] {
                // What the VIEW paints.
                let affordance = row_routes(kind, id);
                // What the CLICK finds: this row's subject handed to the router itself.
                let click = row_router_subject(kind, id).is_some_and(route_select_by_subject_id);
                assert_eq!(
                    affordance, click,
                    "T-784: row kind {kind:?} with id {id:?} paints affordance={affordance} over a \
                     click that resolves {click} — a row is clickable IFF clicking it does \
                     something, and both ends of that must be the SAME resolver's answer"
                );
                saw_selectable |= affordance;
                saw_inert |= !affordance;
            }
        }
        assert!(
            covered.iter().all(|c| *c),
            "T-784: the corpus must cover every NodeKind ordinal — a kind this pin never saw is a \
             kind it never guarded"
        );
        // NON-VACUITY. Without this an all-inert corpus greens a `row_routes` that answers `false`
        // unconditionally, which is the defect (the comment row) restored under a passing test.
        assert!(
            saw_selectable,
            "T-784: VACUOUS — no row in the corpus was selectable, so the equality above proved \
             nothing about the affordance being painted"
        );
        assert!(
            saw_inert,
            "T-784: VACUOUS — no row in the corpus was inert, so the equality above proved nothing \
             about the affordance being WITHHELD"
        );
    }

    /// The refusal is the PROBE's, never a kind ban, and never a fallback when no probe is
    /// registered: no probe means no router to click into, and `false` is the honest answer.
    #[test]
    fn no_probe_means_no_affordance_and_no_fallback() {
        register_route_probe(std::rc::Rc::new(|_: &str| false));
        assert!(
            !row_routes(NodeKind::Comment, "cmt-1"),
            "T-784: a refusing probe must leave the comment row inert"
        );
        let src = live_code(include_str!("outliner_tree.rs"));
        let routes = only_body(&src, "pub(crate) fn row_routes(");
        assert!(
            routes.contains(&format!("subject_id{}", "_routes")),
            "T-784: clickability must BE subject_id_routes — the shape follows that boolean, it \
             does not replace it"
        );
        // NEGATIVE, and scoped to the two functions that make the decision (a negative over the
        // whole file would be green by construction — `single_row` is one giant kind match).
        for marker in [
            "pub(crate) fn row_routes(",
            "pub(crate) fn row_router_subject",
        ] {
            let body = only_body(&src, marker);
            assert!(
                !body.contains(&format!("route{}", "_target")),
                "T-784: {marker} must not re-ask mission_editor::route_target directly — the \
                 registered probe is the one answer, and a second reader of the resolution is how \
                 the affordance and the click drift apart"
            );
        }
    }

    /// **The rendered shape follows that boolean.** Routable ⇒ a real `<button>` (a keyboard user
    /// can activate the selection); refused ⇒ a non-focusable element carrying `aria-disabled` and
    /// [`inert_row_reason`] — never a tab-stop button that does nothing (the wave-115 MINOR-6 shape,
    /// as `eden_settings` fixed it).
    ///
    /// Literals kept (`live_source`): the claim is about the tags and attributes that ship.
    #[test]
    fn the_comment_row_branches_on_the_router_and_is_never_a_dead_button() {
        let lit = live_source(include_str!("outliner_tree.rs"));
        let arm = only_body(&lit, &format!("fn comment{}", "_row("));
        assert!(
            arm.contains(&format!("row{}", "_routes(")),
            "T-784: the comment arm must branch on row_routes — the one boolean that owns \
             clickability"
        );
        assert!(
            arm.contains("<button") && arm.contains("</button>"),
            "T-784: a routable comment row must be a real button"
        );
        assert_eq!(
            arm.matches("<button").count(),
            1,
            "T-784: exactly one <button> in the comment arm — an inert `<button aria-disabled>` is \
             still a tab stop, which is the shape this rejects"
        );
        assert!(
            arm.contains("aria-disabled") && arm.contains(&format!("inert_row{}", "_reason(")),
            "T-784: the inert branch must be a non-focusable element that says WHY"
        );
        let router = ["route_select", "_by_subject_id("].concat();
        assert!(
            arm.contains(&router),
            "T-784: the click must be the shipped router — the same resolution row_routes asked, \
             not a second selection path"
        );
        // The reason must name the refusal it actually got, never a kind ban: dock-left's old
        // "resolves slots and vehicles only" became a lie the moment the router grew an arm.
        let reason = inert_row_reason().to_lowercase();
        assert!(
            reason.contains("not selectable") || reason.contains("resolves nothing"),
            "T-784: the inert reason must name the router's refusal, got {reason:?}"
        );
        for banned in ["slot", "vehicle", "comment"] {
            assert!(
                !reason.contains(banned),
                "T-784: the inert reason must not name a KIND ({banned:?}) — that sentence goes \
                 stale the next time the router grows an arm"
            );
        }
    }
}
