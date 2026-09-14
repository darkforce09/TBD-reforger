//! The server intel route: what the dedicated servers are doing, live.
//!
//! **Role:** fetches the server list, picks the one to show, opens the telemetry stream for it
//! and hands both to the panel.
//! **Position:** the `/server-intel` route, rendered inside the navigation frame behind the
//! sign-in gate.
//! **Signals & state:** reads the session store from context. Owns the resource holding the
//! server list and four signals for the stream: the latest status frame, whether the stream is
//! connected, its last error, and whether a subscription has already been made.
//! **Invariants:** the request future is not `Send`, so the fetch is a browser-only path and a
//! native build resolves to `None` and renders the failure branch. At most one subscription is
//! ever opened, guarded by the `subscribed` flag read untracked.
#![allow(dead_code)]

use super::server_list::{panel, pick_default};
use crate::v2::core::api::dto::{DataEnvelope, ServerStatusDto};
use crate::v2::core::ui::AuthGate;
use leptos::prelude::*;
use serde_json::Value;

// The identifier read is only needed to open the stream, which is a browser-only path.
#[cfg(target_arch = "wasm32")]
use super::server_list::v_str;

#[cfg(test)]
#[path = "tests/server_intel_t385.rs"]
mod t385;

#[cfg(test)]
#[path = "tests/server_intel_t773.rs"]
mod t773;

/// The `/server-intel` route: the panel behind the sign-in gate.
#[component]
pub fn ServerIntelPage() -> impl IntoView {
    view! {
        <AuthGate>
            <ServerIntelInner />
        </AuthGate>
    }
}

/// The signed-in half of the page: the fetch, the stream subscription and the render states.
///
/// Renders a loading line while the list is in flight, an error line when it failed, and the
/// panel once the list arrived — even when that list is empty.
#[component]
fn ServerIntelInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let servers = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<DataEnvelope<Value>>(store, "/servers")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<Value>>
        }
    });
    let live = RwSignal::new(None::<ServerStatusDto>);
    let connected = RwSignal::new(false);
    let sse_error = RwSignal::new(None::<String>);
    let subscribed = RwSignal::new(false);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (connected, sse_error, subscribed);
    // The abort handle is not `Send`, so route-leave cleanup is a zero-capture function that
    // reaches it through a thread-local. It has to be registered on the page owner rather than
    // inside the suspense fragment: a fragment re-run would abort a stream that is still mounted.
    #[cfg(target_arch = "wasm32")]
    on_cleanup(crate::v2::core::api::sse::abort_server_status_stream);

    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                servers
                    .get()
                    .map(|opt| match opt {
                        Some(env) => {
                            let server = pick_default(&env.data);
                            #[cfg(target_arch = "wasm32")]
                            if let Some(s) = &server {
                                let id = v_str(s, "id").to_string();
                                if !id.is_empty() && !subscribed.get_untracked() {
                                    subscribed.set(true);
                                    crate::v2::core::api::sse::stream_server_status(
                                        store,
                                        id,
                                        live,
                                        connected,
                                        sse_error,
                                    );
                                }
                            }
                            panel(server, live).into_any()
                        }
                        None => {
                            view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}
