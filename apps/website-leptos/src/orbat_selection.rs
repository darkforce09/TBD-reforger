//! ORBAT Selection (/events/:id/missions/:emid/orbat) — ported from pages/events.tsx
//! `OrbatSelectionPage`. `<AuthGate>` → `useEvent(id)` → a back-link + PageHeader + the shared
//! `OrbatSelector` for the one `emid` (dossier looked up in the hub by event_mission_id).
//!
//! **Gate scope:** the seeded golden (empty ORBAT) → the selector short-circuits to "No ORBAT slots
//! defined…". The full faction/squad/slot shell is content-gated (same `OrbatSelector` as the hub).
#![allow(dead_code)]
use crate::dto::EventHub;
use crate::event_hub::OrbatSelector;
use crate::ui::{AuthGate, MaterialIcon};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn OrbatSelectionPage() -> impl IntoView {
    view! {
        <AuthGate>
            <OrbatSelectionInner />
        </AuthGate>
    }
}

#[component]
fn OrbatSelectionInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
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
                crate::client::api_get::<EventHub>(store, &path).await.ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                None::<EventHub>
            }
        }
    });
    // Register/withdraw invalidate the event hub in React; here the selector bubbles a refetch so
    // the my_state-derived header/footer stay live (T-159.25).
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
                        // event?.name_override ?? 'Operation'
                        let name = ev
                            .as_ref()
                            .and_then(|e| e.name_override.clone())
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| "Operation".into());
                        // dossier = event?.missions.find(m => m.event_mission_id === emid)
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
