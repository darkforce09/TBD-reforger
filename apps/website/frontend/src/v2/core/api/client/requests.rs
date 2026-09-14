//! The request verbs: one code path for every call the app makes to the backend.
//!
//! **Role:** builds each request, injects the bearer token, and routes the whole thing through the
//! single retry contract so no verb can drift from the others.
//! **Position:** the public surface of the API client; pages and the editor call only these.
//! **Signals & state:** reads `AuthStore` tokens untracked and writes the rotated pair back after a
//! refresh, persisting it. Shares the refresh module's single-flight cell.
//! **Invariants:** browser-only — every function here needs `fetch`. A non-2xx answer is turned
//! into `(status, message)` with the backend's own error body parsed out; a request that never
//! reached the backend is reported as status `0`. The retry closure may run twice, so whatever it
//! sends must be cloneable per attempt.

use super::refresh::{refresh_locked, REFRESH_SF};
use super::{error_body_message, send_with_refresh, ApiErr, Req, API_BASE};
use crate::v2::core::api::dto::MeResponse;
use crate::v2::core::auth::{load_persisted, persist, AuthStore, RefreshResponse, Session};
use futures::future::FutureExt;
use leptos::prelude::*;
use serde::de::DeserializeOwned;

/// How a 2xx response body is consumed.
enum Consume<T> {
    /// Deserialise the JSON body.
    Json(std::marker::PhantomData<T>),
    /// Ignore the body — for `204`s and for mutations whose response the caller discards, carrying
    /// the value to return instead.
    Ignore(T),
}

/// What a request sends, and what one attempt of it costs.
///
/// The retry closure may run twice, so whatever it sends has to be cloned per attempt. That clone
/// is why this enum exists: duplicating a parsed JSON tree costs several times the document's own
/// bytes, which on a large editor payload is tens of megabytes, while duplicating an
/// already-serialised buffer bumps a reference count and copies nothing.
#[derive(Clone)]
enum Body {
    /// No body — `GET` and `DELETE`.
    None,
    /// A value the client serialises itself, once per attempt. Sets the content type.
    Json(serde_json::Value),
    /// An already-serialised JSON document, shared between attempts rather than duplicated. The
    /// content type is set by hand, because the call that normally sets it is exactly the one this
    /// variant exists to skip, and the backend's JSON extractor rejects the request without it.
    Raw(std::rc::Rc<String>),
}

/// One request through the client's contract: inject the bearer token, single-flight a `401`
/// refresh, and retry exactly once.
///
/// Every public verb is a thin wrapper around this, so the contract cannot diverge per verb. The
/// URL is built once and the retry closure clones the owned copy per attempt, which is what lets
/// `path` be borrowed for the duration of the call rather than `'static`.
async fn request<T: DeserializeOwned + Clone + 'static>(
    store: AuthStore,
    method: gloo_net::http::Method,
    path: &str,
    body: Body,
    consume: Consume<T>,
) -> Result<T, ApiErr> {
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
            let built = match &body {
                Body::Json(b) => req.json(b),
                // `.body(&str)` is what `.json` does after serialising — minus the
                // serialise. The header is therefore ours to set.
                Body::Raw(s) => req
                    .header("Content-Type", "application/json")
                    .body(s.as_str()),
                Body::None => req.build(),
            };
            let Ok(req) = built else {
                return Err((0u16, None));
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
                        // Surface the backend's own error string, and the findings behind it.
                        let msg = resp
                            .json::<serde_json::Value>()
                            .await
                            .ok()
                            .and_then(|v| error_body_message(&v));
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
        move || refresh_locked(store).boxed_local(),
        move |r: &RefreshResponse| {
            store.set_tokens(r.clone());
            persist(&store.persist_state());
        },
    )
    .await
}

/// `GET` `path`, relative to the API root. Returns the deserialised body, or the status.
pub async fn api_get<T: DeserializeOwned + Clone + 'static>(
    store: AuthStore,
    path: &str,
) -> Result<T, ApiErr> {
    request(
        store,
        gloo_net::http::Method::GET,
        path,
        Body::None,
        Consume::Json(std::marker::PhantomData),
    )
    .await
}

/// `POST` `path` with a JSON body. Returns the deserialised 2xx body, or the status; the caller
/// maps whatever route-specific statuses it cares about.
pub async fn api_post<T: DeserializeOwned + Clone + 'static>(
    store: AuthStore,
    path: &str,
    body: serde_json::Value,
) -> Result<T, ApiErr> {
    request(
        store,
        gloo_net::http::Method::POST,
        path,
        Body::Json(body),
        Consume::Json(std::marker::PhantomData),
    )
    .await
}

/// `PUT` `path` with a JSON body. Returns the deserialised 2xx body, or the status.
#[allow(dead_code)] // wired by the  suite live-wire
pub async fn api_put<T: DeserializeOwned + Clone + 'static>(
    store: AuthStore,
    path: &str,
    body: serde_json::Value,
) -> Result<T, ApiErr> {
    request(
        store,
        gloo_net::http::Method::PUT,
        path,
        Body::Json(body),
        Consume::Json(std::marker::PhantomData),
    )
    .await
}

/// `PATCH` `path` with a JSON body. Returns the deserialised 2xx body, or the status.
#[allow(dead_code)] // wired by the  suite live-wire
pub async fn api_patch<T: DeserializeOwned + Clone + 'static>(
    store: AuthStore,
    path: &str,
    body: serde_json::Value,
) -> Result<T, ApiErr> {
    request(
        store,
        gloo_net::http::Method::PATCH,
        path,
        Body::Json(body),
        Consume::Json(std::marker::PhantomData),
    )
    .await
}

/// `DELETE` `path`. The response body is ignored.
#[allow(dead_code)] // wired by the  suite live-wire
pub async fn api_delete(store: AuthStore, path: &str) -> Result<(), ApiErr> {
    request(
        store,
        gloo_net::http::Method::DELETE,
        path,
        Body::None,
        Consume::Ignore(()),
    )
    .await
}

/// `POST` `path` with a JSON body whose response the caller discards.
#[allow(dead_code)] // wired by the  suite live-wire
pub async fn api_post_ok(
    store: AuthStore,
    path: &str,
    body: serde_json::Value,
) -> Result<(), ApiErr> {
    request(
        store,
        gloo_net::http::Method::POST,
        path,
        Body::Json(body),
        Consume::Ignore(()),
    )
    .await
}

/// `POST` an already-serialised JSON document, without re-serialising or duplicating it.
///
/// Returns `` rather than a deserialised body on purpose: the routes that take a whole document
/// echo it back in their response, and a generic return would invite the caller to parse a second
/// copy of it only to throw that away.
///
/// Everything else matches [`api_post`] — the same request path, so the same bearer injection, the
/// same single flight, the same one retry, and the same error-body handling.
#[allow(dead_code)] // caller is missions.rs:1919 — a later slice; see this fn's doc + .
pub async fn api_post_raw(store: AuthStore, path: &str, body: String) -> Result<(), ApiErr> {
    request(
        store,
        gloo_net::http::Method::POST,
        path,
        Body::Raw(std::rc::Rc::new(body)),
        Consume::Ignore(()),
    )
    .await
}

/// `POST` a multipart upload under the form field `file`.
///
/// Same authentication contract as the JSON verbs. The content type is deliberately not set: the
/// browser supplies it, with the boundary, when the body is form data.
pub async fn api_upload_file<T: DeserializeOwned + Clone + 'static>(
    store: AuthStore,
    path: &str,
    file: web_sys::File,
) -> Result<T, ApiErr> {
    let sf = REFRESH_SF.with(|s| s.clone());
    let url = format!("{API_BASE}{path}");
    let send = move |tok: Option<String>| -> Req<T> {
        let url = url.clone();
        let file = file.clone();
        async move {
            let Ok(form) = web_sys::FormData::new() else {
                return Err((0u16, None));
            };
            if form
                .append_with_blob_and_filename("file", file.as_ref(), &file.name())
                .is_err()
            {
                return Err((0u16, None));
            }
            let mut req = gloo_net::http::Request::post(&url)
                .credentials(web_sys::RequestCredentials::Include);
            if let Some(t) = tok {
                req = req.header("Authorization", &format!("Bearer {t}"));
            }
            let Ok(req) = req.body(form) else {
                return Err((0u16, None));
            };
            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if (200..300).contains(&status) {
                        resp.json::<T>().await.map_err(|_| (0u16, None))
                    } else {
                        let msg = resp
                            .json::<serde_json::Value>()
                            .await
                            .ok()
                            .and_then(|v| error_body_message(&v));
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
        move || refresh_locked(store).boxed_local(),
        move |r: &RefreshResponse| {
            store.set_tokens(r.clone());
            persist(&store.persist_state());
        },
    )
    .await
}

/// Cold-start bootstrap: hydrate the tokens from persisted storage, then fetch the current user.
///
/// A stale or absent access token handles itself through the usual `401` refresh-and-retry path.
/// Stays a guest and does nothing when nothing is persisted.
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
