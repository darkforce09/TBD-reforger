//! HTTP client retry contract — ports the api/client.ts response interceptor.
//!
//! On a 401 the client refreshes the token **once** (through the single-flight cell so concurrent
//! 401s share one refresh) and retries the original request **once** with the rotated access token;
//! any other status, or a retry that is still 401, propagates. The state machine is generic over
//! the transport so it is unit-tested natively (single-retry, no loop); the wasm client wires
//! `send`/`refresh` to gloo-net on top.

use crate::auth::{RefreshResponse, SingleFlight};
use futures::future::LocalBoxFuture;

/// Request failure: HTTP status (0 = network/serde) + the backend's `{"error": …}` body string when
/// one was sent. Carrying the message is the T-127 U5 parity — ORBAT toasts surface "slot already
/// taken" vs "squad is reserved by a leader", not one flattened failure line.
pub type ApiErr = (u16, Option<String>);

/// A pending request: resolves to `Ok(T)` or `Err((status, backend_error))`.
pub type Req<T> = LocalBoxFuture<'static, Result<T, ApiErr>>;

/// `apiErrorMessage` (pages/events.tsx): the backend's error string, first letter capitalized,
/// else the caller's fallback.
#[allow(dead_code)]
pub fn api_error_message(err: &ApiErr, fallback: &str) -> String {
    match &err.1 {
        Some(msg) if !msg.is_empty() => {
            let mut c = msg.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => fallback.to_string(),
            }
        }
        _ => fallback.to_string(),
    }
}

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
) -> Result<T, ApiErr> {
    match send(token()).await {
        Err((401, _)) => match sf.run(refresh).await {
            Some(r) => {
                on_refreshed(&r);
                send(Some(r.access_token)).await // the single retry
            }
            None => Err((401, None)),
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

    /// How a 2xx response body is consumed.
    enum Consume<T> {
        /// Deserialize the JSON body (`resp.json::<T>()`).
        Json(std::marker::PhantomData<T>),
        /// Ignore the body — for 204s and mutations whose response the caller discards.
        Ignore(T),
    }

    /// One request through the api/client.ts contract: bearer inject + single-flight 401 refresh +
    /// exactly one retry (`send_with_refresh`). All public verbs below are thin wrappers so the
    /// contract can never diverge per-verb. `body: Some(v)` sends JSON (sets Content-Type).
    async fn request<T: DeserializeOwned + Clone + 'static>(
        store: AuthStore,
        method: gloo_net::http::Method,
        path: &str,
        body: Option<serde_json::Value>,
        consume: Consume<T>,
    ) -> Result<T, super::ApiErr> {
        let sf = REFRESH_SF.with(|s| s.clone());
        // Build the URL once (so `path` need only live for this call, not `'static`) — the retry
        // closure clones the owned URL per attempt. Param routes (/missions/:id) pass a dynamic path.
        let url = format!("{API_BASE}{path}");
        let ignore = match &consume {
            Consume::Json(_) => None,
            Consume::Ignore(v) => Some(v.clone()),
        };
        let send = move |tok: Option<String>| -> Req<T> {
            let url = url.clone();
            let method = method.clone();
            let body = body.clone();
            let ignore = ignore.clone();
            async move {
                let mut req = gloo_net::http::RequestBuilder::new(&url)
                    .method(method)
                    .credentials(web_sys::RequestCredentials::Include);
                if let Some(t) = tok {
                    req = req.header("Authorization", &format!("Bearer {t}"));
                }
                let req = match body {
                    Some(b) => {
                        let Ok(r) = req.json(&b) else {
                            return Err((0u16, None));
                        };
                        r
                    }
                    None => {
                        let Ok(r) = req.build() else {
                            return Err((0u16, None));
                        };
                        r
                    }
                };
                match req.send().await {
                    Ok(resp) => {
                        let status = resp.status();
                        if (200..300).contains(&status) {
                            match ignore {
                                Some(v) => Ok(v),
                                None => resp.json::<T>().await.map_err(|_| (0u16, None)),
                            }
                        } else {
                            // Surface the backend's `{"error": …}` string (T-127 U5 toasts).
                            let msg = resp
                                .json::<serde_json::Value>()
                                .await
                                .ok()
                                .and_then(|v| v.get("error")?.as_str().map(str::to_string));
                            Err((status, msg))
                        }
                    }
                    Err(_) => Err((0u16, None)),
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

    /// GET `path` (relative to /api/v1). Returns the deserialized body or the HTTP status.
    pub async fn api_get<T: DeserializeOwned + Clone + 'static>(
        store: AuthStore,
        path: &str,
    ) -> Result<T, super::ApiErr> {
        request(
            store,
            gloo_net::http::Method::GET,
            path,
            None,
            Consume::Json(std::marker::PhantomData),
        )
        .await
    }

    /// POST `path` with a JSON body. Returns the deserialized 2xx body or the HTTP status; the
    /// caller maps route-specific statuses (e.g. the versions route's 409/413). T-159.20.
    pub async fn api_post<T: DeserializeOwned + Clone + 'static>(
        store: AuthStore,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T, super::ApiErr> {
        request(
            store,
            gloo_net::http::Method::POST,
            path,
            Some(body),
            Consume::Json(std::marker::PhantomData),
        )
        .await
    }

    /// PUT `path` with a JSON body (useAssignSlot / useSaveFaction-update). T-159.24.
    #[allow(dead_code)] // wired by the T-159.25 suite live-wire
    pub async fn api_put<T: DeserializeOwned + Clone + 'static>(
        store: AuthStore,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T, super::ApiErr> {
        request(
            store,
            gloo_net::http::Method::PUT,
            path,
            Some(body),
            Consume::Json(std::marker::PhantomData),
        )
        .await
    }

    /// PATCH `path` with a JSON body (useSetMissionStatus / useUpdateUserRole). T-159.24.
    #[allow(dead_code)] // wired by the T-159.25 suite live-wire
    pub async fn api_patch<T: DeserializeOwned + Clone + 'static>(
        store: AuthStore,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T, super::ApiErr> {
        request(
            store,
            gloo_net::http::Method::PATCH,
            path,
            Some(body),
            Consume::Json(std::marker::PhantomData),
        )
        .await
    }

    /// DELETE `path`, ignoring any response body (the delete mutations get 204s or discard the
    /// body — axios parity). Ok(()) on 2xx. T-159.24.
    #[allow(dead_code)] // wired by the T-159.25 suite live-wire
    pub async fn api_delete(store: AuthStore, path: &str) -> Result<(), super::ApiErr> {
        request(
            store,
            gloo_net::http::Method::DELETE,
            path,
            None,
            Consume::Ignore(()),
        )
        .await
    }

    /// POST `path` with a JSON body, ignoring any response body (register/reserve/release/logout —
    /// React invalidates queries and discards the response). Ok(()) on 2xx. T-159.24.
    #[allow(dead_code)] // wired by the T-159.25 suite live-wire
    pub async fn api_post_ok(store: AuthStore, path: &str, body: serde_json::Value) -> Result<(), super::ApiErr> {
        request(
            store,
            gloo_net::http::Method::POST,
            path,
            Some(body),
            Consume::Ignore(()),
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
#[allow(unused_imports)] // the T-159.24 verbs are wired by the T-159.25 suite live-wire
pub use wasm_client::{
    api_delete, api_get, api_patch, api_post, api_post_ok, api_put, bootstrap,
};

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
        let out: Result<&str, ApiErr> = block_on(send_with_refresh(
            &sf,
            move |tok| {
                let s = s.clone();
                async move {
                    s.set(s.get() + 1);
                    if tok.as_deref() == Some("new") {
                        Ok("ok")
                    } else {
                        Err((401u16, None))
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
        let out: Result<&str, ApiErr> = block_on(send_with_refresh(
            &sf,
            move |_tok| {
                let s = s.clone();
                async move {
                    s.set(s.get() + 1);
                    Err((401u16, None))
                }
                .boxed_local()
            },
            || Some("stale".to_string()),
            || async { Some(rr("new")) }.boxed_local(),
            |_| {},
        ));
        assert_eq!(out, Err((401, None)));
        assert_eq!(sends.get(), 2, "one retry only — no loop");
    }

    // A non-401 error is not retried and does not refresh.
    #[test]
    fn non_401_propagates_without_refresh() {
        let refreshes = Rc::new(Cell::new(0));
        let sf = SingleFlight::<Option<RefreshResponse>>::new();
        let r = refreshes.clone();
        let out: Result<&str, ApiErr> = block_on(send_with_refresh(
            &sf,
            |_tok| async { Err((500u16, None)) }.boxed_local(),
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
        assert_eq!(out, Err((500, None)));
        assert_eq!(refreshes.get(), 0, "non-401 never refreshes");
    }
}
