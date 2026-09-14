//! The operations schedule: every upcoming operation beside the hub of the one in focus.
//!
//! **Role:** fetches the operation list, renders it as the master column of a split pane, and
//! drives the detail column from a second fetch keyed on the selected operation.
//! **Position:** the `/events` route, rendered inside the navigation frame behind the sign-in
//! gate.
//! **Signals & state:** reads the session store from context. Owns the list resource, the
//! `picked` signal a card click writes, the `selected_id` memo that resolves it against the
//! first row, and the hub resource keyed on that memo.
//! **Invariants:** the hub resource keeps serving its previous value while the next run is in
//! flight, so [`Hub::Loaded`] carries the operation id it was fetched for and the detail column
//! renders the loading state whenever that id is not the selected one. Both fetches are
//! browser-only paths; a native build resolves the list to `None` and the hub to [`Hub::Idle`].
#![allow(dead_code)]

use super::upcoming_ops::{op_card, vstr};
use crate::v2::core::api::dto::{EventHub, Paginated};
use crate::v2::core::ui::split_pane::{SplitPane, SplitPaneEmpty};
use crate::v2::core::ui::{AuthGate, MaterialIcon};
use crate::v2::pages::operations::event_detail::event_hub_view;
use leptos::prelude::*;
use serde_json::Value;

#[cfg(test)]
#[path = "tests/schedule.rs"]
mod tests;

/// The detail column's fetch state.
///
/// Three variants, not an `Option`: "nothing is selected", "the hub request failed" and "this is
/// the hub of the operation you were looking at before" are different situations, and folding
/// them together renders one operation's slotting under another operation's chrome.
#[derive(Clone, PartialEq)]
enum Hub {
    /// Nothing selected (the empty schedule) — there is no hub to fetch, and any run still in
    /// flight must read as loading rather than as a failure.
    Idle,
    /// The hub request for the selected operation came back an error.
    Failed,
    /// A hub, and the operation id it was fetched for.
    Loaded(String, EventHub),
}

/// The `/events` route: the schedule behind the sign-in gate.
#[component]
pub fn EventSchedulePage() -> impl IntoView {
    view! {
        <AuthGate>
            <EventScheduleInner />
        </AuthGate>
    }
}

/// The signed-in half of the page: the list fetch and its three render states.
///
/// Renders a loading line while the list is in flight, an error line when it failed, and the
/// split pane once the list arrived — including when that list is empty.
#[component]
fn EventScheduleInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let events = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<Paginated<Value>>(store, "/events")
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

/// The split pane over a fetched list of operations.
///
/// Builds the selection state, the hub resource keyed on it, and the master and detail columns.
fn board(events: Vec<Value>) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let events = StoredValue::new(events);
    // `None` means "nobody has picked yet", which resolves to the first row rather than to no
    // selection. Derived rather than seeded through an effect so there is no write during render
    // and no ordering question: a card click writes `picked`, everything else reads `selected_id`.
    let picked = RwSignal::new(None::<String>);
    // The list is fixed for the life of this call, so the fallback is computed once.
    let first_id = StoredValue::new(
        events.with_value(|e| e.first().map(|f| vstr(f, "id")).filter(|id| !id.is_empty())),
    );
    // A memo, not a closure: the master rows, the hub resource and the detail column all read
    // this. A memo is `Copy + Send` — which the reactive `class=` attribute below requires and a
    // captured generic closure is not — and it recomputes once per change rather than per reader.
    let selected_id = Memo::new(move |_| picked.get().or_else(|| first_id.get_value()));

    let hub = LocalResource::new(move || {
        let id = selected_id.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                match id {
                    Some(id) => {
                        match crate::v2::core::api::client::api_get::<EventHub>(
                            store,
                            &format!("/events/{id}"),
                        )
                        .await
                        {
                            Ok(h) => Hub::Loaded(id, h),
                            Err(_) => Hub::Failed,
                        }
                    }
                    None => Hub::Idle,
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                Hub::Idle
            }
        }
    });

    let master_header = view! {
        <h2 class="text-headline-sm tracking-wide text-on-surface uppercase">"Upcoming Ops"</h2>
    }
    .into_any();

    let master = view! {
        {move || {
            events
                .with_value(|events| {
                    if events.is_empty() {
                        return view! {
                            <p class="px-1 py-4 text-label-md text-on-surface-variant">
                                "No upcoming operations scheduled."
                            </p>
                        }
                            .into_any();
                    }
                    events.iter().map(|e| op_card(e, picked, selected_id)).collect_view().into_any()
                })
        }}
    }
    .into_any();

    let detail = view! {
        {move || {
            let want = selected_id.get();
            match (want, hub.get()) {
                // Nothing to select: the empty-schedule resting state.
                (None, _) => {
                    view! {
                        <SplitPaneEmpty
                            icon=view! { <MaterialIcon name="calendar_month" class="text-4xl" /> }
                                .into_any()
                            message="Select an operation to view its hub."
                        />
                    }
                        .into_any()
                }
                // The value in hand is the one asked for: the full hub, inline slotting and all.
                (Some(want), Some(Hub::Loaded(got, ev))) if got == want => {
                    let on_change = Callback::new(move |()| hub.refetch());
                    event_hub_view(ev, on_change).into_any()
                }
                (Some(_), Some(Hub::Failed)) => {
                    view! {
                        <div class="flex h-full flex-col items-center justify-center gap-3 px-8 text-center">
                            <MaterialIcon name="error" class="text-4xl text-error-alert" />
                            <p class="text-label-md text-on-surface-variant">
                                "Could not load this operation's hub."
                            </p>
                        </div>
                    }
                        .into_any()
                }
                // Everything else is in flight: the resource is pending, it is idle because the
                // run that will fetch `want` has not started, or it still holds the previously
                // selected operation's hub. All three are "loading", never "empty" and never the
                // other operation's data.
                (Some(_), _) => {
                    view! {
                        <div class="flex h-full items-center justify-center">
                            <p class="text-label-md text-on-surface-variant">"Loading operation…"</p>
                        </div>
                    }
                        .into_any()
                }
            }
        }}
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
