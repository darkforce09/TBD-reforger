//! Virtual tree for editor outliner trees.

use super::*;

/// render a dock tree, windowed above [`VIRTUAL_SLOT_THRESHOLD`]. Below it the whole
/// flattened list renders eagerly; above it an `h-full` scroll container (: measured from the
/// dock's flex-1 tree region) draws only the visible slice (+ overscan) between two spacer divs, so
/// a mission-scale tree never builds N DOM rows. `stats_key` names this tree in
/// `window.__outlinerStats`.
pub(crate) fn virtual_tree(
    nodes: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    stats_key: &'static str,
    empty_msg: &'static str,
    // enable ORBAT slot→squad pointer-refile in this tree.
    orbat_refile: bool,
    // enable Outliner layer authoring on this tree (Editor Layers dock only): folder-click
    // selection, inline rename, hover delete, pointer-drag reparent/refile, group icon.
    authoring: bool,
) -> AnyView {
    // Per-tree collapse state ( B6). Starts EMPTY = fully expanded, exactly the pre-collapse
    // render — the  windowing smoke's totals depend on the default-expanded boot state.
    let collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    // Inline rename: id signal (list-tracked) + draft signal (input-only).
    let renaming = RwSignal::new(None::<String>);
    let rename_draft = RwSignal::new(String::new());
    // SEL-GROUP-ICON-001 — the "folder directly holds a slot" set, recomputed with the flatten.
    let holds_slots = StoredValue::new(std::collections::HashSet::<String>::new());
    let authoring_ctx = RowAuthoring {
        enabled: authoring,
        holds_slots,
        renaming,
        nodes,
        rename_draft,
    };
    // Flatten once per doc/collapse change (O(n), like the mutation itself); the scroll path only
    // re-slices. Created ONCE per mount (this fn is called outside any reactive closure), so the
    // Effect never leaks — it re-runs on `nodes`/`collapsed` change, and the render `move ||`
    // re-slices on `rev`/scroll.
    let drag_ghost_pos = RwSignal::new(None::<(i32, i32)>);
    // Install once for BOTH eager and windowed trees. Capture sees outside releases even when
    // another panel stops propagation; the zero-delay cleanup runs after the destination drop.
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{closure::Closure, JsCast};
        if let Some(win) = web_sys::window() {
            let release = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |_| {
                if let Some(win) = web_sys::window() {
                    let cleanup = Closure::once_into_js(move || {
                        crate::v2::apps::editor::ui::outliner::drag::cancel_layer_drag();
                        let _ = drag_ghost_pos.try_set(None);
                    });
                    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                        cleanup.unchecked_ref(),
                        0,
                    );
                }
            });
            let cancel = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                crate::v2::apps::editor::ui::outliner::drag::cancel_layer_drag();
                let _ = drag_ghost_pos.try_set(None);
            });
            let _ = win.add_event_listener_with_callback_and_bool(
                "pointerup",
                release.as_ref().unchecked_ref(),
                true,
            );
            // Element blur happens after a trusted row pointerdown focuses that row. Capturing
            // it here would immediately erase the drag just armed by the same press.
            for (event, capture) in [("pointercancel", true), ("blur", false)] {
                let _ = win.add_event_listener_with_callback_and_bool(
                    event,
                    cancel.as_ref().unchecked_ref(),
                    capture,
                );
            }
            let hooks = StoredValue::new_local((win, release, cancel));
            on_cleanup(move || {
                crate::v2::apps::editor::ui::outliner::drag::cancel_layer_drag();
                let _ = hooks.try_with_value(|(win, release, cancel)| {
                    let _ = win.remove_event_listener_with_callback_and_bool(
                        "pointerup",
                        release.as_ref().unchecked_ref(),
                        true,
                    );
                    for (event, capture) in [("pointercancel", true), ("blur", false)] {
                        let _ = win.remove_event_listener_with_callback_and_bool(
                            event,
                            cancel.as_ref().unchecked_ref(),
                            capture,
                        );
                    }
                });
            });
        }
    }
    let flat = StoredValue::new(Vec::<FlatRow>::new());
    let rev = RwSignal::new(0u64);
    Effect::new(move |_| {
        let ns = nodes.get();
        if authoring {
            holds_slots.set_value(folders_holding_slots(&ns));
            // LAYER-CREATE-001 — a create just happened → open that new folder's inline rename.
            // Consumed once (the ops latch clears on read), so a later flatten won't re-arm it.
            #[cfg(target_arch = "wasm32")]
            if let Some(new_id) = engine_ops::take_rename_armed() {
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
    // live scroller height. Starts at the historical 420 px fallback; an Effect reads
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
            // Publishing the same height remounts the scroller, which changes its NodeRef and
            // runs this effect again. Only a real measurement change may invalidate the slice.
            if h > 0.0 && h != container_h.get_untracked() {
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
                                if h > 0.0 && h != container_h.get_untracked() {
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
        // the placed-vehicle rows ride BELOW the layer/slot tree in the LEFT
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
                    on:pointermove=move |ev: web_sys::PointerEvent| {
                        #[cfg(target_arch = "wasm32")]
                        if crate::v2::apps::editor::ui::outliner::drag::PENDING_DRAG.with(|p| p.borrow().is_some()) {
                            drag_ghost_pos.set(Some((ev.client_x(), ev.client_y())));
                        } else if drag_ghost_pos.get_untracked().is_some() {
                            drag_ghost_pos.set(None);
                        }
                    }
                    on:pointerup=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::v2::apps::editor::ui::outliner::drag::cancel_layer_drag();
                        drag_ghost_pos.set(None);
                    }
                    data-testid="outliner-window-scroller"
                    on:scroll=move |ev| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                                let h = el.client_height() as f64;
                                if h > 0.0 && h != container_h.get_untracked() {
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
                    // placed vehicles as a footer under the windowed tree. They ride
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
