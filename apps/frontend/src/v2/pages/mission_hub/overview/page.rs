//! The mission overview route: fetch one mission, then lay its dossier out.
//!
//! **Role:** owns the request for the mission detail, decides whether the viewer may edit its
//! armory and read its review record, and arranges the header, the shared dossier body, the review
//! record and the Edit Armory dialog.
//! **Position:** the `/missions/:id` route, behind the sign-in gate.
//! **Signals & state:** a `LocalResource` keyed on the route parameter and on the armory editor's
//! saved counter, so a successful write re-reads the mission and the read-only armory shows what
//! was just stored. Reads the session store from context through a memo of the viewer's account
//! id and administrator standing, the only two facts the dossier renders from.
//! **Invariants:** the armory handle is created before the resource, because the resource depends
//! on it, and it is owned by the component rather than by the render closure, so the draft
//! survives a refetch. The fetch is browser-only; natively it resolves to `None` and the page
//! renders its failure line.

use super::armory_dialog::armory_dialog;
use super::armory_editor::ArmoryEditor;
use super::dossier_body::dossier_body;
use super::header::dossier_header;
use crate::v2::core::api::dto::MissionDetail;
use crate::v2::core::auth::Role;
use crate::v2::core::ui::AuthGate;
use crate::v2::pages::mission_hub::mission_review::review_record::MissionReviewRecord;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

/// The mission overview, behind the sign-in gate.
#[component]
pub fn MissionOverviewPage() -> impl IntoView {
    view! {
        <AuthGate>
            <MissionOverviewInner />
        </AuthGate>
    }
}

/// What the edit predicate knows about the viewer.
#[derive(Clone, PartialEq)]
struct EditingViewer {
    /// The viewer's account id; empty when nobody is signed in.
    account_id: String,
    /// Whether the viewer's account is an administrator.
    is_admin: bool,
}

/// The dossier itself, once the session gate has let the viewer through.
#[component]
fn MissionOverviewInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let params = use_params_map();
    // Created before the resource because the resource depends on it: the saved counter is one of
    // its reactive inputs, so a successful write re-runs the fetch and the read-only armory shows
    // what was just stored. Owned by the component rather than by the render closure, so the draft
    // is not disposed when the resource re-runs.
    let editor = ArmoryEditor::new();
    let mission = LocalResource::new(move || {
        let id = params
            .read()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_default();
        let _ = editor.saved.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/missions/{id}");
                crate::v2::core::api::client::api_get::<MissionDetail>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                None::<MissionDetail>
            }
        }
    });
    // Memoized, so the dossier below is rebuilt when the viewer's account or administrator
    // standing changes and not on every profile poll; a rebuild discards the dossier's local
    // state, such as a review comment being typed.
    let viewer = Memo::new(move |_| {
        store.user.with(|user| EditingViewer {
            account_id: user
                .as_ref()
                .map(|u| u.discord_id.clone())
                .unwrap_or_default(),
            is_admin: user.as_ref().is_some_and(|u| u.role == Role::Admin),
        })
    });

    // Mirrors the API's own predicate exactly — author or administrator, and deliberately not a
    // role tier. The server's tier is authorship, so a role check here would hide the button from
    // someone the endpoint would serve and show it to someone it would refuse. The review record
    // is served to exactly the same two.
    let can_edit = move |m: &MissionDetail| {
        viewer.with(|viewer| m.author_id == viewer.account_id || viewer.is_admin)
    };
    let refetch = Callback::new(move |()| mission.refetch());

    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                mission
                    .get()
                    .map(|opt| match opt {
                        Some(m) => {
                            let editable = can_edit(&m);
                            body(m, editor, editable, refetch).into_any()
                        }
                        None => view! { <p class="text-error">"Failed to load data."</p> }.into_any(),
                    })
            }}
        </Suspense>
        {armory_dialog(editor)}
    }
}

/// The dossier layout: the header above the glass card that holds the shared body, and — for the
/// author and administrators — the review record under it.
fn body(
    m: MissionDetail,
    ed: ArmoryEditor,
    editable: bool,
    refetch: Callback<()>,
) -> impl IntoView {
    let review_record = editable.then(|| {
        view! {
            <div class="mt-6 rounded-xl p-6 glass">
                <MissionReviewRecord
                    mission_id=m.id.clone()
                    status=m.status.clone()
                    reviewed_at=m.reviewed_at.clone()
                    approved_artifact_id=m.approved_artifact_id.clone()
                    on_resubmitted=refetch
                />
            </div>
        }
    });
    view! {
        <div class="mx-auto w-full max-w-3xl">
            {dossier_header(&m, ed, editable)}
            <div class="relative flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass">
                {dossier_body(&m)}
            </div>
            {review_record}
        </div>
    }
}
