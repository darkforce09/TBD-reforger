//! The announcements route: the unit's dispatches, in a master-detail board.
//!
//! **Role:** fetches the announcement page and hands its rows to the board.
//! **Position:** the `/announcements` and `/announcements/:id` routes, rendered inside the
//! navigation frame behind the sign-in gate.
//! **Signals & state:** reads the session store from context. The payload lives in a
//! `LocalResource` owned by this file and is read inside a suspense boundary.
//! **Invariants:** the request future is not `Send`, so the fetch is a browser-only path, as is the
//! page itself. The list payload already carries every body, so opening a dispatch never fetches
//! again and the reading pane can never show a stale one.

#[cfg(target_arch = "wasm32")]
use super::article_feed::board;
#[cfg(target_arch = "wasm32")]
use crate::foundation::auth::AuthGate;
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::{Announcement, Paginated};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The announcements route: the board behind the sign-in gate.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn AnnouncementsPage() -> impl IntoView {
    view! {
        <AuthGate>
            <AnnouncementsInner />
        </AuthGate>
    }
}

/// The signed-in half of the board: the fetch and its three render states.
#[cfg(target_arch = "wasm32")]
#[component]
fn AnnouncementsInner() -> impl IntoView {
    let store = expect_context::<crate::foundation::auth::AuthStore>();
    let posts = LocalResource::new(move || async move {
        {
            crate::foundation::transport::client::api_get::<Paginated<Announcement>>(
                store,
                "/announcements",
            )
            .await
            .ok()
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
