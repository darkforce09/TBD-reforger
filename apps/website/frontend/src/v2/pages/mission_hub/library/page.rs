//! The mission library route: what is fetched, what is held, and how the page is laid out.
//!
//! **Role:** owns the scope, search and filter state, the two requests that fill the page, and
//! the overlays it can open — the dossier sheet and the create dialog.
//! **Position:** the `/missions` route, behind the sign-in gate.
//! **Signals & state:** `scope_idx`, `q`, `terrain`, `mode` and `players` key the mission
//! resource; a second resource keeps the hero on the newest global mission so it does not change
//! as tabs are switched. `preview_id` and `sheet_open` drive the dossier sheet, `create_open` the
//! create dialog. Reads the session store from context.
//! **Invariants:** only one overlay is open at a time — opening the create dialog closes the
//! dossier first. Both fetches are browser-only; natively they resolve to `None` and the page
//! renders its failure line. The create affordances are gated on a memo that re-reads the
//! session, so a page that has not finished bootstrapping never reads as a mission maker.

use super::card_grid::body;
use super::dossier_sheet::MissionDossierSheet;
use super::header::{library_header, SCOPES};
use crate::v2::core::api::dto::{MissionCard, Paginated};
use crate::v2::core::auth::{has_min_role_authed, Role};
use crate::v2::core::ui::{AuthGate, Sheet};
use crate::v2::pages::mission_hub::create_dialog::CreateMissionDialog;
use leptos::prelude::*;

/// Build the `/missions` request: the scope, plus whichever of the four filters are set.
///
/// An empty filter is omitted rather than sent as an empty parameter, so "no filter" and "filter
/// on nothing" cannot be confused on the wire.
pub(super) fn missions_query(
    scope: &str,
    q: &str,
    terrain: &str,
    mode: &str,
    players: &str,
) -> String {
    let mut url = format!("/missions?scope={scope}");
    #[cfg(target_arch = "wasm32")]
    let enc = |s: &str| {
        js_sys::encode_uri_component(s)
            .as_string()
            .unwrap_or_default()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let enc = |s: &str| s.to_string();
    if !terrain.is_empty() {
        url.push_str(&format!("&terrain={}", enc(terrain)));
    }
    if !mode.is_empty() {
        url.push_str(&format!("&mode={}", enc(mode)));
    }
    if !players.is_empty() {
        url.push_str(&format!("&player_count={}", enc(players)));
    }
    if !q.is_empty() {
        url.push_str(&format!("&q={}", enc(q)));
    }
    url
}

/// The mission library, behind the sign-in gate.
#[component]
pub fn MissionLibraryPage() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    // Reactive and authenticated: a browse-mode role read would treat a page that has not
    // bootstrapped as a mission maker. A pre-bootstrap session, or a guest, is false; once the
    // session lands the memo re-reads the role.
    let is_maker = Memo::new(move |_| {
        has_min_role_authed(store.user.get().map(|u| u.role), Role::MissionMaker)
    });
    let scope_idx = RwSignal::new(0usize);
    let q = RwSignal::new(String::new());
    let terrain = RwSignal::new(String::new());
    let mode = RwSignal::new(String::new());
    let players = RwSignal::new(String::new());
    let preview_id = RwSignal::new(None::<String>);
    let create_open = RwSignal::new(false);
    let sheet_open = RwSignal::new(false);
    // The viewer's own identifier, so a card can tell "my mission came back" from someone else's.
    // The list route already refuses to show a non-live mission to anyone but its author, so this
    // is belt and braces rather than the only guard — but the bookmarked scope has no status
    // predicate at all, and leaning on a server-side filter to keep a reviewer's private note off
    // someone else's screen is the kind of implicit coupling that breaks quietly.
    let me_id = StoredValue::new(store.user.get_untracked().map(|u| u.discord_id));

    let missions = LocalResource::new(move || {
        let url = missions_query(
            SCOPES[scope_idx.get().min(2)].1,
            &q.get(),
            &terrain.get(),
            &mode.get(),
            &players.get(),
        );
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                crate::v2::core::api::client::api_get::<Paginated<MissionCard>>(store, &url)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, url);
                None::<Paginated<MissionCard>>
            }
        }
    });
    // The hero always spotlights the newest global operation, so it stays put across tabs.
    let global = LocalResource::new(move || {
        let url = missions_query(
            "global",
            &q.get(),
            &terrain.get(),
            &mode.get(),
            &players.get(),
        );
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                crate::v2::core::api::client::api_get::<Paginated<MissionCard>>(store, &url)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, url);
                None::<Paginated<MissionCard>>
            }
        }
    });

    // Create is a transient action, so the dossier sheet closes first — one overlay at a time.
    let open_create = move || {
        preview_id.set(None);
        sheet_open.set(false);
        create_open.set(true);
    };
    let open_preview = move |id: String| {
        preview_id.set(Some(id));
        sheet_open.set(true);
    };

    // The create shortcut, for mission makers only, unless a field has focus.
    #[cfg(target_arch = "wasm32")]
    {
        let handle = window_event_listener(leptos::ev::keydown, move |ev| {
            if !is_maker.get_untracked() || create_open.get_untracked() {
                return;
            }
            if ev.key().to_lowercase() != "n" || !(ev.meta_key() || ev.ctrl_key()) {
                return;
            }
            if let Some(el) = document().active_element() {
                let tag = el.tag_name();
                if tag == "INPUT" || tag == "TEXTAREA" || tag == "SELECT" {
                    return;
                }
            }
            ev.prevent_default();
            open_create();
        });
        on_cleanup(move || handle.remove());
    }

    let refetch_all = Callback::new(move |()| {
        missions.refetch();
        global.refetch();
    });

    view! {
        <AuthGate>
            <div class="relative h-full w-full overflow-hidden">
                // The glass tint is baked into the static background layer rather than applied as
                // a backdrop blur on the scrollport, which would re-blur the whole page on every
                // scroll frame.
                <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
                <div class="absolute inset-0 z-0 bg-surface-glass"></div>
                <div class="custom-scrollbar relative z-10 h-full w-full overflow-y-auto">
                    <div class="p-6 md:p-8">
                        {library_header(is_maker, scope_idx, open_create)}
                        <Suspense fallback=move || {
                            view! { <p class="text-on-surface-variant">"Loading…"</p> }
                        }>
                            {move || {
                                missions
                                    .get()
                                    .map(|opt| match opt {
                                        Some(page) => {
                                            let featured = global
                                                .get()
                                                .flatten()
                                                .and_then(|g| g.data.first().cloned());
                                            let no_filters = q.get().is_empty()
                                                && terrain.get().is_empty() && mode.get().is_empty()
                                                && players.get().is_empty();
                                            let show_empty_cta = is_maker.get()
                                                && SCOPES[scope_idx.get().min(2)].1 == "mine"
                                                && page.data.is_empty() && no_filters;
                                            body(
                                                    page.data,
                                                    featured,
                                                    show_empty_cta,
                                                    q,
                                                    terrain,
                                                    mode,
                                                    players,
                                                    me_id,
                                                    open_preview,
                                                    open_create,
                                                    refetch_all,
                                                )
                                                .into_any()
                                        }
                                        None => {
                                            view! {
                                                <p class="text-error">"Failed to load data."</p>
                                            }
                                                .into_any()
                                        }
                                    })
                            }}
                        </Suspense>
                    </div>
                </div>
            </div>

            // The slide-over dossier, with no full-page navigation.
            <Sheet open=sheet_open bleed=true class="w-full max-w-none md:w-[60vw]">
                {move || {
                    preview_id
                        .get()
                        .map(|id| {
                            view! {
                                <MissionDossierSheet
                                    id=id
                                    sheet_open=sheet_open
                                    changed=refetch_all
                                />
                            }
                        })
                }}
            </Sheet>

            // The create dialog is transient — it exists only while it is open.
            <CreateMissionDialog open=create_open />
        </AuthGate>
    }
}
