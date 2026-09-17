//! Menu dispatch.

use super::*;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static MENU: std::cell::RefCell<Option<RwSignal<Option<MenuState>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Registers the page signal that owns the open menu.
#[cfg(target_arch = "wasm32")]
pub fn set_menu_signal(sig: RwSignal<Option<MenuState>>) {
    MENU.with(|m| *m.borrow_mut() = Some(sig));
}

/// Opens the context menu at a viewport point.
#[cfg(target_arch = "wasm32")]
pub fn open(x: f64, y: f64, target: MenuTarget) {
    use website_map_engine::editing::hosted_commands as engine_ops;

    if let Some(id) = target.retarget_to.clone() {
        entity_selection::select_slot(id);
    }
    let target = target.with_armed_connect(engine_ops::pending_connect());
    MENU.with(|m| {
        if let Some(sig) = *m.borrow() {
            sig.set(Some(MenuState {
                x,
                y,
                target,
                open_submenu: None,
            }));
        }
    });
}

/// Toggles the selected submenu while keeping the menu open.
#[cfg(target_arch = "wasm32")]
pub fn toggle_submenu(parent: ContextItem) {
    MENU.with(|m| {
        if let Some(sig) = *m.borrow() {
            if let Some(mut state) = sig.get_untracked() {
                state.open_submenu = if state.open_submenu == Some(parent) {
                    None
                } else {
                    Some(parent)
                };
                sig.set(Some(state));
            }
        }
    });
}

/// Closes the active context menu.
#[cfg(target_arch = "wasm32")]
pub fn close() {
    MENU.with(|m| {
        if let Some(sig) = *m.borrow() {
            sig.set(None);
        }
    });
}

/// Runs the enabled action represented by a menu row.
#[cfg(target_arch = "wasm32")]
pub fn dispatch(item: ContextItem, target_ids: &[String], world: Option<(f64, f64)>) {
    use website_map_engine::editing::hosted_commands as engine_ops;

    match item {
        ContextItem::GoHere => {
            entity_selection::center_on_selection();
        }
        ContextItem::Attributes => {
            if let Some(id) = target_ids.first() {
                crate::v2::apps::editor::bridge::host_state::editor_context::open_attributes(
                    id.clone(),
                );
            }
        }
        ContextItem::EditLoadout => {
            if let Some(id) = target_ids.first() {
                crate::v2::apps::editor::bridge::host_state::editor_context::open_arsenal(
                    id.clone(),
                );
            }
        }
        ContextItem::PlaceComment => {
            if let Some((x, z)) = world {
                let _ = engine_ops::place_comment(x, z, outliner::ensure_active_layer);
            }
        }
        ContextItem::ConnectStart(kind) => {
            if let Some(id) = target_ids.first() {
                let _ = engine_ops::arm_connect(kind.token(), id);
            }
        }
        ContextItem::ConnectComplete => {
            if let Some(id) = target_ids.first() {
                let _ = engine_ops::complete_connect(id);
            }
        }
        ContextItem::ConnectCancel => engine_ops::cancel_connect(),
        ContextItem::ShowConnections => {
            crate::v2::apps::editor::bridge::host_state::editor_context::open_connections_panel()
        }
        ContextItem::MoveToFormation(f) => {
            if let Some(id) = target_ids.first() {
                let _ = engine_ops::force_to_formation(id, f.token());
            }
        }
        ContextItem::ArrangeRun(kind) => {
            crate::v2::apps::editor::ui::docks::top_strip::run_arrange(kind);
        }
        _ => {}
    }
    close();
}
