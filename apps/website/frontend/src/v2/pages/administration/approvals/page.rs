//! The approvals route: the queue of missions waiting on a reviewer.
//!
//! **Role:** fetches the pending queue, puts it behind the administrator gate, and hands the rows
//! to the queue and drawer below.
//! **Position:** the `/admin/approvals` route, rendered inside the navigation frame.
//! **Signals & state:** owns `selected_id` (which submission is open) and the refetch the two
//! decisions call. The queue lives in a `LocalResource` read inside a suspense boundary.
//! **Invariants:** the request future is not `Send`, so the fetch is a browser-only path and a
//! native build resolves to nothing and renders the failure branch. The endpoint lists pending
//! submissions and nothing else, so every row reaching the drawer is one a decision can be made on.
#![allow(dead_code)]

use super::submission_queue::board;
use crate::v2::core::api::dto::{ApprovalRow, Paginated};
use crate::v2::core::ui::AdminGate;
use leptos::prelude::*;

/// The approvals screen, behind the administrator gate.
#[component]
pub fn MissionApprovalsPage() -> impl IntoView {
    view! {
        <AdminGate>
            <MissionApprovalsInner />
        </AdminGate>
    }
}

/// The screen a reviewer sees: the fetch, and its three render states.
#[component]
fn MissionApprovalsInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let approvals = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<Paginated<ApprovalRow>>(store, "/approvals")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Paginated<ApprovalRow>>
        }
    });
    let selected_id = RwSignal::new(None::<String>);
    let refetch = Callback::new(move |()| approvals.refetch());
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                approvals
                    .get()
                    .map(|opt| match opt {
                        Some(page) => {
                            board(page.data, page.total, selected_id, refetch).into_any()
                        }
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
    </Suspense>
    }
}
