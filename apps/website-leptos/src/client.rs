//! HTTP client retry contract — ports the api/client.ts response interceptor.
//!
//! On a 401 the client refreshes the token **once** (through the single-flight cell so concurrent
//! 401s share one refresh) and retries the original request **once** with the rotated access token;
//! any other status, or a retry that is still 401, propagates. The state machine is generic over
//! the transport so it is unit-tested natively (single-retry, no loop); the wasm client wires
//! `send`/`refresh` to gloo-net on top.

use crate::auth::{RefreshResponse, SingleFlight};
use futures::future::LocalBoxFuture;

/// A pending request: resolves to `Ok(T)` or `Err(status)`.
pub type Req<T> = LocalBoxFuture<'static, Result<T, u16>>;

/// Send `send(token)`; on 401, single-flight `refresh`, apply it via `on_refreshed`, and retry once
/// with the rotated token. Mirrors the `!original._retry` guard in api/client.ts (exactly one retry).
// Wired to gloo-net in the wasm client next; the retry state machine is unit-tested now.
#[allow(dead_code)]
pub async fn send_with_refresh<T>(
    sf: &SingleFlight<Option<RefreshResponse>>,
    send: impl Fn(Option<String>) -> Req<T>,
    token: impl Fn() -> Option<String>,
    refresh: impl FnOnce() -> LocalBoxFuture<'static, Option<RefreshResponse>>,
    on_refreshed: impl FnOnce(&RefreshResponse),
) -> Result<T, u16> {
    match send(token()).await {
        Err(401) => match sf.run(refresh).await {
            Some(r) => {
                on_refreshed(&r);
                send(Some(r.access_token)).await // the single retry
            }
            None => Err(401),
        },
        other => other,
    }
}

/* ─────────────────────────── gloo-net client + bootstrap (wasm) ─────────────────────────── */

#[cfg(target_arch = "wasm32")]
mod wasm_client {
    use super::{send_with_refresh, Req};
    use crate::auth::{load_persisted, persist, AuthStore, RefreshResponse, Session, SingleFlight};
    use crate::dto::MeResponse;
    use futures::future::FutureExt;
    use leptos::prelude::*;
    use serde::de::DeserializeOwned;

    const API_BASE: &str = "/api/v1";

    thread_local! {
        // Module-level single-flight cell — mirrors refresh.ts's `inflight`.
        static REFRESH_SF: SingleFlight<Option<RefreshResponse>> = SingleFlight::new();
    }

    async fn refresh_via_gloo(store: AuthStore) -> Option<RefreshResponse> {
        let rt = store.refresh_token.get_untracked();
        let body = serde_json::json!({ "refresh_token": rt });
        let req = gloo_net::http::Request::post(&format!("{API_BASE}/auth/refresh"))
            .credentials(web_sys::RequestCredentials::Include)
            .json(&body)
            .ok()?;
        match req.send().await {
            Ok(resp) if (200..300).contains(&resp.status()) => {
                resp.json::<RefreshResponse>().await.ok()
            }
            _ => None,
        }
    }

    /// GET `path` (relative to /api/v1) with the api/client.ts contract: bearer inject + single-flight
    /// 401 refresh + one retry. Returns the deserialized body or the HTTP status.
    pub async fn api_get<T: DeserializeOwned + 'static>(
        store: AuthStore,
        path: &'static str,
    ) -> Result<T, u16> {
        let sf = REFRESH_SF.with(|s| s.clone());
        let send = move |tok: Option<String>| -> Req<T> {
            let url = format!("{API_BASE}{path}");
            async move {
                let mut req = gloo_net::http::Request::get(&url)
                    .credentials(web_sys::RequestCredentials::Include);
                if let Some(t) = tok {
                    req = req.header("Authorization", &format!("Bearer {t}"));
                }
                match req.send().await {
                    Ok(resp) => {
                        let status = resp.status();
                        if (200..300).contains(&status) {
                            resp.json::<T>().await.map_err(|_| 0u16)
                        } else {
                            Err(status)
                        }
                    }
                    Err(_) => Err(0u16),
                }
            }
            .boxed_local()
        };
        send_with_refresh(
            &sf,
            send,
            move || store.access_token.get_untracked(),
            move || refresh_via_gloo(store).boxed_local(),
            move |r: &RefreshResponse| {
                store.set_tokens(r.clone());
                persist(&store.persist_state());
            },
        )
        .await
    }

    /// Cold-load bootstrap (useAuthBootstrap): hydrate tokens from tbd-auth, then GET /me — which
    /// self-handles a stale/absent access token via the 401 → single-flight refresh → retry path.
    /// No-ops (stays guest) when nothing is persisted.
    pub async fn bootstrap(store: AuthStore) {
        let Some(p) = load_persisted() else {
            return;
        };
        let Some(rt) = p.refresh_token else {
            return;
        };
        store.refresh_token.set(Some(rt));
        store.expires_at.set(p.expires_at);
        if let Some(u) = p.user {
            store.user.set(Some(u));
        }
        store.bootstrapping.set(true);
        if let Ok(me) = api_get::<MeResponse>(store, "/me").await {
            store.set_session(Session {
                access_token: store.access_token.get_untracked().unwrap_or_default(),
                refresh_token: store.refresh_token.get_untracked().unwrap_or_default(),
                expires_at: store.expires_at.get_untracked().unwrap_or_default(),
                user: me.user,
                arma_linked: me.arma_linked,
            });
            persist(&store.persist_state());
        }
        store.bootstrapping.set(false);
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm_client::{api_get, bootstrap};

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use futures::FutureExt;
    use std::cell::Cell;
    use std::rc::Rc;

    fn rr(access: &str) -> RefreshResponse {
        RefreshResponse {
            access_token: access.into(),
            refresh_token: "r".into(),
            expires_at: "e".into(),
        }
    }

    // The api/client.ts contract: a 401 refreshes once and retries once with the new token.
    #[test]
    fn retries_once_after_refresh() {
        let sends = Rc::new(Cell::new(0));
        let refreshes = Rc::new(Cell::new(0));
        let sf = SingleFlight::<Option<RefreshResponse>>::new();
        let s = sends.clone();
        let r = refreshes.clone();
        let out: Result<&str, u16> = block_on(send_with_refresh(
            &sf,
            move |tok| {
                let s = s.clone();
                async move {
                    s.set(s.get() + 1);
                    if tok.as_deref() == Some("new") {
                        Ok("ok")
                    } else {
                        Err(401u16)
                    }
                }
                .boxed_local()
            },
            || Some("stale".to_string()),
            move || {
                let r = r.clone();
                async move {
                    r.set(r.get() + 1);
                    Some(rr("new"))
                }
                .boxed_local()
            },
            |_| {},
        ));
        assert_eq!(out, Ok("ok"));
        assert_eq!(refreshes.get(), 1, "exactly one refresh");
        assert_eq!(sends.get(), 2, "original + exactly one retry");
    }

    // No retry loop: a still-401 retry gives up (send twice total, then propagate 401).
    #[test]
    fn no_loop_if_retry_still_401() {
        let sends = Rc::new(Cell::new(0));
        let sf = SingleFlight::<Option<RefreshResponse>>::new();
        let s = sends.clone();
        let out: Result<&str, u16> = block_on(send_with_refresh(
            &sf,
            move |_tok| {
                let s = s.clone();
                async move {
                    s.set(s.get() + 1);
                    Err(401u16)
                }
                .boxed_local()
            },
            || Some("stale".to_string()),
            || async { Some(rr("new")) }.boxed_local(),
            |_| {},
        ));
        assert_eq!(out, Err(401));
        assert_eq!(sends.get(), 2, "one retry only — no loop");
    }

    // A non-401 error is not retried and does not refresh.
    #[test]
    fn non_401_propagates_without_refresh() {
        let refreshes = Rc::new(Cell::new(0));
        let sf = SingleFlight::<Option<RefreshResponse>>::new();
        let r = refreshes.clone();
        let out: Result<&str, u16> = block_on(send_with_refresh(
            &sf,
            |_tok| async { Err(500u16) }.boxed_local(),
            || Some("t".to_string()),
            move || {
                let r = r.clone();
                async move {
                    r.set(r.get() + 1);
                    Some(rr("new"))
                }
                .boxed_local()
            },
            |_| {},
        ));
        assert_eq!(out, Err(500));
        assert_eq!(refreshes.get(), 0, "non-401 never refreshes");
    }
}
