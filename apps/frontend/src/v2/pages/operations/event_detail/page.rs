//! The operation dossier route: one operation, its missions, and the slotting for each.
//!
//! **Role:** reads the operation id from the path, fetches the operation, and wraps the shared
//! hub body in this route's own chrome — the topographic backdrop, the scroll surface and the
//! link back to the schedule.
//! **Position:** the `/events/:id` route, rendered inside the navigation frame behind the
//! sign-in gate.
//! **Signals & state:** reads the session store and the route params from context. Owns the
//! operation resource and the callback every slotting mutation calls to refetch it.
//! **Invariants:** the fetch is a browser-only path, so a native build resolves it to `None` and
//! renders the failure branch. A refetch rebuilds this subtree, which resets the selector's
//! faction and squad tabs to their defaults.
#![allow(dead_code)]

use super::hero_countdown::event_hub_view;
use crate::v2::core::api::dto::EventHub;
use crate::v2::core::ui::{AuthGate, MaterialIcon};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

/// The `/events/:id` route: the operation dossier behind the sign-in gate.
#[component]
pub fn EventHubPage() -> impl IntoView {
    view! {
        <AuthGate>
            <EventHubInner />
        </AuthGate>
    }
}

/// The signed-in half of the page: the operation fetch and its two render states.
///
/// A mutation anywhere in the slotting below bubbles up to `on_change`, which refetches the
/// operation so the header and footer states derived from it stay live.
#[component]
fn EventHubInner() -> impl IntoView {
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
    let on_change = Callback::new(move |()| event.refetch());
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                event
                    .get()
                    .map(|opt| match opt {
                        Some(ev) => hub_shell(ev, on_change).into_any(),
                        None => view! { <p class="text-error">"Failed to load data."</p> }.into_any(),
                    })
            }}
        </Suspense>
    }
}

/// This route's chrome around the shared hub body: the backdrop, the scroll surface and the
/// link back to the schedule.
fn hub_shell(ev: EventHub, on_change: Callback<()>) -> impl IntoView {
    view! {
        <div class="relative h-full w-full overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="custom-scrollbar relative z-10 h-full w-full overflow-y-auto bg-surface-glass backdrop-blur-xl">
                <div class="mx-auto w-full max-w-5xl p-6 md:p-8">
                    <a
                        href="/events"
                        class="mb-4 inline-flex items-center gap-1 text-label-md text-primary hover:underline"
                    >
                        <MaterialIcon name="chevron_left" class="text-base" />
                        " All Operations"
                    </a>
                    {event_hub_view(ev, on_change)}
                </div>
            </div>
        </div>
    }
}
