//! Dialog lifecycle for the ORBAT manager.

use super::*;

/// Registers the dialog with the modal stack and refreshes its faction library.
/// Provides install orbat dialog lifecycle for the ORBAT dialog.
pub(super) fn install_orbat_dialog_lifecycle(
    open: RwSignal<bool>,
    library: RwSignal<Vec<UserFaction>>,
    #[cfg(target_arch = "wasm32")] auth: crate::v2::core::auth::AuthStore,
) -> u64 {
    let modal_id = crate::v2::core::ui::modal_stack::register(move || {
        open.try_get_untracked().unwrap_or(false)
    });
    let esc = window_event_listener(leptos::ev::keydown, move |ev| {
        if open.get_untracked()
            && ev.key() == "Escape"
            && crate::v2::core::ui::modal_stack::is_topmost_open(modal_id)
        {
            open.set(false);
        }
    });
    on_cleanup(move || {
        esc.remove();
        crate::v2::core::ui::modal_stack::unregister(modal_id);
        crate::v2::apps::editor::ui::outliner::drag::cancel_layer_drag();
    });
    Effect::new(move |_| {
        if !open.get() {
            crate::v2::apps::editor::ui::outliner::drag::cancel_layer_drag();
        }
    });

    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if !open.get() {
                return;
            }
            leptos::task::spawn_local(async move {
                if let Ok(r) = crate::v2::core::api::client::api_get::<
                    crate::v2::core::api::dto::FactionListResponse,
                >(auth, "/factions")
                .await
                {
                    library.set(r.data);
                }
            });
        });
    }
    modal_id
}
