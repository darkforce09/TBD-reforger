//! The member typeahead a squad manager fills an empty slot with.
//!
//! **Role:** renders the inline search field and result list under a slot row, and assigns the
//! picked member to that slot.
//! **Position:** expanded beneath one slot row of the squad pane, for the manager who opened it.
//! **Signals & state:** owns the query signal, the member search resource keyed on it, and a
//! busy flag; closes the picker by clearing the selector's open-picker signal on success.
//! **Invariants:** the search runs on every keystroke, so the query is URL-encoded before it
//! reaches the path. The search and the assignment are browser-only paths; a native build shows
//! the field and lists nothing.
#![allow(dead_code)]

use crate::v2::core::api::dto::Member;
use crate::v2::core::ui::DEFAULT_AVATAR;
use leptos::prelude::*;

// The member search only runs in the browser.
#[cfg(target_arch = "wasm32")]
use crate::v2::core::api::dto::DataEnvelope;

/// The inline member typeahead for filling one slot of a squad the caller manages.
///
/// `assigning` is the selector's open-picker signal — cleared once a member is assigned — and
/// `changed` reloads the order of battle and whatever mounted it.
#[component]
pub(super) fn AssignPicker(
    emid: String,
    slot_id: String,
    assigning: RwSignal<Option<String>>,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    // Stored values so `on_pick` stays `Copy`: it is used inside the reactive result list.
    let emid = StoredValue::new(emid);
    let slot_id = StoredValue::new(slot_id);
    let q = RwSignal::new(String::new());
    let members = LocalResource::new(move || {
        let q = q.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!(
                    "/members?q={}",
                    js_sys::encode_uri_component(&q)
                        .as_string()
                        .unwrap_or_default()
                );
                crate::v2::core::api::client::api_get::<DataEnvelope<Member>>(store, &path)
                    .await
                    .ok()
                    .map(|e| e.data)
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, q);
                None::<Vec<Member>>
            }
        }
    });
    let assign_busy = RwSignal::new(false);
    // All of these feed only the browser-only assignment request.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (assign_busy, store, emid, slot_id, assigning, &changed);
    let on_pick = move |m: Member| {
        #[cfg(target_arch = "wasm32")]
        {
            if assign_busy.get_untracked() {
                return;
            }
            assign_busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path = format!(
                "/event-missions/{}/slots/{}/assign",
                emid.get_value(),
                slot_id.get_value()
            );
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_put::<serde_json::Value>(
                    store,
                    &path,
                    serde_json::json!({ "discord_id": m.discord_id }),
                )
                .await
                {
                    Ok(_) => {
                        toasts.success(format!("Assigned {}", m.username));
                        assigning.set(None);
                        changed.run(());
                    }
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not assign member",
                    )),
                }
                assign_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = m;
    };

    view! {
        <div class="border-t border-border-subtle bg-surface p-2">
            <input
                autofocus
                prop:value=move || q.get()
                on:input=move |ev| q.set(event_target_value(&ev))
                placeholder="Search members…"
                class="w-full rounded-lg border border-border-subtle bg-surface-container px-3 py-1.5 text-sm"
            />
            <ul class="mt-2 max-h-40 overflow-y-auto">
                {move || {
                    members
                        .get()
                        .flatten()
                        .map(|list| {
                            if list.is_empty() {
                                view! {
                                    <li class="px-2 py-1 text-xs text-on-surface-variant">
                                        "No matching members."
                                    </li>
                                }
                                    .into_any()
                            } else {
                                list.into_iter()
                                    .map(|m| {
                                        let avatar = m
                                            .avatar_url
                                            .as_deref()
                                            .map(crate::v2::core::utils::safe_avatar_url)
                                            .unwrap_or_else(|| DEFAULT_AVATAR.to_string());
                                        let username = m.username.clone();
                                        let pick = m.clone();
                                        view! {
                                            <li>
                                                <button
                                                    type="button"
                                                    on:click=move |_| on_pick(pick.clone())
                                                    class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm hover:bg-surface-container-high"
                                                >
                                                    <img src=avatar alt="" class="h-5 w-5 rounded-full" />
                                                    {username}
                                                </button>
                                            </li>
                                        }
                                    })
                                    .collect_view()
                                    .into_any()
                            }
                        })
                }}
            </ul>
        </div>
    }
}
