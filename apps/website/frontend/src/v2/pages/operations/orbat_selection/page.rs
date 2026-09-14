//! The standalone slotting view: one mission's order of battle, on its own page.
//!
//! **Role:** reads the operation and mission ids from the path, fetches the operation, and
//! renders the back link, the mission heading and the shared slotting selector for that mission.
//! **Position:** the `/events/:id/missions/:emid/orbat` route, rendered inside the navigation
//! frame behind the sign-in gate.
//! **Signals & state:** reads the session store and the route params from context. Owns the
//! operation resource and the callback the selector runs after every mutation.
//! **Invariants:** the mission heading and the caller's registration state are looked up in the
//! operation by mission id, so a path naming a mission this operation does not carry still
//! renders — with the generic heading and no registration. The selector is mounted only when the
//! path actually carries a mission id. The fetch is a browser-only path and resolves to `None`
//! in a native build.
#![allow(dead_code)]

use super::super::event_detail::OrbatSelector;
use crate::v2::core::api::dto::EventHub;
use crate::v2::core::ui::{AuthGate, MaterialIcon};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

/// The `/events/:id/missions/:emid/orbat` route: the slotting view behind the sign-in gate.
#[component]
pub fn OrbatSelectionPage() -> impl IntoView {
    view! {
        <AuthGate>
            <OrbatSelectionInner />
        </AuthGate>
    }
}

/// The signed-in half of the page: the operation fetch, the mission lookup, and the selector.
#[component]
fn OrbatSelectionInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let params = use_params_map();
    let event = LocalResource::new(move || {
        let id = params
            .read()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_default();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/events/{id}");
                crate::v2::core::api::client::api_get::<EventHub>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                None::<EventHub>
            }
        }
    });
    // The selector bubbles a refetch after every mutation, so the header and footer states
    // derived from the caller's registration stay live.
    let on_change = Callback::new(move |()| event.refetch());
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                event
                    .get()
                    .map(|ev| {
                        let id = params
                            .read()
                            .get("id")
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        let emid = params
                            .read()
                            .get("emid")
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        let name = ev
                            .as_ref()
                            .and_then(|e| e.name_override.clone())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "Operation".into());
                        let dossier = ev
                            .as_ref()
                            .and_then(|e| e.missions.iter().find(|m| m.event_mission_id == emid));
                        let title = dossier
                            .map(|d| d.title.clone())
                            .unwrap_or_else(|| "Order of Battle".into());
                        let my_state = dossier.and_then(|d| d.my_state.clone());
                        let href = format!("/events/{id}");
                        view! {
                            <div class="mx-auto w-full max-w-5xl">
                                <a
                                    href=href
                                    class="mb-4 inline-flex items-center gap-1 text-sm text-primary hover:underline"
                                >
                                    <MaterialIcon name="chevron_left" class="text-base" />
                                    " "
                                    {name}
                                </a>
                                <header class="mb-8">
                                    <h1 class="mb-2 text-3xl font-bold text-on-surface">{title}</h1>
                                    <p class="max-w-3xl text-on-surface-variant">
                                        "Select your faction, squad, and slot, then register for deployment."
                                    </p>
                                </header>
                                {(!emid.is_empty())
                                    .then(|| {
                                        view! {
                                            <OrbatSelector
                                                emid=emid.clone()
                                                my_state=my_state
                                                on_change=on_change
                                            />
                                        }
                                    })}
                            </div>
                        }
                        .into_any()
                    })
            }}
        </Suspense>
    }
}
