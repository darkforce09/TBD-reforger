//! The review workspace route: the Scenario Creator opened read-only on exactly the version an
//! artifact compiled from.
//!
//! **Role:** reads the workspace of one artifact — the artifact and its version, verified by the
//! backend against the payload digest the artifact recorded — opens the editor's review mode on it,
//! and mounts the editor with the review banner over it.
//! **Position:** the chromeless `/missions/:id/artifacts/:artifact_id/workspace` route, behind the
//! sign-in gate; linked from the approvals drawer and from the review record.
//! **Signals & state:** the workspace lives in a `LocalResource` keyed on nothing reactive, so it is
//! read once per visit. The editor's review mode is a cell the editor's write paths consult; this
//! route opens it before the editor mounts and closes it when the route goes away.
//! **Invariants:** the editor mounts only once the workspace has been read, so its boot always finds
//! the reviewed version rather than restoring a draft. Only the mission's author and administrators
//! are served a workspace; anyone else is told so instead of being shown an empty editor.

use super::banner::review_banner;
use crate::v2::apps::editor::mission_editor::MissionEditorPage;
use crate::v2::apps::editor::shell::review_mode::{self, ReviewedVersion};
use crate::v2::core::api::dto::ReviewWorkspace;
use crate::v2::core::ui::{AuthGate, MaterialIcon};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

/// The workspace as read: answered, or refused with the sentence to show.
#[derive(Clone)]
enum WorkspaceRead {
    Loaded(Box<ReviewWorkspace>),
    Failed(String),
}

/// What a refused workspace read is told, by the status the backend answered.
pub(crate) fn workspace_failure_sentence(status: u16, message: Option<&str>) -> String {
    match status {
        401 => "Your session has ended — sign in again to open the review workspace.".to_string(),
        403 => "Only the mission's author and administrators can open its review workspace."
            .to_string(),
        404 => "There is no such artifact of this mission, or the mission is not visible to you."
            .to_string(),
        _ => crate::v2::core::api::client::api_error_message(
            &(status, message.map(str::to_string)),
            "The review workspace could not be opened",
        ),
    }
}

/// The review workspace, behind the sign-in gate.
#[component]
pub fn ReviewWorkspacePage() -> impl IntoView {
    view! {
        <AuthGate>
            <ReviewWorkspaceInner />
        </AuthGate>
    }
}

/// The read, then the editor in review mode with its banner.
#[component]
fn ReviewWorkspaceInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let params = use_params_map().get_untracked();
    let param = |key: &str| params.get(key).map(|s| s.to_string()).unwrap_or_default();
    let ids = StoredValue::new((param("id"), param("artifact_id")));
    on_cleanup(review_mode::close);
    let workspace = LocalResource::new(move || {
        let (mission_id, artifact_id) = ids.get_value();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                use crate::v2::core::api::endpoints::mission_reviews::load_review_workspace;
                match load_review_workspace(store, &mission_id, &artifact_id).await {
                    Ok(workspace) => WorkspaceRead::Loaded(Box::new(workspace)),
                    Err((status, message)) => WorkspaceRead::Failed(workspace_failure_sentence(
                        status,
                        message.as_deref(),
                    )),
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, mission_id, artifact_id);
                WorkspaceRead::Failed(workspace_failure_sentence(0, None))
            }
        }
    });
    view! {
        <Suspense fallback=move || {
            view! {
                <div class="flex h-screen w-screen items-center justify-center bg-background text-on-surface-variant">
                    "Opening the review workspace…"
                </div>
            }
        }>
            {move || {
                workspace
                    .get()
                    .map(|read| match read {
                        WorkspaceRead::Loaded(workspace) => {
                            review_mode::open(ReviewedVersion::from_workspace(&workspace));
                            view! {
                                <MissionEditorPage />
                                {review_banner(&workspace)}
                            }
                                .into_any()
                        }
                        WorkspaceRead::Failed(why) => {
                            failure_screen(ids.get_value().0, why).into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// A workspace that could not be opened: why, and the way back to the mission.
fn failure_screen(mission_id: String, why: String) -> impl IntoView {
    let back = format!(
        "/missions/{}",
        crate::v2::core::api::endpoints::encode_path_segment(&mission_id)
    );
    view! {
        <div class="flex h-screen w-screen flex-col items-center justify-center gap-4 bg-background px-6 text-center">
            <MaterialIcon name="visibility_off" class="text-4xl text-outline" />
            <p class="max-w-md text-body-md text-on-surface">{why}</p>
            <a
                href=back
                class="rounded-full border border-white/10 px-4 py-2 text-label-md text-primary transition hover:bg-white/5"
            >
                "Back to the mission"
            </a>
        </div>
    }
}
