//! Changing a member's standing: the role editor, and the two reason-bearing confirmations.
//!
//! **Role:** the inline role picker and the request behind it, and the ban and warning dialogs
//! with the reason each of them requires.
//! **Position:** inside the dossier pane — the picker in its body, the two dialogs at its foot.
//! **Signals & state:** the picker writes `role` and restores the previous value when the request
//! is refused. The dialogs read and write their own `*_open`, `*_reason` and `*_busy` signals, bump
//! `warnings` or set `banned` on success, and refetch the roster so the table agrees with the
//! dossier.
//! **Invariants:** a ban and a warning both **require** a reason. The server refuses a missing or
//! whitespace-only one, and sending a placeholder instead would put an unexplained sanction on a
//! member's record — so the confirm button stays disabled until something has been typed, and the
//! answer is trimmed on the way out because the server trims it too. These are real dialogs rather
//! than the browser's own prompt: a native prompt cannot be driven by the automated gate that
//! checks this screen. A refusal from the server is shown as the server worded it.
#![allow(dead_code)]

use crate::v2::core::auth::AuthStore;
use crate::v2::core::ui::Dialog;
use leptos::prelude::*;

/// Shared field styling for the role picker and the two reason boxes.
pub(super) const INPUT_CLASS: &str = "w-full rounded-lg border border-outline-variant/40 bg-surface px-3 py-2 text-label-md outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/40 disabled:opacity-50";

/// The roles a member can be moved between, as the wire value paired with its label.
pub(super) const ROLE_OPTIONS: [(&str, &str); 4] = [
    ("enlisted", "Enlisted"),
    ("leader", "Leader"),
    ("mission_maker", "Mission Maker"),
    ("admin", "Admin"),
];

/// Path of the ban route for one member; the unban is a delete on the same path.
pub(super) fn admin_user_ban_path(discord_id: &str) -> String {
    format!("/admin/users/{discord_id}/ban")
}

/// Path of the warnings route for one member.
pub(super) fn admin_user_warnings_path(discord_id: &str) -> String {
    format!("/admin/users/{discord_id}/warnings")
}

/// What to do with the reason typed into a ban or warning dialog.
///
/// A plain function rather than an inline check, so the decision can be tested on its own.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum BanReason {
    /// The dialog was cancelled or dismissed: no request, no notification.
    Abort,
    /// Blank or whitespace only. Refused here: sending nothing is what the server rejects, and
    /// substituting a placeholder would put an unexplained sanction on the record. The dialog
    /// disables its confirm while the box is empty, so this branch is defensive.
    Reject,
    /// A real reason, trimmed — the server trims it too, so both agree on what counts as empty.
    Send(String),
}

/// Read a dialog answer into one of the three outcomes.
pub(super) fn classify_ban_reason(answer: Option<&str>) -> BanReason {
    match answer {
        None => BanReason::Abort,
        Some(raw) => {
            let reason = raw.trim();
            if reason.is_empty() {
                BanReason::Reject
            } else {
                BanReason::Send(reason.to_string())
            }
        }
    }
}

/// True when the Dialog Confirm may fire — non-empty after trim (mirrors `BanReason::Send`).
pub(super) fn reason_confirm_enabled(reason: &str) -> bool {
    matches!(classify_ban_reason(Some(reason)), BanReason::Send(_))
}

/// The inline role picker, shown only while the dossier is in role-editing mode.
///
/// Writing the picker updates the shown role at once and sends the change; a refusal puts the
/// previous role back, so the control never claims a promotion the server did not make.
pub(super) fn role_editor(
    store: AuthStore,
    uid: StoredValue<String>,
    prev_role: StoredValue<String>,
    role: RwSignal<String>,
    editing_role: RwSignal<bool>,
    refetch: Callback<()>,
) -> impl IntoView {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (store, uid, prev_role, refetch);

    let on_role_change = move |ev: leptos::ev::Event| {
        let next = event_target_value(&ev);
        role.set(next.clone());
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path = format!("/admin/users/{}", uid.get_value());
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_patch::<serde_json::Value>(
                    store,
                    &path,
                    serde_json::json!({ "role": next }),
                )
                .await
                {
                    Ok(_) => {
                        toasts.success("Role updated");
                        refetch.run(());
                    }
                    Err(e) => {
                        toasts.error(crate::v2::core::api::client::api_error_message(
                            &e,
                            "Failed to update role",
                        ));
                        role.set(prev_role.get_value());
                    }
                }
            });
        }
    };

    view! {
        {move || {
            editing_role
                .get()
                .then(|| {
                    view! {
                        <div class="mt-4">
                            <label class="mb-1 block text-label-sm text-on-surface-variant uppercase">
                                "Role"
                            </label>
                            <select
                                prop:value=move || role.get()
                                on:change=on_role_change
                                class=INPUT_CLASS
                            >
                                {ROLE_OPTIONS
                                    .iter()
                                    .map(|(v, l)| view! { <option value=*v>{*l}</option> })
                                    .collect_view()}
                            </select>
                        </div>
                    }
                })
        }}
    }
}

/// The ban confirmation: the member being banned, the reason box, and the send.
///
/// Confirm stays disabled while the reason is blank, so the rule is stated by the control rather
/// than by an error after the fact.
pub(super) fn ban_dialog(
    store: AuthStore,
    uid: StoredValue<String>,
    banned: RwSignal<bool>,
    ban_open: RwSignal<bool>,
    ban_reason: RwSignal<String>,
    ban_busy: RwSignal<bool>,
    refetch: Callback<()>,
) -> impl IntoView {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (store, uid, refetch);

    let on_confirm_ban = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if banned.get_untracked() || ban_busy.get_untracked() {
                return;
            }
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let reason = match classify_ban_reason(Some(ban_reason.get_untracked().as_str())) {
                BanReason::Abort | BanReason::Reject => return,
                BanReason::Send(reason) => reason,
            };
            ban_busy.set(true);
            let path = admin_user_ban_path(&uid.get_value());
            let body = serde_json::json!({ "reason": reason });
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_post_ok(store, &path, body).await {
                    Ok(()) => {
                        toasts.success("Personnel banned");
                        banned.set(true);
                        ban_open.set(false);
                        ban_reason.set(String::new());
                        refetch.run(());
                    }
                    // The refusal body says exactly what is wrong, so it is shown as written
                    // rather than replaced with a flat "Ban failed".
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Ban failed",
                    )),
                }
                ban_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store, ban_busy, ban_open, ban_reason, banned, refetch);
        }
    };

    view! {
    <Dialog
        open=ban_open
        title="Ban personnel?"
        description="A reason is required. This action is recorded on the roster."
    >
        <label class="mb-1 block text-label-sm text-on-surface-variant uppercase" for="personnel-ban-reason">
            "Ban reason"
        </label>
        <textarea
            id="personnel-ban-reason"
            data-testid="personnel-ban-reason"
            aria-label="Ban reason (required)"
            prop:value=move || ban_reason.get()
            on:input=move |ev| ban_reason.set(event_target_value(&ev))
            placeholder="Reason (required)"
            rows="4"
            class=INPUT_CLASS
        ></textarea>
        <div class="mt-5 flex justify-end gap-2">
            <button
                type="button"
                data-testid="personnel-ban-cancel"
                on:click=move |_| {
                    ban_open.set(false);
                    ban_reason.set(String::new());
                }
                class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
            >
                "Cancel"
            </button>
            <button
                type="button"
                data-testid="personnel-ban-confirm"
                on:click=on_confirm_ban
                prop:disabled=move || {
                    ban_busy.get() || !reason_confirm_enabled(&ban_reason.get())
                }
                class="rounded-md bg-error-alert/20 px-3 py-1.5 text-label-md text-error-alert transition-colors hover:bg-error-alert/30 disabled:cursor-not-allowed disabled:opacity-40"
            >
                {move || if ban_busy.get() { "Banning…" } else { "Ban personnel" }}
            </button>
        </div>
    </Dialog>
    }
}

/// The warning confirmation: the member being warned, the reason box, and the send.
///
/// Same required-reason contract as the ban, and the dossier's warning count is bumped only once
/// the server has accepted it.
pub(super) fn warning_dialog(
    store: AuthStore,
    uid: StoredValue<String>,
    warnings: RwSignal<i64>,
    warn_open: RwSignal<bool>,
    warn_reason: RwSignal<String>,
    warn_busy: RwSignal<bool>,
    refetch: Callback<()>,
) -> impl IntoView {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (store, uid, refetch);

    let on_confirm_warn = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if warn_busy.get_untracked() {
                return;
            }
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let reason = match classify_ban_reason(Some(warn_reason.get_untracked().as_str())) {
                BanReason::Abort | BanReason::Reject => return,
                BanReason::Send(reason) => reason,
            };
            warn_busy.set(true);
            let path = admin_user_warnings_path(&uid.get_value());
            let body = serde_json::json!({ "reason": reason });
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_post_ok(store, &path, body).await {
                    Ok(()) => {
                        toasts.success("Warning issued");
                        warnings.update(|n| *n = n.saturating_add(1));
                        warn_open.set(false);
                        warn_reason.set(String::new());
                        refetch.run(());
                    }
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Warning failed",
                    )),
                }
                warn_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (store, warn_busy, warn_open, warn_reason, warnings, refetch);
        }
    };

    view! {
    <Dialog
        open=warn_open
        title="Issue warning?"
        description="A reason is required. The warning count on this dossier updates after a successful POST."
    >
        <label class="mb-1 block text-label-sm text-on-surface-variant uppercase" for="personnel-warn-reason">
            "Warning reason"
        </label>
        <textarea
            id="personnel-warn-reason"
            data-testid="personnel-warn-reason"
            aria-label="Warning reason (required)"
            prop:value=move || warn_reason.get()
            on:input=move |ev| warn_reason.set(event_target_value(&ev))
            placeholder="Reason (required)"
            rows="4"
            class=INPUT_CLASS
        ></textarea>
        <div class="mt-5 flex justify-end gap-2">
            <button
                type="button"
                data-testid="personnel-warn-cancel"
                on:click=move |_| {
                    warn_open.set(false);
                    warn_reason.set(String::new());
                }
                class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
            >
                "Cancel"
            </button>
            <button
                type="button"
                data-testid="personnel-warn-confirm"
                on:click=on_confirm_warn
                prop:disabled=move || {
                    warn_busy.get() || !reason_confirm_enabled(&warn_reason.get())
                }
                class="rounded-md border border-tactical-yellow/40 bg-tactical-yellow/15 px-3 py-1.5 text-label-md text-tactical-yellow transition-colors hover:bg-tactical-yellow/25 disabled:cursor-not-allowed disabled:opacity-40"
            >
                {move || if warn_busy.get() { "Issuing…" } else { "Issue warning" }}
            </button>
        </div>
    </Dialog>
    }
}
