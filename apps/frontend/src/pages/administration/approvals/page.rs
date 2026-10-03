//! The approvals route: the queue of missions waiting on a reviewer.
//!
//! **Role:** fetches the pending queue, puts it behind the administrator gate, and hands the rows
//! to the queue and drawer below together with the desk they share.
//! **Position:** the `/admin/approvals` route, rendered inside the navigation frame.
//! **Signals & state:** owns the desk — `selected_id` (which submission is open), the refetch every
//! decision calls, and the notice a stale decision leaves behind. The queue lives in a
//! `LocalResource` read inside a suspense boundary.
//! **Invariants:** the request future is not `Send`, so the fetch is a browser-only path, as is the
//! page itself. The endpoint lists pending submissions and nothing else, so every row reaching the
//! drawer is awaiting a decision — though a row that predates reviews has no artifact to decide
//! yet. The notice lives here rather than in the drawer because reading the queue again rebuilds
//! the drawer, and the reviewer must still be told why their decision did not land.

#[cfg(target_arch = "wasm32")]
use super::submission_queue::board;
#[cfg(target_arch = "wasm32")]
use crate::foundation::auth::AdminGate;
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::ApprovalRow;
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::Paginated;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// What the queue and the drawer share.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(super) struct ApprovalsDesk {
    /// Which submission is open; the first row when none is picked.
    pub(super) selected_id: RwSignal<Option<String>>,
    /// Reads the queue again.
    pub(super) refetch: Callback<()>,
    /// The mission a refused decision concerned, and what the reviewer is told about it.
    pub(super) notice: RwSignal<Option<(String, String)>>,
}

/// The approvals screen, behind the administrator gate.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn MissionApprovalsPage() -> impl IntoView {
    view! {
        <AdminGate>
            <MissionApprovalsInner />
        </AdminGate>
    }
}

/// The screen a reviewer sees: the fetch, and its three render states.
#[cfg(target_arch = "wasm32")]
#[component]
fn MissionApprovalsInner() -> impl IntoView {
    let store = expect_context::<crate::foundation::auth::AuthStore>();
    let approvals = LocalResource::new(move || async move {
        {
            crate::foundation::transport::client::api_get::<Paginated<ApprovalRow>>(
                store,
                "/approvals",
            )
            .await
            .ok()
        }
    });
    let desk = ApprovalsDesk {
        selected_id: RwSignal::new(None::<String>),
        refetch: Callback::new(move |()| approvals.refetch()),
        notice: RwSignal::new(None),
    };
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                approvals
                    .get()
                    .map(|opt| match opt {
                        Some(page) => board(page.data, page.total, desk).into_any(),
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}
