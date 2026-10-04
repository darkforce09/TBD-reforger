//! Reactive page effects and Arrange keyboard shortcuts.

#[cfg(target_arch = "wasm32")]
use super::*;

#[cfg(target_arch = "wasm32")]
/// Updates the estimated compiled mission size after object count changes.
pub(super) fn track_mission_size(obj_count: RwSignal<usize>, sz_bytes: RwSignal<Option<usize>>) {
    use std::cell::Cell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    let timer: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
    Effect::new(move |_| {
        let _ = obj_count.get();
        let Some(win) = web_sys::window() else { return };
        if let Some(id) = timer.get() {
            win.clear_timeout_with_handle(id);
        }
        let timer2 = timer.clone();
        let cb = Closure::once_into_js(move || {
            timer2.set(None);
            sz_bytes.set(
                editor_context::slots_json()
                    .as_deref()
                    .and_then(mission_creator_session::mission_size::estimate_compiled_bytes),
            );
        });
        if let Ok(id) = win
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 500)
        {
            timer.set(Some(id));
        }
    });
}

#[cfg(target_arch = "wasm32")]
/// Installs the window shortcuts for Arrange commands.
pub(super) fn install_arrange_chords() {
    let arrange = window_event_listener(leptos::ev::keydown, move |ev| {
        if mission_history::in_editable_field() {
            return;
        }
        let modk = ev.ctrl_key() || ev.meta_key();
        let handled = match ev.code().as_str() {
            "KeyL" if !modk && ev.alt_key() && !ev.shift_key() => {
                arrange_chord(top_strip::ArrangeKind::AlignLeft)
            }
            "KeyR" if !modk && ev.alt_key() && !ev.shift_key() => {
                arrange_chord(top_strip::ArrangeKind::AlignRight)
            }
            "KeyT" if !modk && ev.alt_key() && !ev.shift_key() => {
                arrange_chord(top_strip::ArrangeKind::AlignTop)
            }
            "KeyB" if !modk && ev.alt_key() && !ev.shift_key() => {
                arrange_chord(top_strip::ArrangeKind::AlignBottom)
            }
            "KeyH" if !modk && ev.alt_key() && !ev.shift_key() => {
                arrange_chord(top_strip::ArrangeKind::SpaceHorizontal)
            }
            "KeyV" if !modk && ev.alt_key() && !ev.shift_key() => {
                arrange_chord(top_strip::ArrangeKind::SpaceVertical)
            }
            _ => false,
        };
        if handled {
            ev.prevent_default();
        }
    });
    on_cleanup(move || arrange.remove());
}

/// Invalidates the transform widget when selection changes.
#[cfg(target_arch = "wasm32")]
pub(super) fn update_widget_tick(selected_ids: RwSignal<Vec<String>>, widget_tick: RwSignal<u64>) {
    Effect::new(move |_| {
        let _ = selected_ids.get();
        widget_tick.update(|t| *t = t.wrapping_add(1));
    });
}

/// Rebuilds the active side catalog when registry data changes.
#[cfg(target_arch = "wasm32")]
pub(super) fn update_catalog(
    active_side: RwSignal<String>,
    registry_items: RwSignal<Option<Vec<frontend_api_dtos::RegistryItem>>>,
    catalog: RwSignal<mission_creator_state::asset_catalog::CatalogState>,
) {
    use mission_creator_state::asset_catalog::{CatalogState, build_catalog_tree};
    Effect::new(move |_| {
        let side = active_side.get();
        if let Some(items) = registry_items.get() {
            catalog.set(CatalogState::Ready(build_catalog_tree(&items, &side)));
        }
    });
}
