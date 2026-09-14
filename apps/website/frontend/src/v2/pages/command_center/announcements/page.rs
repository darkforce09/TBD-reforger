//! The announcements route: the unit's dispatches, in a master-detail board.
//!
//! **Role:** fetches the announcement page and hands its rows to the board.
//! **Position:** the `/announcements` and `/announcements/:id` routes, rendered inside the
//! navigation frame behind the sign-in gate.
//! **Signals & state:** reads the session store from context. The payload lives in a
//! `LocalResource` owned by this file and is read inside a suspense boundary.
//! **Invariants:** the request future is not `Send`, so the fetch is a browser-only path and a
//! native build resolves to `None` and renders the failure branch. The list payload already
//! carries every body, so opening a dispatch never fetches again and the reading pane can
//! never show a stale one.
#![allow(dead_code)]

use super::article_feed::board;
use crate::v2::core::api::dto::Paginated;
use crate::v2::core::ui::AuthGate;
use leptos::prelude::*;
use serde_json::Value;

/// The announcements route: the board behind the sign-in gate.
#[component]
pub fn AnnouncementsPage() -> impl IntoView {
    view! {
        <AuthGate>
            <AnnouncementsInner />
        </AuthGate>
    }
}

/// The signed-in half of the board: the fetch and its three render states.
#[component]
fn AnnouncementsInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let posts = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<Paginated<Value>>(store, "/announcements")
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
