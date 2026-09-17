//! Comment row for editor outliner trees.

use super::*;

/// Renders a comment row with selection, drag, and edit affordances.
/// A routable comment uses a keyboard-accessible button; an unresolved id uses an inert element
/// with the refusal reason. Comments route through the subject selector, not slot selection.
pub(super) fn comment_row(
    row: &FlatRow,
    selected: RwSignal<Vec<String>>,
    collapsed: RwSignal<std::collections::HashSet<String>>,
    toggle: AnyView,
    // Authoring context supplies the visible tree order for multi-row drags.
    authoring: RowAuthoring,
) -> AnyView {
    let authoring_enabled = authoring.enabled;
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
        crate::v2::apps::editor::bridge::host_state::editor_context::open_comment_editor(
            id_dbl.clone(),
        );
        #[cfg(not(target_arch = "wasm32"))]
        let _ = &id_dbl;
    };
    let on_down = move |_: web_sys::PointerEvent| {
        if authoring_enabled {
            // a comment row arms the SELECTION too, so refiling a group of
            // notes into a folder is one drag and one Ctrl+Z rather than one drag per note.
            #[cfg(target_arch = "wasm32")]
            {
                let drag = drag_set_for(
                    &id_drag,
                    &selected.get_untracked(),
                    &authoring.nodes.get_untracked(),
                );
                crate::v2::apps::editor::ui::outliner::drag::begin_layer_comment_drag(drag);
                engine_ops::begin_layer_comment_drag(id_drag.clone());
            }
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
                    let _ = crate::v2::apps::editor::ui::inspector::validation_panel::route_select_by_subject_id(&id_click);
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
