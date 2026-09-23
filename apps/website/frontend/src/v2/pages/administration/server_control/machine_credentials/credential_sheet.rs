//! The credential sheet: the secret just issued, the form that issues one, and every credential.
//!
//! **Role:** renders the sheet the server card opens — the one-time secret with its copy control and
//! its warning, the issue form (a label and the program the credential is for), and the list of the
//! server's credentials with each one's program, label, issue, last use or revocation, and the
//! revoke control that asks for a reason.
//! **Position:** a side sheet over the server control screen.
//! **Signals & state:** reads and writes the [`CredentialPanel`]; owns the issue form's fields and,
//! per row, whether the revoke form is open and the reason typed into it.
//! **Invariants:** the secret is shown in one place, once, with a warning that it cannot be shown
//! again; it is copied through the crate's one clipboard path, which reports a copy only once the
//! browser confirms it, and it can also be selected by hand. A revocation cannot be sent without a
//! reason. Every request is browser-only.

use super::credential_text::{
    executor_kind, executor_label, issued_line, listing_order, standing_line, validated_label,
    validated_reason, EXECUTOR_KINDS,
};
use super::{CredentialList, CredentialPanel};
use crate::v2::core::api::dto::MachineCredential;
use crate::v2::core::ui::{badge_class, MaterialIcon, Sheet};
use leptos::prelude::*;

/// Shared styling for the sheet's fields.
const FIELD: &str = "w-full rounded-md border border-outline-variant/40 bg-surface px-3 py-1.5 text-sm text-on-surface outline-none focus:border-primary/60";

/// The credential sheet of one server.
pub(in super::super) fn credential_sheet(panel: CredentialPanel) -> impl IntoView {
    let me = StoredValue::new(panel.store.user.get_untracked().map(|u| u.discord_id));
    view! {
        <Sheet open=panel.open bleed=true class="w-full max-w-none md:w-[40rem]">
            <div class="flex h-full flex-col">
                <header class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"Machine credentials"</h2>
                        <p class="mt-1 truncate text-label-md text-on-surface-variant">
                            {panel.server_name.get_value()}
                        </p>
                    </div>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| panel.open.set(false)
                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <MaterialIcon name="close" />
                    </button>
                </header>
                <div class="custom-scrollbar flex-1 space-y-6 overflow-y-auto px-6 py-5">
                    {secret_reveal(panel)}
                    {issue_form(panel)}
                    {move || match panel.list.get() {
                        CredentialList::Loaded(list) if list.is_empty() => view! {
                            <p class="text-sm text-on-surface-variant">"This server has no credentials yet."</p>
                        }
                        .into_any(),
                        CredentialList::Loaded(list) => view! {
                            <ul class="space-y-2" data-testid="machine-credentials">
                                {listing_order(list)
                                    .into_iter()
                                    .map(|credential| credential_row(panel, credential, me))
                                    .collect_view()}
                            </ul>
                        }
                        .into_any(),
                        CredentialList::Failed(why) => view! { <p class="text-sm text-error-alert">{why}</p> }.into_any(),
                        CredentialList::Loading | CredentialList::Idle => view! {
                            <p class="text-sm text-on-surface-variant">"Loading credentials…"</p>
                        }
                        .into_any(),
                    }}
                </div>
            </div>
        </Sheet>
    }
}

/// The secret an issue just answered with, shown once.
fn secret_reveal(panel: CredentialPanel) -> impl IntoView {
    move || {
        panel.issued.get().map(|issued| {
            let secret = StoredValue::new(issued.secret.clone());
            let copy = move |_| {
                #[cfg(target_arch = "wasm32")]
                crate::v2::core::utils::clipboard::write_clipboard(
                    secret.get_value(),
                    "Secret copied".to_string(),
                    crate::v2::core::ui::toast::use_toasts(),
                );
                #[cfg(not(target_arch = "wasm32"))]
                let _ = secret;
            };
            view! {
                <section
                    class="rounded-xl border border-tactical-yellow/40 bg-tactical-yellow/10 p-4"
                    role="alert"
                    data-testid="issued-secret"
                >
                    <p class="flex items-center gap-2 text-sm font-semibold text-tactical-yellow">
                        <MaterialIcon name="warning" class="text-base" />
                        "Store this secret now — it cannot be shown again"
                    </p>
                    <p class="mt-1 text-xs text-on-surface-variant">
                        {format!(
                            "It authenticates the {} credential \"{}\" of this server. Closing this sheet discards it.",
                            executor_label(&issued.executor_kind).to_lowercase(),
                            issued.label
                        )}
                    </p>
                    <div class="mt-3 flex gap-2">
                        <input
                            readonly
                            aria-label="Issued secret"
                            prop:value=issued.secret.clone()
                            on:focus=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    use wasm_bindgen::JsCast;
                                    if let Some(input) = ev
                                        .target()
                                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                    {
                                        input.select();
                                    }
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = ev;
                            }
                            class="min-w-0 flex-1 rounded-md border border-outline-variant/40 bg-surface px-3 py-1.5 font-mono text-xs text-on-surface"
                        />
                        <button
                            type="button"
                            on:click=copy
                            class="flex items-center gap-1.5 rounded-full bg-action px-4 py-1.5 text-sm font-medium text-on-action"
                        >
                            <MaterialIcon name="content_copy" class="text-base" />
                            "Copy"
                        </button>
                    </div>
                    <button
                        type="button"
                        on:click=move |_| panel.issued.set(None)
                        class="mt-3 text-xs text-on-surface-variant hover:underline"
                    >
                        "I have stored it — hide the secret"
                    </button>
                </section>
            }
        })
    }
}

/// The form that issues a credential.
fn issue_form(panel: CredentialPanel) -> impl IntoView {
    let label = RwSignal::new(String::new());
    let kind = RwSignal::new(EXECUTOR_KINDS[0].1.to_string());
    let issue = move |_| {
        let toasts = crate::v2::core::ui::toast::use_toasts();
        let (label_text, Some(executor_kind)) = (
            validated_label(&label.get_untracked()),
            executor_kind(&kind.get_untracked()),
        ) else {
            return;
        };
        match label_text {
            Ok(label_text) => {
                #[cfg(target_arch = "wasm32")]
                {
                    label.set(String::new());
                    panel.issue(crate::v2::core::api::dto::MachineCredentialIssue {
                        executor_kind,
                        label: label_text,
                    });
                }
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (label_text, executor_kind);
            }
            Err(problem) => toasts.error(problem),
        }
    };
    view! {
        <section class="rounded-xl border border-white/10 p-4">
            <h3 class="mb-2 text-sm font-semibold text-on-surface">"Issue a credential"</h3>
            <div class="grid gap-2 md:grid-cols-2">
                <input
                    aria-label="Credential label"
                    placeholder="Label, for example the host it runs on"
                    prop:value=move || label.get()
                    on:input=move |ev| label.set(event_target_value(&ev))
                    class=FIELD
                />
                <select
                    aria-label="Program"
                    on:change=move |ev| kind.set(event_target_value(&ev))
                    class=FIELD
                >
                    {EXECUTOR_KINDS
                        .iter()
                        .map(|(_, wire, text)| view! { <option value=*wire>{*text}</option> })
                        .collect_view()}
                </select>
            </div>
            <div class="mt-2 flex justify-end">
                <button
                    type="button"
                    on:click=issue
                    prop:disabled=move || panel.busy.get()
                    class="rounded-full bg-action px-4 py-1.5 text-sm font-medium text-on-action disabled:opacity-50"
                >
                    "Issue credential"
                </button>
            </div>
        </section>
    }
}

/// One credential: its program, label, issue, standing and — while live — its revoke control.
fn credential_row(
    panel: CredentialPanel,
    credential: MachineCredential,
    me: StoredValue<Option<String>>,
) -> impl IntoView {
    let live = credential.revoked_at.is_none();
    let revoking = RwSignal::new(false);
    let reason = RwSignal::new(String::new());
    let id = StoredValue::new(credential.id.clone());
    let (issued, standing) = me.with_value(|me| {
        (
            issued_line(&credential, me.as_deref()),
            standing_line(&credential, me.as_deref()),
        )
    });
    let revoke = move |_| {
        let toasts = crate::v2::core::ui::toast::use_toasts();
        match validated_reason(&reason.get_untracked()) {
            Ok(reason_text) => {
                #[cfg(target_arch = "wasm32")]
                panel.revoke(id.get_value(), reason_text, move || revoking.set(false));
                #[cfg(not(target_arch = "wasm32"))]
                let _ = (reason_text, id);
            }
            Err(problem) => toasts.error(problem),
        }
    };
    view! {
        <li class="rounded-xl border border-white/10 p-3 text-sm">
            <div class="flex flex-wrap items-start justify-between gap-2">
                <div class="min-w-0">
                    <p class="flex items-center gap-2 text-on-surface">
                        <span class=badge_class(if live { "success" } else { "neutral" })>
                            {executor_label(&credential.executor_kind)}
                        </span>
                        {credential.label.clone()}
                    </p>
                    <p class="mt-1 text-xs text-on-surface-variant">{issued}</p>
                    <p class="text-xs text-on-surface-variant">{standing}</p>
                </div>
                {live
                    .then(|| {
                        view! {
                            <button
                                type="button"
                                on:click=move |_| revoking.update(|open| *open = !*open)
                                class="rounded-full border border-error-alert/30 px-3 py-1 text-xs text-error-alert hover:bg-error-alert/10"
                            >
                                {move || if revoking.get() { "Keep" } else { "Revoke" }}
                            </button>
                        }
                    })}
            </div>
            {move || {
                revoking
                    .get()
                    .then(|| {
                        view! {
                            <div class="mt-3 space-y-2 rounded-lg border border-error-alert/30 bg-error-alert/10 p-3">
                                <p class="text-xs text-error-alert">
                                    "Revoking stops this credential at once and ends every game-runtime session it authenticated. The server's other credentials are untouched."
                                </p>
                                <textarea
                                    aria-label="Reason for revoking"
                                    rows="2"
                                    placeholder="Why it is revoked (kept in the audit trail)"
                                    prop:value=move || reason.get()
                                    on:input=move |ev| reason.set(event_target_value(&ev))
                                    class=FIELD
                                ></textarea>
                                <div class="flex justify-end">
                                    <button
                                        type="button"
                                        on:click=revoke
                                        prop:disabled=move || panel.busy.get() || reason.with(|r| r.trim().is_empty())
                                        class="rounded-full bg-error-alert/20 px-4 py-1.5 text-sm text-error-alert disabled:opacity-50"
                                    >
                                        "Revoke credential"
                                    </button>
                                </div>
                            </div>
                        }
                    })
            }}
        </li>
    }
}
