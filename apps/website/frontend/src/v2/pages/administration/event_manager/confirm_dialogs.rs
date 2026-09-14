//! The two destructive confirmations: deleting an operation, and detaching a mission from one.
//!
//! **Role:** the confirm before `DELETE /events/:id`, and the confirm before
//! `DELETE /events/:id/missions/:emid`, each with the request it guards.
//! **Position:** siblings of the calendar in the route's view. The detach confirm is rendered
//! **last** of everything on the page.
//! **Signals & state:** the delete confirm reads `confirm_open` and `selected_event` and sets
//! `delete_busy`; the detach confirm reads `detach_open` and `detach_target` and sets
//! `detach_busy`. Both refetch the operation list on success, and detach refetches the roster too,
//! because nothing else changes that would make it re-run.
//! **Invariants:** the two confirmations describe two different things, and the wording is not
//! interchangeable. Deleting an operation hides it — from the schedule, the dashboard and everyone's
//! deployments — and destroys nothing, so it is recoverable. Detaching a mission really does delete
//! that mission's ORBAT slots and every registration on it, so that confirm says the change cannot
//! be undone and means it. The detach confirm comes last in the view because it and the edit form
//! share a stacking level, and document order is what puts it on top of the form that launched it.
#![allow(dead_code)]

use super::lifecycle::{DELETE_EVENT_CONFIRM_DESC, DELETE_EVENT_CONFIRM_TITLE};
use super::state::Manager;
use crate::v2::core::ui::Dialog;
use leptos::prelude::*;

/// The confirmation shown before an operation is deleted.
///
/// The title and body come from the two pinned constants rather than being typed inline: they are
/// an assertion about what the endpoint does, and are held to it by a guard test.
pub(super) fn delete_confirm(st: Manager) -> impl IntoView {
    let Manager {
        store,
        events,
        selected_event,
        confirm_open,
        delete_busy,
        ..
    } = st;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (store, events, selected_event);

    let on_confirm_delete = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let Some(id) = selected_event.get_untracked() else {
                return;
            };
            confirm_open.set(false);
            if delete_busy.get_untracked() {
                return;
            }
            delete_busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_delete(store, &format!("/events/{id}"))
                    .await
                {
                    Ok(()) => {
                        toasts.success("Operation deleted");
                        selected_event.set(None);
                        events.refetch();
                    }
                    Err(_) => toasts.error("Failed to delete operation"),
                }
                delete_busy.set(false);
            });
        }
    };

    view! {
        <Dialog
            open=confirm_open
            title=DELETE_EVENT_CONFIRM_TITLE
            description=DELETE_EVENT_CONFIRM_DESC
        >
            <div class="flex justify-end gap-2">
                <button
                    type="button"
                    on:click=move |_| confirm_open.set(false)
                    class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                >
                    "Cancel"
                </button>
                <button
                    type="button"
                    on:click=on_confirm_delete
                    prop:disabled=move || delete_busy.get()
                    class="rounded-md bg-error-alert/20 px-3 py-1.5 text-label-md text-error-alert transition-colors hover:bg-error-alert/30 disabled:opacity-60"
                >
                    "Delete operation"
                </button>
            </div>
        </Dialog>
    }
}

/// The confirmation shown before a mission is detached from an operation.
///
/// Detaching is a real cascade — the mission's ORBAT slots and every registration on it go with it
/// — so this one is honest about being irreversible. The mission itself stays in the library.
pub(super) fn detach_confirm(st: Manager) -> impl IntoView {
    let Manager {
        store,
        events,
        hub,
        selected_event,
        detach_target,
        detach_open,
        detach_busy,
        ..
    } = st;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (store, events, hub, selected_event);

    let on_confirm_detach = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let (Some(id), Some((emid, _))) = (
                selected_event.get_untracked(),
                detach_target.get_untracked(),
            ) else {
                return;
            };
            detach_open.set(false);
            detach_target.set(None);
            if detach_busy.get_untracked() {
                return;
            }
            detach_busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_delete(
                    store,
                    &format!("/events/{id}/missions/{emid}"),
                )
                .await
                {
                    Ok(()) => {
                        toasts.success("Mission detached");
                        // Both: the roster loses a row and the day list's mission_count drops.
                        hub.refetch();
                        events.refetch();
                    }
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not detach mission",
                    )),
                }
                detach_busy.set(false);
            });
        }
    };

    view! {
        <Dialog
            open=detach_open
            title="Detach this mission?"
            description="The mission's ORBAT slots and every registration on it are deleted. The mission itself stays in the library. This cannot be undone."
        >
            <p class="mb-4 truncate text-sm text-on-surface">
                {move || detach_target.get().map(|(_, title)| title)}
            </p>
            <div class="flex justify-end gap-2">
                <button
                    type="button"
                    on:click=move |_| {
                        detach_open.set(false);
                        detach_target.set(None);
                    }
                    class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                >
                    "Cancel"
                </button>
                <button
                    type="button"
                    on:click=on_confirm_detach
                    prop:disabled=move || detach_busy.get()
                    class="rounded-md bg-error-alert/20 px-3 py-1.5 text-label-md text-error-alert transition-colors hover:bg-error-alert/30 disabled:opacity-60"
                >
                    "Detach mission"
                </button>
            </div>
        </Dialog>
    }
}
