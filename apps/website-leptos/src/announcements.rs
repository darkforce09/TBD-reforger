//! Announcements (/announcements) — ported from pages/operations.tsx `AnnouncementsPage`.
//! `<AuthGate>` → `/announcements` Resource → `QueryState` → a topo-map/frosted-glass encasing
//! around a transparent `SplitPane` (Comms Link master list + reading detail pane).
//!
//! **Gate scope (this slice):** the empty-DB `/announcements` golden (Paginated empty) → the master
//! shows "No announcements yet." and, with nothing selected, the detail shows `SplitPaneEmpty`.
//! Byte-exact-verified. The populated list (ListDetailItem rows) + `AnnouncementDetail` reader are
//! content-golden gated; the announcement item type stays `serde_json::Value` until then.
#![allow(dead_code)]
use crate::dto::Paginated;
use crate::split_pane::{SplitPane, SplitPaneEmpty};
use crate::ui::{AuthGate, MaterialIcon};
use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn AnnouncementsPage() -> impl IntoView {
    view! {
        <AuthGate>
            <AnnouncementsInner />
        </AuthGate>
    }
}

#[component]
fn AnnouncementsInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let posts = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Paginated<Value>>(store, "/announcements")
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
                posts
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

fn board(posts: Vec<Value>) -> impl IntoView {
    // Pinned-first sort + selection are content-golden gated (empty list → no rows, nothing selected).
    let master = if posts.is_empty() {
        view! {
            <p class="px-1 py-4 text-label-md text-on-surface-variant">"No announcements yet."</p>
        }
        .into_any()
    } else {
        // ListDetailItem rows — content-golden gated.
        ().into_any()
    };
    let master_header = view! {
        <>
            <h2 class="text-headline-sm tracking-wide text-on-surface uppercase">"Comms Link"</h2>
            <MaterialIcon name="filter_list" class="text-outline" />
        </>
    }
    .into_any();
    let detail = view! {
        <SplitPaneEmpty
            icon=view! { <MaterialIcon name="campaign" class="text-4xl" /> }.into_any()
            message="Select a broadcast to read."
        />
    }
    .into_any();

    view! {
        <div class="relative h-full w-full overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="relative z-10 flex h-full w-full bg-surface-glass backdrop-blur-xl">
                <SplitPane transparent=true master_header=master_header master=master detail=detail />
            </div>
        </div>
    }
}
