//! The mission overview route: fetch one mission, then lay its dossier out.
//!
//! **Role:** owns the request for the mission detail, decides whether the viewer may edit its
//! armory, and arranges the header, the shared dossier body and the Edit Armory dialog.
//! **Position:** the `/missions/:id` route, behind the sign-in gate.
//! **Signals & state:** a `LocalResource` keyed on the route parameter and on the armory editor's
//! saved counter, so a successful write re-reads the mission and the read-only armory shows what
//! was just stored. Reads the session store from context.
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
    let me = move || store.user.get().map(|u| u.discord_id).unwrap_or_default();

    // Mirrors the API's own predicate exactly — author or administrator, and deliberately not a
    // role tier. The server's tier is authorship, so a role check here would hide the button from
    // someone the endpoint would serve and show it to someone it would refuse.
    let can_edit = move |m: &MissionDetail| {
        m.author_id == me() || store.user.get().map(|u| u.role) == Some(Role::Admin)
    };

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
                            body(m, editor, editable).into_any()
                        }
                        None => view! { <p class="text-error">"Failed to load data."</p> }.into_any(),
                    })
            }}
        </Suspense>
        {armory_dialog(editor)}
    }
}

/// The dossier layout: the header above the glass card that holds the shared body.
fn body(m: MissionDetail, ed: ArmoryEditor, editable: bool) -> impl IntoView {
    view! {
        <div class="mx-auto w-full max-w-3xl">
            {dossier_header(&m, ed, editable)}
            <div class="relative flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass">
                {dossier_body(&m)}
            </div>
        </div>
    }
}
