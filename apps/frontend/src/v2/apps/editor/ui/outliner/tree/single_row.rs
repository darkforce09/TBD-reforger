//! Single row for editor outliner trees.

use super::*;

/// Render ONE flattened outliner row (no recursion — the windowed list draws a flat slice).
/// Header kinds (Unfiled / Faction) are inert; Squad is a refile drop target when `orbat_refile`;
/// Folder → active-layer + folder-click selection; Slot → select + dbl-click→Attributes
/// (SEL-ORBAT-DBL-001); Comment → the click-to-select router, iff [`row_routes`] says so.
pub(super) fn single_row(
    row: &FlatRow,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    collapsed: RwSignal<std::collections::HashSet<String>>,
    // when true, slot pointerdown arms refile; squad pointerup completes it.
    orbat_refile: bool,
    // layer-authoring context (create/rename/delete/reparent/refile + group icon).
    authoring: RowAuthoring,
) -> AnyView {
    let label = row.label.clone();
    let aria = row.label.clone();
    let id = row.id.clone();
    let is_leader = row.is_leader;
    // /per-row guide continuation + click-to-toggle owners.
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
                            // same two-step as the folder drop: consume the set
                            // if one is armed, else fall back to the single-id completion.
                            #[cfg(target_arch = "wasm32")]
                            {
                                if !crate::v2::apps::editor::ui::outliner::drag::complete_multi_refile_onto_squad(&dest) {
                                    engine_ops::complete_refile_onto_squad(dest.clone());
                                }
                            }
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
            // dim a folder that is effectively hidden (own or inherited); the eye/lock
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
            // is THIS folder being inline-renamed?
            // track ONLY `renaming` (the id). Reading a draft-bearing signal here would
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
                // `autofocus` alone does NOT focus this input. The row is inserted by a
                // reactive `{move || …}` re-render, not present at parse time, and the browser only
                // honours the `autofocus` content attribute for elements in the initial parse /
                // first document insertion — a node created by a later reactive update is skipped
                // (same mechanism note as eden_dock_left.rs ). `on_load` fires once when
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
                            let _ = engine_ops::rename_layer(&id, &text);
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
                    // the drop-target folder reads as ROW_DROP_TARGET, distinct from
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
            // folder click: select the folder's DIRECT slot children
            // (SEL-LAYER-CHILDREN-001) AND keep the  "make this the drop target" behavior. A
            // modifier (Alt or Shift) selects ALL descendants instead (SEL-LAYER-DESC-001 — the
            // "second affordance"). Both read the UNFILTERED doc (see `editor_ops` selectors).
            let id_click = id.clone();
            let authoring_on = authoring.enabled;
            let click = move |ev: web_sys::MouseEvent| {
                #[cfg(target_arch = "wasm32")]
                {
                    outliner::set_active_layer(Some(id_click.clone()));
                    if authoring_on {
                        if ev.alt_key() || ev.shift_key() {
                            entity_selection::select_layer_descendants(&id_click);
                        } else {
                            entity_selection::select_layer_children(&id_click);
                        }
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (&id_click, &ev, authoring_on);
            };
            // pointer-drag reparent: arm this folder on pointerdown; a drop onto another
            // folder reparents it, and dropping onto the header root-dropzone reparents to root.
            // Same pointer idiom as ORBAT refile (the Leptos frontend has no HTML5-DnD lane).
            let id_down = id.clone();
            let id_up = id.clone();
            let authoring_dnd = authoring.enabled;
            // the drop handler's own read of the row tree, for the
            // "is the destination inside a dragged row's own subtree?" question `plan_drop` asks.
            // Captured as the signal (not a snapshot) so the drop sees the tree as it stands at
            // RELEASE — a drop is decided against what is on screen now, not at arm time.
            let drop_nodes = authoring.nodes;
            // Hover row actions (rename / delete) — . `group`/`group-hover` reveal them.
            let row_actions = if authoring.enabled {
                folder_row_actions(
                    &id,
                    &label,
                    authoring.renaming,
                    authoring.rename_draft,
                    authoring.nodes,
                )
            } else {
                ().into_any()
            };
            // the active-drop-target folder wears ROW_DROP_TARGET (tertiary plate +
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
            // the "stated in the row UI" half of fold (c): a small target glyph that
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
                            // one shared set builder for all three arms.
                            let drag = drag_set_for(
                                &id_down,
                                &selected.get_untracked(),
                                &authoring.nodes.get_untracked(),
                            );
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::v2::apps::editor::ui::outliner::drag::begin_layer_drag(drag);
                                // The single-id latch stays armed BESIDE the set: the header's
                                // root dropzone still completes through
                                // `complete_layer_drop_onto_root`, which reads only that latch.
                                // Whichever drop claims the release clears both.
                                engine_ops::begin_layer_drag(id_down.clone());
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = drag;
                        }
                    }
                    on:pointerup=move |ev: web_sys::PointerEvent| {
                        if authoring_dnd {
                            ev.stop_propagation();
                            // Consume the multi-select drag set armed on pointerdown.
                            //
                            // The fallback is not decoration: `complete_multi_drop_onto_folder`
                            // returns false only when NO set was armed — a drag begun by some
                            // other row kind that still arms the single-id latch — and
                            // `complete_layer_drop_onto_folder` is the correct completion for
                            // exactly that case. Whichever runs, the drop is claimed once.
                            #[cfg(target_arch = "wasm32")]
                            {
                                let nodes_now = drop_nodes.get_untracked();
                                let claimed = crate::v2::apps::editor::ui::outliner::drag::complete_multi_drop_onto_folder(
                                    &id_up,
                                    |id| node_descendant_ids(&nodes_now, id),
                                );
                                if !claimed {
                                    let _ = engine_ops::complete_layer_drop_onto_folder(id_up.clone());
                                }
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
        // Comment selection and editing use the dedicated row renderer.
        NodeKind::Comment => comment_row(row, selected, collapsed, toggle, authoring),
        NodeKind::Slot => {
            let is_sel = {
                let id = id.clone();
                move || selected.get().iter().any(|s| s == &id)
            };
            let id_dbl = id.clone();
            let id_refile = id.clone();
            let id_layer_refile = id.clone();
            // the row tree the slot arm orders its DragSet by.
            let drag_nodes = authoring.nodes;
            let authoring_slot = authoring.enabled;
            // a slot on a hidden layer (or hidden ancestor) renders dimmed; one on a locked
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
                        entity_selection::select_slot(id.clone());
                    }
                    // outliner activate (native dblclick) opens Attributes,
                    // the SEL-ORBAT-DBL-001 contract.
                    on:dblclick=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::v2::apps::editor::bridge::host_state::editor_context::open_attributes(id_dbl.clone());
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &id_dbl;
                    }
                    on:pointerdown=move |_| {
                        // A selected slot arms the selection in render order for a grouped drop.
                        if orbat_refile {
                            // ORBAT tree: arm refile onto a squad.
                            #[cfg(target_arch = "wasm32")]
                            {
                                let drag = drag_set_for(
                                    &id_refile,
                                    &selected.get_untracked(),
                                    &drag_nodes.get_untracked(),
                                );
                                crate::v2::apps::editor::ui::outliner::drag::begin_refile(drag);
                                engine_ops::begin_refile(id_refile.clone());
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_refile;
                        } else if authoring_slot {
                            // Editor-Layers tree: arm refile of this slot into a folder
                            // (a folder-row `pointerup` completes it via `move_slot_to_layer`).
                            #[cfg(target_arch = "wasm32")]
                            {
                                let drag = drag_set_for(
                                    &id_layer_refile,
                                    &selected.get_untracked(),
                                    &drag_nodes.get_untracked(),
                                );
                                crate::v2::apps::editor::ui::outliner::drag::begin_layer_slot_drag(drag);
                                engine_ops::begin_layer_slot_drag(id_layer_refile.clone());
                            }
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

// ──  (fold c, F-22) — placed vehicles belong in the LEFT outliner, beside slots/groups ────────
//
// The eye-pass fold: a placed VEHICLE is a thing on the map, and things on the map live in the left
