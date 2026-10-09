//! The modpacks route: fetch the manifests, pick one, and lay the two panes out.
//!
//! **Role:** owns the request for `GET /modpacks`, holds the selection and the read/edit mode,
//! and arranges the pack list and the detail pane in a split view.
//! **Position:** the `/modpacks` route, behind the authentication gate.
//! **Signals & state:** a `LocalResource` for the pack list; `selected_id`, `search`, `mode` and
//! `create_busy` signals shared with the panes; an `is_admin` memo over the `AuthStore` from
//! context; the toast queue from context.
//! **Invariants:** the admin memo re-reads the store, so the create, edit and delete affordances
//! appear only for a signed-in administrator and never during bootstrap. Selecting another pack
//! returns the detail pane to reading. The fetch runs on `wasm32` only, as does the page.

#[cfg(target_arch = "wasm32")]
use super::mod_table::dossier;
#[cfg(target_arch = "wasm32")]
use super::mode_toggle::MpMode;
#[cfg(target_arch = "wasm32")]
use super::pack_editor::editor;
#[cfg(target_arch = "wasm32")]
use super::preset_list::{master_header, pack_list};
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::DataEnvelope;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::ModpackDto;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::role::{Role, has_min_role_authed};
#[cfg(target_arch = "wasm32")]
use frontend_ui::split_pane::GlassSplit;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The modpacks page, behind the authentication gate.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn ModpacksPage() -> impl IntoView {
    view! {
        <frontend_session::AuthGate>
            <ModpacksInner />
        </frontend_session::AuthGate>
    }
}

/// Fetches the modpack list and renders the board, a loading line, or a failure line.
#[cfg(target_arch = "wasm32")]
#[component]
fn ModpacksInner() -> impl IntoView {
    let store = expect_context::<frontend_session::AuthStore>();
    let packs = LocalResource::new(move || async move {
        {
            frontend_transport::client::api_get::<DataEnvelope<ModpackDto>>(store, "/modpacks")
                .await
                .ok()
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="px-8 py-10 text-on-surface-variant">"Loading modpacks…"</p> }
        }>
            {move || {
                packs.get().map(|opt| match opt {
                    Some(env) => modpacks_board(env.data, packs).into_any(),
                    None => {
                        view! { <p class="px-8 py-10 text-error">"Failed to load modpacks."</p> }
                            .into_any()
                    }
                })
            }}
        </Suspense>
    }
}

/// The split view over a fetched modpack list.
///
/// `list` is that list and `packs_res` the resource it came from, kept so a write can refetch it.
#[cfg(target_arch = "wasm32")]
fn modpacks_board(
    list: Vec<ModpackDto>,
    packs_res: LocalResource<Option<DataEnvelope<ModpackDto>>>,
) -> impl IntoView {
    let store = expect_context::<frontend_session::AuthStore>();
    // Re-read the store on every change: the browse-mode role check treats a signed-out
    // visitor as permitted, so it must never drive the create and edit affordances.
    let is_admin =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin));
    let selected_id = RwSignal::new(
        list.first()
            .map(|p| p.modpack.id.to_string())
            .unwrap_or_default(),
    );
    let search = RwSignal::new(String::new());
    let mode = RwSignal::new(MpMode::Read);
    let toasts = frontend_ui::toast::use_toasts();
    let create_busy = RwSignal::new(false);

    Effect::new(move |prev: Option<String>| {
        let id = selected_id.get();
        if prev.as_ref().is_some_and(|p| p != &id) {
            mode.set(MpMode::Read);
        }
        id
    });

    let list_master = list.clone();
    let list_detail = list;

    view! {
        <GlassSplit
            master_width="18rem"
            master_header=master_header(search, is_admin, create_busy, packs_res, selected_id, toasts)
                .into_any()
            master=view! {
                {move || {
                    pack_list(
                        &list_master,
                        selected_id,
                        &search.get(),
                    )
                }}
            }
                .into_any()
            detail=view! {
                {move || {
                    let id = selected_id.get();
                    let Some(p) = list_detail.iter().find(|p| p.modpack.id == id.as_str()) else {
                        return view! {
                            <p class="px-8 py-10 text-on-surface-variant">
                                "No modpack selected."
                            </p>
                        }
                            .into_any();
                    };
                    if mode.get() == MpMode::Edit && is_admin.get() {
                        editor(p, mode, packs_res, toasts).into_any()
                    } else {
                        dossier(p, mode, is_admin.get(), packs_res, toasts).into_any()
                    }
                }}
            }
                .into_any()
        />
    }
}
