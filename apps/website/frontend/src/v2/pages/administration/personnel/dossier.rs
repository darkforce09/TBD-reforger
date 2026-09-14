//! The dossier pane: one member's record, and the actions an administrator can take on it.
//!
//! **Role:** the profile header, the four service readings, the role editor, the action buttons,
//! and the two reason-bearing confirmations they open.
//! **Position:** the right pane of the personnel screen, beside the roster table.
//! **Signals & state:** every reading is a signal seeded from the selected row — `role`, `banned`,
//! `warnings` — so the pane updates the moment an action succeeds, without waiting for the roster
//! refetch it also triggers. `ban_busy` and `warn_busy` keep a second click from sending a second
//! request.
//! **Invariants:** a banned member is offered an unban rather than a dead label, so the state is
//! always reversible from the same place it was set. A member with no linked game identity says so
//! instead of showing a blank. The deployment count is shown as the integer the roster carries,
//! including zero.
#![allow(dead_code)]

use super::member_roster::{avatar, display_name};
#[cfg(target_arch = "wasm32")]
use super::role_dialog::admin_user_ban_path;
use super::role_dialog::{ban_dialog, role_editor, warning_dialog};
use crate::v2::core::api::dto::AdminUserRow;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// One member's dossier: profile, readings, role editor, actions and confirmations.
pub(super) fn dossier(u: AdminUserRow, refetch: Callback<()>) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, &refetch);
    let name = display_name(&u);
    let arma = if !u.arma_character.is_empty() {
        u.arma_character.clone()
    } else {
        u.arma_id
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unlinked Arma identity".to_string())
    };
    let uid = StoredValue::new(u.discord_id.clone());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = uid;
    let role = RwSignal::new(u.role.as_str().to_string());
    let prev_role = StoredValue::new(u.role.as_str().to_string());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = prev_role;
    let editing_role = RwSignal::new(false);
    let banned = RwSignal::new(u.is_banned);
    let ban_busy = RwSignal::new(false);
    let warn_busy = RwSignal::new(false);
    let warnings = RwSignal::new(u.warnings);
    // Real dialog fields rather than the browser's own prompt, which the automated gate that
    // checks this screen cannot drive.
    let ban_open = RwSignal::new(false);
    let ban_reason = RwSignal::new(String::new());
    let warn_open = RwSignal::new(false);
    let warn_reason = RwSignal::new(String::new());

    let on_ban = move |_| {
        if banned.get_untracked() || ban_busy.get_untracked() {
            return;
        }
        ban_reason.set(String::new());
        ban_open.set(true);
    };

    let on_unban = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if !banned.get_untracked() || ban_busy.get_untracked() {
                return;
            }
            let toasts = crate::v2::core::ui::toast::use_toasts();
            ban_busy.set(true);
            let path = admin_user_ban_path(&uid.get_value());
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_delete(store, &path).await {
                    Ok(()) => {
                        toasts.success("Personnel unbanned");
                        banned.set(false);
                        refetch.run(());
                    }
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Unban failed",
                    )),
                }
                ban_busy.set(false);
            });
        }
    };

    let on_warn = move |_| {
        if warn_busy.get_untracked() {
            return;
        }
        warn_reason.set(String::new());
        warn_open.set(true);
    };

    view! {
        <div class="flex min-h-0 flex-1 flex-col">
            <div class="custom-scrollbar min-h-0 flex-1 overflow-y-auto p-6">
                <div class="flex flex-col items-center text-center">
                    {avatar(&name, "size-20 text-xl")}
                    <h2 class="mt-4 text-headline-sm text-on-surface">{name.clone()}</h2>
                    <p class="mt-1 font-mono text-code-md text-on-surface-variant">
                        {u.discord_id.clone()}
                    </p>
                    <p class="mt-2 text-label-md text-on-surface-variant">{arma}</p>
                </div>

                <div class="mt-6 grid grid-cols-2 gap-3">
                    {stat("Deployments", u.total_deployments.to_string())}
                    {stat_reactive("Current Rank", move || role.get().to_uppercase())}
                    {stat_reactive("Warnings", move || warnings.get().to_string())}
                    {stat_reactive(
                        "Status",
                        move || if banned.get() { "Banned".into() } else { "Active".into() },
                    )}
                </div>
                {role_editor(store, uid, prev_role, role, editing_role, refetch)}
            </div>

            <div class="flex flex-col gap-2 border-t border-white/10 p-6">
                <button
                    type="button"
                    on:click=move |_| editing_role.update(|v| *v = !*v)
                    class="flex items-center justify-center gap-2 rounded-lg border border-white/10 py-2.5 text-label-md text-on-surface transition hover:bg-white/5"
                >
                    <MaterialIcon name="manage_accounts" class="text-[18px]" />
                    "Edit Roles"
                </button>
                <button
                    type="button"
                    on:click=on_warn
                    prop:disabled=move || warn_busy.get()
                    data-testid="personnel-warn"
                    class="flex items-center justify-center gap-2 rounded-lg border border-tactical-yellow/30 py-2.5 text-label-md text-tactical-yellow transition hover:bg-tactical-yellow/10 disabled:cursor-not-allowed disabled:opacity-40"
                >
                    <MaterialIcon name="warning" class="text-[18px]" />
                    {move || if warn_busy.get() { "Issuing…" } else { "Issue Warning" }}
                </button>
                {move || {
                    if banned.get() {
                        view! {
                            <button
                                type="button"
                                on:click=on_unban
                                prop:disabled=move || ban_busy.get()
                                data-testid="personnel-unban"
                                class="flex items-center justify-center gap-2 rounded-lg border border-success/30 bg-success/10 py-2.5 text-label-md font-medium text-success transition hover:bg-success/20 disabled:cursor-not-allowed disabled:opacity-40"
                            >
                                <MaterialIcon name="lock_open" class="text-[18px]" />
                                {move || if ban_busy.get() { "Unbanning…" } else { "Unban Personnel" }}
                            </button>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button
                                type="button"
                                on:click=on_ban
                                prop:disabled=move || ban_busy.get()
                                data-testid="personnel-ban"
                                class="flex items-center justify-center gap-2 rounded-lg bg-error-alert/15 py-2.5 text-label-md font-medium text-error-alert transition hover:bg-error-alert/25 disabled:cursor-not-allowed disabled:opacity-40"
                            >
                                <MaterialIcon name="gavel" class="text-[18px]" />
                                {move || if ban_busy.get() { "Banning…" } else { "Ban Personnel" }}
                            </button>
                        }
                            .into_any()
                    }
                }}
            </div>

            {ban_dialog(store, uid, banned, ban_open, ban_reason, ban_busy, refetch)}
            {warning_dialog(store, uid, warnings, warn_open, warn_reason, warn_busy, refetch)}
        </div>
    }
}

/// One fixed reading in the dossier's grid.
pub(super) fn stat(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/[0.02] px-3 py-2.5 text-center">
            <p class="text-label-sm text-on-surface-variant uppercase">{label}</p>
            <p class="mt-0.5 truncate text-label-md font-semibold text-on-surface">{value}</p>
        </div>
    }
}

/// One live reading in the dossier's grid, re-rendered whenever its source changes.
pub(super) fn stat_reactive(
    label: &'static str,
    value: impl Fn() -> String + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/[0.02] px-3 py-2.5 text-center">
            <p class="text-label-sm text-on-surface-variant uppercase">{label}</p>
            <p class="mt-0.5 truncate text-label-md font-semibold text-on-surface">{move || value()}</p>
        </div>
    }
}
