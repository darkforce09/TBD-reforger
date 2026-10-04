//! The slide-over mission dossier: fetch one mission, then hand it to the dossier body.
//!
//! **Role:** the sheet's own component — it requests the mission detail, keeps the load bar up
//! through the slide-in, works out what the viewer may do with the mission, and renders either
//! the dossier or a failure line.
//! **Position:** the content of the library's slide-over sheet; opened by any card or by the
//! featured hero, and closed without leaving the library route.
//! **Signals & state:** a `LocalResource` for the mission detail, the three overlay open flags it
//! owns and passes down, and the animation latch that gates the first render. Reads the session
//! store from context.
//! **Invariants:** the fetch starts immediately but the heavy dossier subtree is withheld until the
//! sheet's slide finishes, so a fast local response cannot mount it mid-animation. The fetch itself
//! is browser-only, as is the sheet.

#[cfg(target_arch = "wasm32")]
use super::dossier_body::dossier_sheet_body;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::MissionDetail;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::identifiers::MissionId;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::role::{Role, has_min_role_authed};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The dossier for one mission, behind the library's slide-over sheet.
///
/// `changed` is run after any write so the card grid and the dossier both re-read the mission
/// rather than showing what it looked like before. The component is public because the props
/// builder `#[component]` derives has `pub` methods whatever the component's visibility.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn MissionDossierSheet(
    /// The mission the sheet fetches and shows.
    id: MissionId,
    /// The sheet's open flag, which the delete action clears.
    sheet_open: RwSignal<bool>,
    /// Run after every write, so the library re-reads the missions.
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<frontend_session::AuthStore>();
    let id_sv = StoredValue::new(id.to_string());
    let mission = LocalResource::new(move || {
        let id = id_sv.get_value();
        async move {
            {
                let path = format!("/missions/{id}");
                frontend_transport::client::api_get::<MissionDetail>(store, &path)
                    .await
                    .ok()
            }
        }
    });
    let comments_open = RwSignal::new(false);
    let invite_open = RwSignal::new(false);
    let confirm_delete_open = RwSignal::new(false);
    // The same reactive, authenticated gate the library header uses: a pre-bootstrap session must
    // not read as a mission maker.
    let is_maker = Memo::new(move |_| {
        has_min_role_authed(store.user.get().map(|u| u.role), Role::MissionMaker)
    });
    let is_admin =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin));
    let me = StoredValue::new(
        store
            .user
            .get_untracked()
            .map(|u| u.discord_id.into_inner()),
    );
    // Hold the heavy dossier subtree until the sheet's slide finishes, so a fast local response
    // cannot land a large mount mid-animation. The fetch above is not gated, only the render.
    let anim_done = RwSignal::new(false);
    {
        use leptos::prelude::set_timeout;
        set_timeout(
            move || anim_done.set(true),
            std::time::Duration::from_millis(320),
        );
    }

    view! {
        <Suspense fallback=dossier_loading>
            {move || {
                let fetched = mission.get();
                if !anim_done.get() {
                    // Keep the load bar up through the slide even when the fetch beat it.
                    return Some(dossier_loading().into_any());
                }
                fetched
                    .map(|opt| match opt {
                        Some(m) => {
                            let is_owner = me.get_value().as_deref()
                                == Some(m.author_id.as_str());
                            // `can_edit` gates what needs the editor: the Mission Creator link and
                            // the collaboration row. The edit route really does require the
                            // mission-maker tier, so promising it to anyone else would only bounce
                            // them off a role gate.
                            let can_edit = is_maker.get() && (is_owner || is_admin.get());
                            // `can_manage` gates the lifecycle — submit, archive, delete — and the
                            // review feedback, and deliberately drops the maker tier to match the
                            // API exactly: those three routes each test author-or-administrator
                            // and nothing else. Only creating a mission needs the tier, which is
                            // why the New Mission button keeps it.
                            //
                            // The gap is reachable: a role sync can demote someone who still owns
                            // missions, and they keep full rights over them. Putting the rejection
                            // reason behind the stricter gate would hide it from exactly the
                            // person who has to read it.
                            let can_manage = is_owner || is_admin.get();
                            dossier_sheet_body(
                                    m,
                                    id_sv,
                                    can_edit,
                                    can_manage,
                                    sheet_open,
                                    comments_open,
                                    invite_open,
                                    confirm_delete_open,
                                    changed,
                                )
                                .into_any()
                        }
                        None => {
                            view! { <p class="p-8 text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// The indeterminate load gate: a label and a sweeping bar while the dossier fetches, also shown
/// through the sheet's slide-in.
#[cfg(target_arch = "wasm32")]
fn dossier_loading() -> impl IntoView {
    view! {
        <div class="flex h-full flex-col items-center justify-center gap-4 p-8">
            <p class="font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                "Loading dossier…"
            </p>
            <div class="h-1 w-56 overflow-hidden rounded-full bg-surface-variant/40">
                <div class="animate-mc-load-bar h-full w-1/4 rounded-full bg-primary"></div>
            </div>
        </div>
    }
}
