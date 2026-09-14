//! The mission dossier itself: everything the slide-over shows once the mission has loaded.
//!
//! **Role:** holds the dossier's own state, renders the cinematic header and the sticky footer,
//! and composes the review, shared-body, version, upload, collaboration and management sections
//! in the order an author reads them.
//! **Position:** the content of the library's slide-over sheet, below the sheet chrome.
//! **Signals & state:** the bookmark latch and the three lifecycle busy latches are created here
//! and handed to the sections that share them; the overlay open flags come from the sheet above.
//! Reads the session store and the toast queue from context.
//! **Invariants:** everything that needs the whole mission detail runs before the stored payload
//! is moved out of it, so that payload is never cloned — it is the one value here that reaches
//! hundreds of megabytes. `can_edit` gates what needs the editor; `can_manage` gates the
//! lifecycle, and mirrors the API's own author-or-administrator predicate.

use super::card_grid::{mission_art_url, visibility_badge};
// The bookmark route is reached only from the browser-only half of the toggle below.
#[cfg(target_arch = "wasm32")]
use super::card_grid::bookmark_api_path;
use super::dossier_collaboration::{collaboration_section, comments_sheet, invite_dialog};
use super::dossier_lifecycle::{delete_confirm_dialog, manage_section, returned_section};
use super::dossier_upload::next_semver;
use super::dossier_upload_panel::upload_panel;
use super::dossier_versions::version_history_section;
use crate::v2::core::api::dto::MissionDetail;
use crate::v2::core::ui::MaterialIcon;
use leptos::prelude::*;

/// The loaded dossier.
///
/// `can_edit` gates the things that need the editor — the Mission Creator link, the collaboration
/// controls and the upload panel — because the edit route really does require the mission-maker
/// tier. `can_manage` gates the lifecycle and the review feedback, and deliberately drops that
/// tier to match the API exactly: submit, update and delete each take a plain authenticated user
/// and test author-or-administrator.
#[allow(clippy::too_many_arguments)]
pub(super) fn dossier_sheet_body(
    mut m: MissionDetail,
    id_sv: StoredValue<String>,
    can_edit: bool,
    can_manage: bool,
    sheet_open: RwSignal<bool>,
    comments_open: RwSignal<bool>,
    invite_open: RwSignal<bool>,
    confirm_delete_open: RwSignal<bool>,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    // These feed only the wasm-gated mutation closures.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, id_sv, &changed);
    let art = mission_art_url(m.thumbnail_url.as_deref());
    let is_archived = m.status == "archived";
    let status_busy = RwSignal::new(false);
    let delete_busy = RwSignal::new(false);
    let submit_busy = RwSignal::new(false);

    // Everything that needs the whole mission detail runs first, so the stored payload can then
    // be moved out of it rather than cloned.
    let overview_body = crate::v2::pages::mission_hub::overview::dossier_body(&m);
    let version_rail = version_history_section(&m);
    // Read the current version's number before the payload is moved out below.
    let next_version = next_semver(m.current_version.as_ref().map(|v| v.semver.as_str()));
    let current_payload = m.current_version.take().map(|v| v.json_payload);
    // The dossier star mirrors the mission detail's own named field.
    let bookmarked = RwSignal::new(m.bookmarked);
    let bookmark_busy = RwSignal::new(false);
    // The only two statuses the submit route accepts; everything else is refused. Gating the
    // button on the same predicate means the author never sees an action guaranteed to fail.
    let can_submit = can_manage && (m.status == "draft" || m.status == "rejected");
    // "Resubmit" on a returned mission tells the author the queue accepts a second attempt, which
    // is the whole point of that transition existing.
    let submit_label = if m.status == "rejected" {
        "Resubmit for review"
    } else {
        "Submit for review"
    };
    // The reviewer's note, for the author's own dossier. Empty or absent means there is nothing
    // to show: a reviewer may reject without typing a reason, and the empty string never reaches
    // the wire.
    let rejection_reason = (m.status == "rejected" && can_manage)
        .then(|| {
            m.rejection_reason
                .clone()
                .map(|r| r.trim().to_string())
                .filter(|r| !r.is_empty())
        })
        .flatten();
    let reviewed_at = m.reviewed_at.clone();
    let show_returned = m.status == "rejected" && can_manage;

    // Post when off, delete when on; an optimistic latch, then a list refetch.
    let toggle_bookmark = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if bookmark_busy.get_untracked() {
                return;
            }
            let next = !bookmarked.get_untracked();
            bookmarked.set(next);
            bookmark_busy.set(true);
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path = bookmark_api_path(&id_sv.get_value());
            leptos::task::spawn_local(async move {
                let result = if next {
                    crate::v2::core::api::client::api_post_ok(store, &path, serde_json::json!({}))
                        .await
                } else {
                    crate::v2::core::api::client::api_delete(store, &path).await
                };
                match result {
                    Ok(()) => {
                        toasts.success(if next {
                            "Mission bookmarked"
                        } else {
                            "Bookmark removed"
                        });
                        changed.run(());
                    }
                    Err(e) => {
                        bookmarked.set(!next);
                        toasts.error(crate::v2::core::api::client::api_error_message(
                            &e,
                            if next {
                                "Could not bookmark mission"
                            } else {
                                "Could not remove bookmark"
                            },
                        ));
                    }
                }
                bookmark_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (&store, &changed, id_sv, bookmarked, bookmark_busy);
        }
    };
    let toasts_planner = move |_| {
        #[cfg(target_arch = "wasm32")]
        crate::v2::core::ui::toast::use_toasts().success("2D Tactical Planner — coming soon");
    };
    let goto_editor = move |_| {
        #[cfg(target_arch = "wasm32")]
        if let Some(win) = web_sys::window() {
            let _ = win
                .location()
                .set_href(&format!("/missions/{}/edit", id_sv.get_value()));
        }
    };

    view! {
        // Edge-to-edge cinematic header.
        <div class="relative h-64 w-full shrink-0 md:h-80">
            <img src=art alt="" class="h-full w-full object-cover" />
            <div class="absolute inset-0 bg-gradient-to-t from-surface/90 to-transparent"></div>
            <button
                type="button"
                data-testid="mission-bookmark-toggle"
                aria-label=move || {
                    if bookmarked.get() {
                        "Remove bookmark"
                    } else {
                        "Bookmark mission"
                    }
                }
                prop:disabled=move || bookmark_busy.get()
                on:click=toggle_bookmark
                class="absolute top-5 right-16 flex h-10 w-10 items-center justify-center rounded-full border border-white/10 bg-black/30 text-tactical-yellow backdrop-blur-md transition-colors hover:bg-black/50 disabled:opacity-60"
            >
                {move || {
                    let filled = bookmarked.get();
                    view! {
                        <MaterialIcon name="bookmark" class="text-[20px]" filled=filled />
                    }
                }}
            </button>
            <button
                type="button"
                aria-label="Close"
                on:click=move |_| sheet_open.set(false)
                class="absolute top-5 right-5 flex h-10 w-10 items-center justify-center rounded-full border border-white/10 bg-black/30 text-on-surface backdrop-blur-md transition-colors hover:bg-black/50"
            >
                <span class="material-symbols-outlined">"close"</span>
            </button>
            <div class="absolute right-8 bottom-6 left-8">
                <span class="mb-2 inline-block">{visibility_badge(&m.status)}</span>
                <h2 class="text-4xl font-black tracking-tighter text-white uppercase">
                    {m.title.clone()}
                </h2>
                <p class="mt-1 font-mono text-label-md text-on-surface-variant">
                    {format!("Authored by {}", m.author_name)}
                </p>
            </div>
        </div>

        // Scrollable content — the bottom padding clears the sticky footer.
        <div class="custom-scrollbar flex-1 overflow-y-auto px-8 pt-6 pb-32">
            <div class="space-y-8">
                {returned_section(show_returned, rejection_reason, reviewed_at)}

                {overview_body}

                // The version rail sits above collaboration because what is in the saved version
                // is dossier fact, not a collaboration action.
                {version_rail}

                // The upload panel writes the next version, so it belongs directly under the rail
                // that shows the current one.
                {upload_panel(can_edit, id_sv, changed, next_version, current_payload)}

                {collaboration_section(can_edit, comments_open, invite_open)}

                {manage_section(
                    can_manage,
                    can_submit,
                    submit_label,
                    is_archived,
                    id_sv,
                    changed,
                    status_busy,
                    submit_busy,
                    delete_busy,
                    confirm_delete_open,
                )}
            </div>
        </div>

        // Sticky action footer.
        <div class="absolute right-0 bottom-0 left-0 flex">
            {can_edit
                .then(|| {
                    view! {
                        <button
                            type="button"
                            on:click=goto_editor
                            class="flex-1 bg-action py-5 font-bold tracking-wide text-on-action transition-colors hover:bg-action/80"
                        >
                            "[ OPEN IN MISSION CREATOR ]"
                        </button>
                    }
                })}
            <button
                type="button"
                on:click=toasts_planner
                class="flex-1 border-t border-white/10 bg-surface-container-high py-5 font-bold tracking-wide text-primary transition-colors hover:bg-surface-container-highest"
            >
                "[ LAUNCH TACTICAL PLANNER ]"
            </button>
        </div>

        {comments_sheet(comments_open)}
        {invite_dialog(invite_open)}
        {delete_confirm_dialog(confirm_delete_open, delete_busy, sheet_open, id_sv, changed)}
    }
}
