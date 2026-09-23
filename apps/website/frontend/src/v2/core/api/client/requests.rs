//! The request verbs: one code path for every call the app makes to the backend.
//!
//! **Role:** builds each request, injects the bearer token, and routes the whole thing through the
//! single retry contract so no verb can drift from the others.
//! **Position:** the public surface of the API client; pages and the editor call only these.
//! **Signals & state:** reads `AuthStore` tokens untracked and writes the rotated pair back after a
//! refresh, persisting it. Shares the refresh module's single-flight cell.
//! **Invariants:** browser-only — every function here needs `fetch`. A non-2xx answer is turned
//! into `(status, message)` with the backend's own error body parsed out; a request that never
//! reached the backend is reported as status `0`. The refusal-keeping verbs differ only in that
//! failure: they hand a refused answer back as an [`ApiRefusal`] carrying the structured reason its
//! body names, while a `401` still goes through the one refresh-and-retry contract. The retry
//! closure may run twice, so whatever it sends must be cloneable per attempt.

use super::refresh::{refresh_locked, send_with_refresh_for_generation, REFRESH_SF};
use super::refusals::{decode_answer, ApiRefusal};
use super::{error_body_message, ApiErr, Req, API_BASE};
use crate::v2::core::api::dto::MeResponse;
use crate::v2::core::auth::{load_persisted, AuthStore, RefreshResponse};
use futures::future::FutureExt;
use leptos::prelude::*;
use serde::de::DeserializeOwned;

/// A backend answer kept whole: its status, and its body when that parsed as JSON.
type KeptAnswer = (u16, Option<serde_json::Value>);

/// How a response body is consumed.
#[derive(Clone)]
enum Consume<T> {
    /// Deserialise the JSON body of a 2xx answer.
    Json(std::marker::PhantomData<T>),
    /// Ignore the body — for `204`s and for mutations whose response the caller discards, carrying
    /// the value to return instead.
    Ignore(T),
    /// Keep every answer but a `401` whole, refusals included, for a caller that branches on the
    /// reason a refusal names. The function builds the value from the status and the parsed body.
    Answer(fn(u16, Option<serde_json::Value>) -> T),
}

impl<T> Consume<T> {
    /// The answer builder, when this request reads a refused answer as its answer. A `401` never
    /// is one: it goes through the refresh contract like any other request's.
    fn refusal_reader(&self, status: u16) -> Option<fn(u16, Option<serde_json::Value>) -> T> {
        match self {
            Consume::Answer(keep) if status != 401 => Some(*keep),
            _ => None,
        }
    }
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
    // A request issued during the cold start is sent under the restored session, never under
    // the generation the restore is about to end.
    store.session_restored().await;
    let generation = store.current_generation();
    let sf = REFRESH_SF.with(|s| s.clone());
    // Build the URL once (so `path` need only live for this call, not `'static`) — the retry
    // closure clones the owned URL per attempt. Param routes (/missions/:id) pass a dynamic path.
    let url = format!("{API_BASE}{path}");
    let send = move |tok: Option<String>| -> Req<T> {
        let url = url.clone();
        let method = method.clone();
        let body = body.clone();
        let consume = consume.clone();
        async move {
            if !store.is_current_generation(generation) {
                return Err((401, None));
            }
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
                        match consume {
                            Consume::Json(_) => resp.json::<T>().await.map_err(|_| (0u16, None)),
                            Consume::Ignore(v) => Ok(v),
                            Consume::Answer(keep) => resp
                                .json::<serde_json::Value>()
                                .await
                                .map(|v| keep(status, Some(v)))
                                .map_err(|_| (0u16, None)),
                        }
                    } else if let Some(keep) = consume.refusal_reader(status) {
                        // The refusal is the answer this caller reads; its body names the reason.
                        Ok(keep(status, resp.json::<serde_json::Value>().await.ok()))
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
    let result = send_with_refresh_for_generation(
        &sf,
        generation,
        move || store.is_current_generation(generation),
        send,
        move || store.access_token.get_untracked(),
        move || {
            async move {
                if !store.is_current_generation(generation) {
                    return None;
                }
                refresh_locked(store).await
            }
            .boxed_local()
        },
        move |r: &RefreshResponse| {
            if store.is_current_generation(generation) {
                store.set_tokens(r.clone());
            }
        },
    )
    .await;
    if store.is_current_generation(generation) {
        result
    } else {
        Err((401, None))
    }
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
#[allow(dead_code)] // Called only from browser-side pages; the native build has no caller.
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
#[allow(dead_code)] // Called only from browser-side pages; the native build has no caller.
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
#[allow(dead_code)] // Called only from browser-side pages; the native build has no caller.
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
#[allow(dead_code)] // Called only from browser-side pages; the native build has no caller.
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
/// Returns `()` rather than a deserialised body on purpose: the routes that take a whole document
/// echo it back in their response, and a generic return would invite the caller to parse a second
/// copy of it only to throw that away.
///
/// Everything else matches [`api_post`] — the same request path, so the same bearer injection, the
/// same single flight, the same one retry, and the same error-body handling.
#[allow(dead_code)] // Reserved for the whole-document upload path; no caller today.
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

/// One request whose refusal the caller reads: the shared request path, so the same bearer
/// injection, single flight and single retry, with a refused answer decoded into the reason its
/// body names instead of being flattened to a sentence.
async fn request_keeping_refusal<T: DeserializeOwned>(
    store: AuthStore,
    method: gloo_net::http::Method,
    path: &str,
    body: Body,
) -> Result<T, ApiRefusal> {
    let keep: fn(u16, Option<serde_json::Value>) -> KeptAnswer = |status, body| (status, body);
    let (status, body) = request(store, method, path, body, Consume::Answer(keep))
        .await
        .map_err(ApiRefusal::from)?;
    decode_answer(status, body)
}

/// `POST` `path` with a JSON body. Returns the deserialised 2xx body, or the refusal with the
/// structured reason its body names.
#[allow(dead_code)] // Called only from browser-side pages; the native build has no caller.
pub async fn api_post_keeping_refusal<T: DeserializeOwned>(
    store: AuthStore,
    path: &str,
    body: serde_json::Value,
) -> Result<T, ApiRefusal> {
    request_keeping_refusal(store, gloo_net::http::Method::POST, path, Body::Json(body)).await
}

/// `PUT` `path` with a JSON body, keeping a refusal's structured reason.
#[allow(dead_code)] // Called only from browser-side pages; the native build has no caller.
pub async fn api_put_keeping_refusal<T: DeserializeOwned>(
    store: AuthStore,
    path: &str,
    body: serde_json::Value,
) -> Result<T, ApiRefusal> {
    request_keeping_refusal(store, gloo_net::http::Method::PUT, path, Body::Json(body)).await
}

/// `PATCH` `path` with a JSON body, keeping a refusal's structured reason.
#[allow(dead_code)] // Called only from browser-side pages; the native build has no caller.
pub async fn api_patch_keeping_refusal<T: DeserializeOwned>(
    store: AuthStore,
    path: &str,
    body: serde_json::Value,
) -> Result<T, ApiRefusal> {
    request_keeping_refusal(store, gloo_net::http::Method::PATCH, path, Body::Json(body)).await
}

/// `DELETE` `path`, reading the answer's body and keeping a refusal's structured reason.
#[allow(dead_code)] // Called only from browser-side pages; the native build has no caller.
pub async fn api_delete_keeping_refusal<T: DeserializeOwned>(
    store: AuthStore,
    path: &str,
) -> Result<T, ApiRefusal> {
    request_keeping_refusal(store, gloo_net::http::Method::DELETE, path, Body::None).await
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
    store.session_restored().await;
    let generation = store.current_generation();
    let sf = REFRESH_SF.with(|s| s.clone());
    let url = format!("{API_BASE}{path}");
    let send = move |tok: Option<String>| -> Req<T> {
        let url = url.clone();
        let file = file.clone();
        async move {
            if !store.is_current_generation(generation) {
                return Err((401, None));
            }
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
    let result = send_with_refresh_for_generation(
        &sf,
        generation,
        move || store.is_current_generation(generation),
        send,
        move || store.access_token.get_untracked(),
        move || {
            async move {
                if !store.is_current_generation(generation) {
                    return None;
                }
                refresh_locked(store).await
            }
            .boxed_local()
        },
        move |r: &RefreshResponse| {
            if store.is_current_generation(generation) {
                store.set_tokens(r.clone());
            }
        },
    )
    .await;
    if store.is_current_generation(generation) {
        result
    } else {
        Err((401, None))
    }
}

/// Cold-start bootstrap: hydrate the tokens from persisted storage, then fetch the current user.
///
/// A stale or absent access token handles itself through the usual `401` refresh-and-retry path.
/// When nothing is persisted it settles the restore and stays a guest.
pub async fn bootstrap(store: AuthStore) {
    // A peer may temporarily remove its single-use credential while rotating it. Read only after
    // that peer releases the lock, so a new tab observes the settled result.
    let persisted =
        super::refresh::with_refresh_lock(async { load_persisted() }.boxed_local()).await;
    let Some(persisted) = persisted.filter(|p| p.refresh_token.is_some()) else {
        store.settle_session_restore();
        store.bootstrapping.set(false);
        return;
    };
    store.restore_persisted(persisted);
    store.bootstrapping.set(true);
    let generation = store.current_generation();
    let profile_request = store.begin_profile_request();
    if let Ok(me) = api_get::<MeResponse>(store, "/me").await {
        if store.adopt_profile(profile_request, &me) {
            crate::v2::core::auth::session::persist_profile_if_current(&store.persist_state())
                .await;
        }
    }
    if store.is_current_generation(generation) {
        store.bootstrapping.set(false);
    }
}
