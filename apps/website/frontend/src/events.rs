//! Event Schedule (/events) — ported from pages/operations.tsx `EventSchedulePage`. `<AuthGate>` →
//! `/events` Resource → `QueryState` → a `SplitPane` (upcoming-ops master list + an event-hub detail).
//!
//! **Gate scope (this slice):** the empty-DB `/events` golden (Paginated empty) → master shows
//! "No upcoming operations scheduled." and, with nothing selected, the detail shows `SplitPaneEmpty`.
//! Byte-exact-verified. The populated op rows + `EventScheduleDetail` (the event hub) are
//! content-golden gated; the event item type stays `serde_json::Value` until then.
#![allow(dead_code)]
use crate::dto::Paginated;
use crate::split_pane::{SplitPane, SplitPaneEmpty};
use crate::ui::{AuthGate, MaterialIcon};
use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn EventSchedulePage() -> impl IntoView {
    view! {
        <AuthGate>
            <EventScheduleInner />
        </AuthGate>
    }
}

#[component]
fn EventScheduleInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let events = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Paginated<Value>>(store, "/events")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<Value>>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                events
                    .get()
                    .map(|opt| match opt {
                        Some(page) => board(page.data).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

fn board(events: Vec<Value>) -> impl IntoView {
    let master_header = view! {
        <h2 class="text-headline-sm tracking-wide text-on-surface uppercase">"Upcoming Ops"</h2>
    }
    .into_any();
    // Populated op rows are content-golden gated (empty golden → empty state).
    let master = if events.is_empty() {
        view! {
            <p class="px-1 py-4 text-label-md text-on-surface-variant">
                "No upcoming operations scheduled."
            </p>
        }
        .into_any()
    } else {
        ().into_any()
    };
    let detail = view! {
        <SplitPaneEmpty
            icon=view! { <MaterialIcon name="calendar_month" class="text-4xl" /> }.into_any()
            message="Select an operation to view its hub."
        />
    }
    .into_any();
    view! {
        <SplitPane
            master_width="24rem"
            master_header=master_header
            master=master
            detail=detail
        />
    }
}
