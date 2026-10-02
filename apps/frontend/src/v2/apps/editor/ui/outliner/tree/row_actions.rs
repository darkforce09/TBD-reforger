//! Row actions for editor outliner trees.

use super::*;

/// The eye and lock toggle glyphs for a folder row. Each is a `role="button"` span
/// so it nests inside the row `<button>`
/// and `stop_propagation`s, so clicking a glyph flips the flag WITHOUT firing the row's
/// make-active-layer action. The glyph shows this layer's OWN flag (filled = on); when an ANCESTOR
/// carries the flag (inherited, `*_effective && !own`) the icon renders muted and inert, because the
/// flag lives on the parent — you toggle it there, and this row only reflects the inherited state.
/// Trailing on the row (pushed right by a spacer in [`single_row`]).
pub(super) fn layer_flag_toggles(
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
                    engine_ops::set_layer_hidden(&eye_id, !hidden);
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
                    engine_ops::set_layer_locked(&lock_id, !locked);
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

/// per-tree authoring context threaded into [`single_row`]. Grouped into one struct so the
/// row signature stays readable: the editor-layers tree passes `authoring = true` (create/rename/
/// delete/reparent/refile controls live), the ORBAT tree passes `false` (its rows are inert / its
/// own refile is the `orbat_refile` latch). `holds_slots` is the SEL-GROUP-ICON-001 set.
///
/// rename state is TWO signals, not one `(id, draft)` tuple: the tree list may track
/// [`RowAuthoring::renaming`] (which folder is open) so the input mounts/unmounts, but it must
/// NEVER read [`RowAuthoring::rename_draft`]. A draft round-trip through the list-tracked signal
/// remounts the input on every keystroke ( F2 / the  remount trap); the NodeRef
/// `on_load` focus+select then re-selects the whole field and each next char replaces the name.
#[derive(Clone, Copy)]
pub(super) struct RowAuthoring {
    /// Enable the Outliner layer-authoring affordances on this tree (editor-layers only).
    pub(super) enabled: bool,
    /// SEL-GROUP-ICON-001 — folder ids that DIRECTLY hold a slot (distinct glyph).
    pub(super) holds_slots: StoredValue<std::collections::HashSet<String>>,
    /// Which folder id is being inline-renamed; `None` = no edit in flight. List-safe.
    pub(super) renaming: RwSignal<Option<String>>,
    /// Live draft text for the open rename. Input-only — do not read from the list render.
    pub(super) rename_draft: RwSignal<String>,
    pub(super) nodes: RwSignal<Vec<crate::v2::apps::editor::ui::outliner::outliner::OutlinerNode>>,
}

/// the hover row actions on a Folder row: **rename** (arms the inline input) and **delete**
/// (LAYER-DEL-001, behind a confirm). Two `role="button"` spans (like the chevron / flag toggles)
/// so they nest inside the row `<button>` and `stop_propagation` — clicking one never fires the
/// row's select/drop action. Hidden until the row is hovered (`opacity-0 group-hover:opacity-100`).
pub(super) fn folder_row_actions(
    id: &str,
    label: &str,
    renaming: RwSignal<Option<String>>,
    rename_draft: RwSignal<String>,
    nodes: RwSignal<Vec<crate::v2::apps::editor::ui::outliner::outliner::OutlinerNode>>,
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
                        let _ = outliner::delete_layer(&del_id);
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

// Click affordances use the subject router's current resolution for each row id.
// A row with no route stays inert and exposes the refusal reason.

/// Returns the subject id for rows whose click delegates to the subject router.
/// Comment rows route there; slot selection and folder activation use their own actions.
/// The exhaustive kind match requires new row kinds to choose an explicit click path.
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
        .is_some_and(crate::v2::apps::editor::ui::inspector::validation_panel::subject_id_routes)
}

/// Why an unroutable row is inert, in words, rendered as its `title` — the answer available exactly
/// where the click would have been. It names the refusal it actually got, never a kind ban: the
/// dock-left row's old "resolves slots and vehicles only" sentence became a lie the moment the
/// router grew an arm, and this one cannot.
#[must_use]
pub(crate) fn inert_row_reason() -> &'static str {
    "Not selectable right now — the editor's click-to-select router resolves nothing for this row"
}
