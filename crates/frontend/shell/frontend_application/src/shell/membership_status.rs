//! Discord membership freshness warnings and audited administrator access extensions.
//!
//! **Role:** the status strip in the frame's bottom-left corner: it warns when the session runs on
//! cached permissions because Discord verification is delayed, says when an administrative access
//! extension is active, and gives an administrator the control that extends a member's cached
//! access by 48 hours with a recorded reason.
//! **Position:** a frame component of the shell, mounted by `AppLayout` beside the routed content;
//! it reads the session store and calls `GET /me` and
//! `POST /admin/users/{discord_id}/membership-grace` through the transport crate.
//! **Signals & state:** reads the session store's membership flags; owns the reason, target,
//! in-flight and result signals, and a 30-second profile poll that runs while a session is signed
//! in and stops when the strip unmounts.
//! **Invariants:** renders nothing unless the membership is stale or the viewer may manage
//! extensions; at most one profile poll is in flight; an extension is sent only with a non-empty
//! reason and an all-digit Discord account ID.

use frontend_session::AuthStore;
use leptos::prelude::*;

/// Show stale membership status and the reason-bearing access extension controls for administrators.
#[component]
pub fn MembershipStatus() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    let reason = RwSignal::new(String::new());
    let target = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let result = RwSignal::new(String::new());
    {
        let polling = RwSignal::new(false);
        if let Ok(timer) = set_interval_with_handle(
            move || {
                if polling.get_untracked()
                    || store.bootstrapping.get_untracked()
                    || store.access_token.get_untracked().is_none()
                {
                    return;
                }
                polling.set(true);
                let profile_request = store.begin_profile_request();
                leptos::task::spawn_local(async move {
                    if let Ok(profile) = frontend_transport::client::api_get::<
                        frontend_api_dtos::MeResponse,
                    >(store, "/me")
                    .await
                    {
                        store.adopt_profile(profile_request, &profile);
                    }
                    polling.set(false);
                });
            },
            std::time::Duration::from_secs(30),
        ) {
            on_cleanup(move || timer.clear());
        }
    }
    let extend = move |_| {
        if busy.get_untracked() || reason.get_untracked().trim().is_empty() {
            return;
        }
        let Some(user) = store.user.get_untracked() else {
            return;
        };
        let entered = target.get_untracked();
        let target_id = if entered.trim().is_empty() {
            user.discord_id.into_inner()
        } else {
            entered.trim().to_owned()
        };
        if !target_id.chars().all(|c| c.is_ascii_digit()) {
            result.set("Enter a Discord account ID using digits only.".into());
            return;
        }
        let body = serde_json::json!({"reason": reason.get_untracked(), "duration_hours": 48});
        busy.set(true);
        leptos::task::spawn_local(async move {
            match frontend_transport::client::api_post_ok(
                store,
                &format!("/admin/users/{target_id}/membership-grace"),
                body,
            )
            .await
            {
                Ok(()) => {
                    result.set("Cached access extended for 48 hours. The reason is recorded in the audit log.".into());
                    let profile_request = store.begin_profile_request();
                    if let Ok(profile) = frontend_transport::client::api_get::<
                        frontend_api_dtos::MeResponse,
                    >(store, "/me")
                    .await
                    {
                        store.adopt_profile(profile_request, &profile);
                    }
                }
                Err(error) => result.set(error.message_or("Access extension failed")),
            }
            busy.set(false);
        });
    };
    view! {
        <Show when=move || store.membership_stale.get() || store.can_manage_membership_override.get()>
            <aside role="status" class="fixed bottom-3 left-3 z-50 max-w-md rounded-lg border border-outline-variant bg-surface-container p-4 text-body-sm shadow-lg">
                <Show when=move || store.membership_stale.get()>
                    <p>"Discord verification is delayed. We are using your last verified permissions during the grace period. Older snapshots retain Guest access."</p>
                </Show>
                <Show when=move || store.membership_override_active.get()><p>"An administrative access extension is active."</p></Show>
                <Show when=move || store.can_manage_membership_override.get()>
                    <details class="mt-2">
                        <summary>"Extend cached access"</summary>
                        <label class="mt-2 block">"Discord account ID (blank for yourself)"
                            <input class="block w-full bg-surface p-2" on:input=move |event| target.set(event_target_value(&event)) />
                        </label>
                        <label class="mt-2 block">"Reason"
                            <input class="block w-full bg-surface p-2" maxlength="2000" on:input=move |event| reason.set(event_target_value(&event)) />
                        </label>
                        <button class="mt-2 rounded bg-primary px-3 py-2 text-on-primary" on:click=extend
                            disabled=move || busy.get() || reason.get().trim().is_empty()>"Extend for 48 hours"</button>
                    </details>
                </Show>
                <p aria-live="polite">{move || result.get()}</p>
            </aside>
        </Show>
    }
}
