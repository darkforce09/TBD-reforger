//! The dashboard route: one screen summarising what the unit is doing right now.
//!
//! **Role:** fetches the dashboard payload and arranges the five panels it feeds.
//! **Position:** the `/` route, rendered inside the navigation frame behind the sign-in gate.
//! **Signals & state:** reads the session store from context. The payload lives in a
//! `LocalResource` owned by this file and is read inside a suspense boundary.
//! **Invariants:** the request future is not `Send`, so the fetch is a browser-only path, as is the
//! page itself. Each panel receives its slice of the payload owned, so no panel re-reads the
//! resource.
//!
//! The card grid is written out as literal class strings per card rather than composed from a
//! shared base: the merge rules that produced them are not reproduced here, so the merged
//! result is what is written.

#[cfg(target_arch = "wasm32")]
use super::deployment::deployment;
#[cfg(target_arch = "wasm32")]
use super::hero_banner::hero_banner;
#[cfg(target_arch = "wasm32")]
use super::modpack::modpack_card;
#[cfg(target_arch = "wasm32")]
use super::recent_intel::recent_intel;
#[cfg(target_arch = "wasm32")]
use super::server_uplink::server_uplink;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::DashboardResponse;
#[cfg(target_arch = "wasm32")]
use frontend_session::AuthGate;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The `/` route: the dashboard behind the sign-in gate.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn DashboardPage() -> impl IntoView {
    view! {
        <AuthGate>
            <DashboardInner />
        </AuthGate>
    }
}

/// The signed-in half of the dashboard: the fetch and its three render states.
///
/// Renders a loading line while the request is in flight, an error line when it failed, and
/// the panel grid once the payload arrived.
#[cfg(target_arch = "wasm32")]
#[component]
fn DashboardInner() -> impl IntoView {
    let store = expect_context::<frontend_session::AuthStore>();
    let dash = LocalResource::new(move || async move {
        {
            frontend_transport::client::api_get::<DashboardResponse>(store, "/dashboard")
                .await
                .ok()
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                dash.get()
                    .map(|opt| match opt {
                        Some(d) => bento(d).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// The panel grid for one loaded payload: banner, three cards, then the intelligence feed.
#[cfg(target_arch = "wasm32")]
fn bento(d: DashboardResponse) -> impl IntoView {
    view! {
        <div class="custom-scrollbar flex h-full w-full flex-col gap-8 overflow-y-auto p-6 md:p-8">
            {hero_banner(d.next_event)}
            <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
                {server_uplink(d.fleet)}
                {deployment(d.my_assignment)}
                {modpack_card(d.current_modpack)}
            </div>
            {recent_intel(d.recent_announcements)}
        </div>
    }
}
