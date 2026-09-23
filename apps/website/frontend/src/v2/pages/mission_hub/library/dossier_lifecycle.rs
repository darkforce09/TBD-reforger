//! The review and lifecycle half of the mission dossier.
//!
//! **Role:** tells an author why their mission came back from review, and gives whoever owns it
//! the three actions that move it: submit for review, archive or restore, and delete.
//! **Position:** two sections inside the dossier's scroll area plus the confirmation dialog the
//! delete button opens, which renders outside that scroll area.
//! **Signals & state:** the archive and delete busy latches are owned by the dossier and passed
//! in, so the Manage row and the confirmation dialog agree on when a delete is in flight; the
//! submission control owns its own. Reads the session store and the toast queue from context;
//! `changed` re-reads the dossier and the card grid.
//! **Invariants:** every gate here mirrors the API's own predicate — author or administrator, and
//! for submission only a draft or a returned mission — so nobody is shown a control that is
//! guaranteed to be refused. A refused submission names its reason and lists every finding the
//! compile reported. All three writes are browser-only.

use crate::v2::core::ui::MaterialIcon;
use crate::v2::pages::mission_hub::mission_review::submission_action::SubmitForReview;
use leptos::prelude::*;

/// The reviewer's verdict, shown above the dossier body because it is why the author opened it.
///
/// The rejection reason is the headline of a returned mission; the review record below the
/// dossier carries the rest of the review. A rejection with no reason says so rather than
/// rendering a blank panel that reads as a loading bug.
pub(super) fn returned_section(
    show_returned: bool,
    rejection_reason: Option<String>,
    reviewed_at: Option<String>,
) -> impl IntoView {
    show_returned.then(|| {
        view! {
                            <section class="rounded-xl border border-error-alert/30 bg-error-alert/10 p-5">
                                <div class="flex items-center gap-2">
                                    <MaterialIcon
                                        name="assignment_return"
                                        class="text-[20px] text-error-alert"
                                    />
                                    <h3 class="font-mono text-label-md tracking-widest text-error-alert uppercase">
                                        "Returned by review"
                                    </h3>
                                </div>
                                {match rejection_reason {
                                    Some(reason) => {
                                        view! {
                                            <p class="mt-3 text-body-md whitespace-pre-line text-on-surface">
                                                {reason}
                                            </p>
                                        }
                                            .into_any()
                                    }
                                    // Rejected with an empty reason. Saying so is better
                                    // than a blank panel that reads as a loading bug.
                                    None => {
                                        view! {
                                            <p class="mt-3 text-body-md text-on-surface-variant italic">
                                                "The reviewer did not leave a reason."
                                            </p>
                                        }
                                            .into_any()
                                    }
                                }}
                                {reviewed_at
                                    .map(|at| {
                                        view! {
                                            <p class="mt-3 font-mono text-label-sm text-on-surface-variant">
                                                "Reviewed " {crate::v2::core::utils::datefmt::format_local_datetime(&at)}
                                            </p>
                                        }
                                    })}
                                <p class="mt-4 text-label-md text-on-surface-variant">
                                    "Address the notes above, then use Submit for review to put it back in the queue."
                                </p>
                            </section>
        }
    })
}

/// The Manage row: submit for review, archive or restore, and delete.
///
/// Gated on `can_manage` — author or administrator, which is the predicate the submit, update and
/// delete routes each enforce — deliberately without the mission-maker tier, which none of them
/// requires. The gap matters: a role sync can demote someone who still owns missions, and they
/// keep full rights over them.
pub(super) fn manage_section(
    can_manage: bool,
    can_submit: bool,
    submit_label: &'static str,
    is_archived: bool,
    id_sv: StoredValue<String>,
    changed: Callback<()>,
    status_busy: RwSignal<bool>,
    delete_busy: RwSignal<bool>,
    confirm_delete_open: RwSignal<bool>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    // These feed only the wasm-gated mutation closures.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, id_sv, &changed);

    let toggle_archive = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if status_busy.get_untracked() {
                return;
            }
            status_busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path = format!("/missions/{}", id_sv.get_value());
            let next = if is_archived { "draft" } else { "archived" };
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_patch::<serde_json::Value>(
                    store,
                    &path,
                    serde_json::json!({ "status": next }),
                )
                .await
                {
                    Ok(_) => {
                        toasts.success(if is_archived {
                            "Mission restored to draft"
                        } else {
                            "Mission archived"
                        });
                        changed.run(());
                    }
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        if is_archived {
                            "Could not unarchive mission"
                        } else {
                            "Could not archive mission"
                        },
                    )),
                }
                status_busy.set(false);
            });
        }
    };

    can_manage.then(|| {
        view! {
                            <section>
                                <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                                    "Manage"
                                </h3>
                                // Primary-styled and on its own line because on a draft this is
                                // the only action that moves the mission forward — nothing else
                                // puts it in front of a reviewer — and a refusal lists findings
                                // under it.
                                {can_submit
                                    .then(|| {
                                        view! {
                                            <div class="mb-2">
                                                <SubmitForReview
                                                    mission_id=id_sv.get_value()
                                                    label=submit_label
                                                    on_submitted=changed
                                                />
                                            </div>
                                        }
                                    })}
                                <div class="flex flex-wrap gap-2">
                                    <button
                                        type="button"
                                        on:click=toggle_archive
                                        prop:disabled=move || status_busy.get()
                                        class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10 disabled:opacity-60"
                                    >
                                        {if is_archived {
                                            "Unarchive (restore to draft)"
                                        } else {
                                            "Archive mission"
                                        }}
                                    </button>
                                    <button
                                        type="button"
                                        on:click=move |_| confirm_delete_open.set(true)
                                        prop:disabled=move || delete_busy.get()
                                        class="rounded-lg border border-error-alert/30 bg-error-alert/10 px-4 py-2 text-label-md text-error-alert transition-colors hover:bg-error-alert/20 disabled:opacity-60"
                                    >
                                        "Delete mission"
                                    </button>
                                </div>
                            </section>
        }
    })
}

/// The confirmation the delete button opens.
///
/// A dialog rather than the browser's own confirm, so the wording can say what deletion actually
/// does and what refuses it.
pub(super) fn delete_confirm_dialog(
    confirm_delete_open: RwSignal<bool>,
    delete_busy: RwSignal<bool>,
    sheet_open: RwSignal<bool>,
    id_sv: StoredValue<String>,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    // These feed only the wasm-gated mutation closure.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, id_sv, &changed, sheet_open);

    let confirm_delete = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            confirm_delete_open.set(false);
            if delete_busy.get_untracked() {
                return;
            }
            delete_busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path = format!("/missions/{}", id_sv.get_value());
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_delete(store, &path).await {
                    Ok(()) => {
                        toasts.success("Mission deleted");
                        sheet_open.set(false);
                        changed.run(());
                    }
                    Err(e) => toasts.error(crate::v2::core::api::client::api_error_message(
                        &e,
                        "Could not delete mission",
                    )),
                }
                delete_busy.set(false);
            });
        }
    };

    view! {
        <crate::v2::core::ui::Dialog
            open=confirm_delete_open
            title="Delete this mission?"
            description="The mission and its versions are removed from the library for everyone. Deletion is refused while the mission is attached to an event."
        >
            <div class="flex justify-end gap-2">
                <button
                    type="button"
                    on:click=move |_| confirm_delete_open.set(false)
                    class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                >
                    "Cancel"
                </button>
                <button
                    type="button"
                    on:click=confirm_delete
                    prop:disabled=move || delete_busy.get()
                    class="rounded-lg bg-error-alert/20 px-4 py-2 text-label-md text-error-alert transition-colors hover:bg-error-alert/30 disabled:opacity-60"
                >
                    "Delete mission"
                </button>
            </div>
        </crate::v2::core::ui::Dialog>
    }
}
