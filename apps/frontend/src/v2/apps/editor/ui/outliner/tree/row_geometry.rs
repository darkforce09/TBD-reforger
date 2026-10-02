//! Row geometry for editor outliner trees.

use super::*;

// All tree rows share a 16 px border-box height. The selected row's top border fits inside
// that height, so virtual spacers remain aligned with both idle and selected rows.

/// the geometry EVERY tree row shares, and the single place the 16 px pitch is stated.
/// `h-4` is [`ROW_H`]; `items-center` centres the 16 px chevron/glyph cells inside it.
pub(crate) const ROW_GEOM: &str =
    "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm";

/// A tree row's shared recipe (idle): [`ROW_GEOM`] + [`crate::v2::apps::editor::shell::layout::HOVER_FILL`]. Depth
/// renders as leading guide-line spans (see `guide_spans`).
pub(crate) const ROW: &str = "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface";
/// A tree row's SELECTED/active recipe: [`ROW_GEOM`] + [`crate::v2::apps::editor::shell::layout::TOGGLED_PLATE`] (the
/// lighter primary plate PLUS the 1px dark top border that makes it distinct-by-construction from a
/// hovered [`ROW`]). The border is inside the `h-4` box, so this row is not a pixel taller than [`ROW`].
pub(crate) const ROW_ACTIVE: &str = "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm bg-primary/20 text-primary border-t border-background/60";
/// a FOLDER row that is the **active drop target** (the layer the next placement /
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
/// the palette-leaf variant of [`ROW`]: adds `cursor-grab` (→ `cursor-grabbing` while
/// pressed) so hovering a placeable role advertises the drag affordance. Folders keep `cursor-pointer`
/// and outliner slots keep the plain [`ROW`] default (only palette leaves are drag-to-place). Same
/// [`crate::v2::apps::editor::shell::layout::HOVER_FILL`] as [`ROW`].
pub(crate) const PALETTE_LEAF: &str = "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface cursor-grab active:cursor-grabbing";
/// the non-interactive row kinds (Squad / Comment headers): [`ROW_GEOM`] at the muted rest
/// weight. They are `<div>`s, not buttons, but they occupy the same 16 px pitch — a group header that
/// was a different height from its children is what made the tree read as ragged.
pub(crate) const ROW_STATIC: &str =
    "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm text-on-surface-variant";
/// the "Unfiled" pseudo-root's row: [`ROW_GEOM`] at the faintest weight (it is a virtual
/// bucket, not a doc layer).
pub(crate) const ROW_UNFILED: &str =
    "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm text-outline";
/// an ORBAT faction header: [`ROW_GEOM`] plus the small-caps treatment that marks a section.
pub(crate) const ROW_FACTION: &str = "relative flex h-4 w-full items-center gap-1 rounded px-1.5 text-left text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant";
/// the ORBAT squad-leader badge, sized for the dense row. `ui::badge_class` is the page-level
/// pill (`px-2 py-0.5` ⇒ 22 px with its border) and it burst out of a 16 px row; this is the same
/// primary tint at `h-3` with `leading-none`, so the badge sits INSIDE the row instead of setting its
/// height.
pub(crate) const ROW_BADGE: &str = "inline-flex h-3 shrink-0 items-center rounded border border-primary/30 bg-primary/10 px-1 text-label-sm leading-none text-primary";

/// Hierarchy guide lines — continuous YouTube spines ( A3/A4; supersedes  L-hooks).
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

/// The 16 px row height used by window spacers and shared row classes.
pub(super) const ROW_H: f64 = 16.0;
/// Height used before the live scroller is measured and in native rendering.
pub(super) const CONTAINER_H_FALLBACK: f64 = 420.0;
/// Extra rows rendered on each side of the viewport to cover fast scrolling.
pub(super) const OVERSCAN: usize = 6;
