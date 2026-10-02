//! The two places missions are attached to an operation: before it exists, and after.
//!
//! **Role:** the schedule form's staging list with its picker, and the edit form's live roster with
//! its picker and per-mission detach control.
//! **Position:** one section inside each of the two forms.
//! **Signals & state:** the staging half reads and writes `staged` and `attach_open` and never
//! sends a request — the publish action does that. The roster half reads `hub` and
//! `selected_event`, arms `detach_target`, and attaches through `edit_attach_open` and
//! `attach_busy`.
//! **Invariants:** both pickers hide missions that are already attached, so the common path cannot
//! produce the duplicate the server would refuse. The roster renders only when the answer belongs
//! to the operation currently in focus: a resource keeps serving its previous value while the next
//! run is in flight, and showing that value here would put one operation's detach target under
//! another operation's button. Anything else — never resolved, the shut-form idle state, or an
//! answer for the previous operation — reads as loading.
#![allow(dead_code)]

use super::lifecycle::terrain_label;
use super::state::{Manager, Roster};
use crate::v2::core::api::dto::MissionCard;
use crate::v2::core::ui::MaterialIcon;
use crate::v2::core::utils::datefmt::format_local_datetime;
use leptos::prelude::*;

/// The schedule form's mission section: what is staged, and the picker that stages more.
///
/// Nothing here reaches the network; the staged pairs are attached one by one after the operation
/// has been created.
pub(super) fn staged_missions(st: Manager) -> impl IntoView {
    let Manager {
        missions,
        staged,
        attach_open,
        ..
    } = st;

    view! {
        <div class="mt-6">
            <p class="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                "Missions"
            </p>
            <div class="space-y-2">
                {move || {
                    let list = staged.get();
                    if list.is_empty() {
                        view! {
                            <p class="px-1 text-sm text-on-surface-variant/70">
                                "No missions attached yet."
                            </p>
                        }
                            .into_any()
                    } else {
                        list.into_iter()
                            .map(|(id, title)| {
                                let title_label = title.clone();
                                view! {
                                    <div class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3">
                                        <MaterialIcon name="map" class="text-on-surface-variant" />
                                        <span class="flex-1 text-sm text-on-surface">
                                            {title_label}
                                        </span>
                                        <button
                                            type="button"
                                            on:click=move |_| {
                                                staged.update(|s| s.retain(|(sid, _)| sid != &id))
                                            }
                                            aria-label=format!("Remove {title}")
                                            class="flex size-7 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert"
                                        >
                                            <MaterialIcon name="close" class="text-base" />
                                        </button>
                                    </div>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }
                }}
            </div>

            // + Attach Mission dropdown
            <div class="relative mt-2">
                <button
                    type="button"
                    on:click=move |_| attach_open.update(|o| *o = !*o)
                    class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-sm text-on-surface transition hover:bg-white/5"
                >
                    <MaterialIcon name="add" class="text-base" />
                    "Attach Mission"
                </button>
                {move || {
                    attach_open
                        .get()
                        .then(|| {
                            let available: Vec<MissionCard> = missions
                                .get()
                                .flatten()
                                .map(|p| p.data)
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|m| {
                                    !staged.get().iter().any(|(id, _)| id == &m.id)
                                })
                                .collect();
                            view! {
                                <div class="absolute z-10 mt-2 max-h-64 w-full overflow-y-auto rounded-xl border border-white/10 bg-surface-container-high/95 p-1 shadow-2xl backdrop-blur-xl">
                                    {if available.is_empty() {
                                        view! {
                                            <p class="px-3 py-2 text-sm text-on-surface-variant">
                                                "No more missions in the library."
                                            </p>
                                        }
                                            .into_any()
                                    } else {
                                        available
                                            .into_iter()
                                            .map(|m| {
                                                let id = m.id.clone();
                                                let title = m.title.clone();
                                                let terrain = terrain_label(&m.terrain);
                                                view! {
                                                    <button
                                                        type="button"
                                                        on:click=move |_| {
                                                            staged.update(|s| s.push((id.clone(), title.clone())));
                                                            attach_open.set(false);
                                                        }
                                                        class="flex w-full items-center justify-between gap-2 rounded-lg px-3 py-2 text-left text-sm text-on-surface transition hover:bg-white/5"
                                                    >
                                                        <span class="truncate">{m.title.clone()}</span>
                                                        <span class="shrink-0 font-mono text-xs text-on-surface-variant">
                                                            {terrain}
                                                        </span>
                                                    </button>
                                                }
                                            })
                                            .collect_view()
                                            .into_any()
                                    }}
                                </div>
                            }
                        })
                }}
            </div>
        </div>
    }
}

/// The edit form's mission section: the operation's live roster, and the picker that adds to it.
///
/// Attaching posts immediately against the saved operation, using its own start time as the
/// mission start — the same default the publish path uses.
pub(super) fn attached_missions(st: Manager) -> impl IntoView {
    let Manager {
        store,
        events,
        missions,
        hub,
        selected_event,
        edit_orig,
        detach_target,
        detach_open,
        detach_busy,
        edit_attach_open,
        attach_busy,
        ..
    } = st;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (store, events, hub, edit_orig);

    let on_attach_mission = move |mission_id: String, title: String| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let (Some(id), Some(orig)) =
                (selected_event.get_untracked(), edit_orig.get_untracked())
            else {
                return;
            };
            if attach_busy.get_untracked() {
                return;
            }
            attach_busy.set(true);
            edit_attach_open.set(false);
            let start = orig.start_time.clone();
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_post::<serde_json::Value>(
                    store,
                    &format!("/events/{id}/missions"),
                    serde_json::json!({ "mission_id": mission_id, "start_time": start }),
                )
                .await
                {
                    Ok(_) => {
                        toasts.success(format!("Attached {title}"));
                        hub.refetch();
                        events.refetch();
                    }
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not attach mission",
                    )),
                }
                attach_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (mission_id, title);
        }
    };

    view! {
        <div class="mt-6">
            <p class="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                "Attached Missions"
            </p>
            <div class="space-y-2">
                {move || {
                    let current = selected_event.get();
                    match hub.get() {
                        Some(Roster::Failed) => {
                            view! {
                                <p class="px-1 text-sm text-error-alert">
                                    "Could not load attached missions."
                                </p>
                            }
                                .into_any()
                        }
                        Some(Roster::Loaded(id, ms))
                            if Some(id.as_str()) == current.as_deref() && ms.is_empty() =>
                        {
                            view! {
                                <p class="px-1 text-sm text-on-surface-variant/70">
                                    "No missions attached."
                                </p>
                            }
                                .into_any()
                        }
                        Some(Roster::Loaded(id, ms)) if Some(id.as_str()) == current.as_deref() => {
                            ms
                                .into_iter()
                                .map(|m| {
                                    let emid = m.event_mission_id.clone();
                                    let title = m.title.clone();
                                    let aria = format!("Detach {}", m.title);
                                    let meta = format!(
                                        "{} · {}/{} filled",
                                        format_local_datetime(&m.start_time),
                                        m.filled,
                                        m.total,
                                    );
                                    view! {
                                        <div class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3">
                                            <MaterialIcon name="map" class="text-on-surface-variant" />
                                            <div class="min-w-0 flex-1">
                                                <p class="truncate text-sm text-on-surface">{m.title.clone()}</p>
                                                <p class="mt-0.5 font-mono text-xs text-on-surface-variant">
                                                    {meta}
                                                </p>
                                            </div>
                                            <button
                                                type="button"
                                                on:click=move |_| {
                                                    detach_target.set(Some((emid.clone(), title.clone())));
                                                    detach_open.set(true);
                                                }
                                                prop:disabled=move || detach_busy.get()
                                                aria-label=aria
                                                class="flex size-7 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert disabled:opacity-40"
                                            >
                                                <MaterialIcon name="link_off" class="text-base" />
                                            </button>
                                        </div>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }
                        // Never resolved, the shut-form idle state, or a roster belonging
                        // to the previously selected operation: all of them mean the answer
                        // for this one has not arrived yet.
                        _ => {
                            view! {
                                <p class="px-1 text-sm text-on-surface-variant/70">"Loading…"</p>
                            }
                                .into_any()
                        }
                    }
                }}
            </div>

            // The same control the schedule form has, but it posts straight away instead of
            // staging for publish.
            <div class="relative mt-2">
                <button
                    type="button"
                    data-testid="edit-attach-mission"
                    on:click=move |_| edit_attach_open.update(|o| *o = !*o)
                    prop:disabled=move || attach_busy.get()
                    class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-sm text-on-surface transition hover:bg-white/5 disabled:opacity-50"
                >
                    <MaterialIcon name="add" class="text-base" />
                    {move || {
                        if attach_busy.get() {
                            "Attaching…"
                        } else {
                            "Attach Mission"
                        }
                    }}
                </button>
                {move || {
                    edit_attach_open
                        .get()
                        .then(|| {
                            let current = selected_event.get();
                            let attached: Vec<String> = match hub.get() {
                                Some(Roster::Loaded(id, ms))
                                    if Some(id.as_str()) == current.as_deref() =>
                                {
                                    ms.into_iter().map(|m| m.mission_id).collect()
                                }
                                _ => Vec::new(),
                            };
                            let available: Vec<MissionCard> = missions
                                .get()
                                .flatten()
                                .map(|p| p.data)
                                .unwrap_or_default()
                                .into_iter()
                                .filter(|m| !attached.iter().any(|id| id == &m.id))
                                .collect();
                            view! {
                                <div class="absolute z-10 mt-2 max-h-64 w-full overflow-y-auto rounded-xl border border-white/10 bg-surface-container-high/95 p-1 shadow-2xl backdrop-blur-xl">
                                    {if available.is_empty() {
                                        view! {
                                            <p class="px-3 py-2 text-sm text-on-surface-variant">
                                                "No more missions in the library."
                                            </p>
                                        }
                                            .into_any()
                                    } else {
                                        available
                                            .into_iter()
                                            .map(|m| {
                                                let id = m.id.clone();
                                                let title = m.title.clone();
                                                let terrain = terrain_label(&m.terrain);
                                                view! {
                                                    <button
                                                        type="button"
                                                        on:click=move |_| {
                                                            on_attach_mission(
                                                                id.clone(),
                                                                title.clone(),
                                                            );
                                                        }
                                                        prop:disabled=move || attach_busy.get()
                                                        class="flex w-full items-center justify-between gap-2 rounded-lg px-3 py-2 text-left text-sm text-on-surface transition hover:bg-white/5 disabled:opacity-50"
                                                    >
                                                        <span class="truncate">{m.title.clone()}</span>
                                                        <span class="shrink-0 font-mono text-xs text-on-surface-variant">
                                                            {terrain}
                                                        </span>
                                                    </button>
                                                }
                                            })
                                            .collect_view()
                                            .into_any()
                                    }}
                                </div>
                            }
                        })
                }}
            </div>
        </div>
    }
}
