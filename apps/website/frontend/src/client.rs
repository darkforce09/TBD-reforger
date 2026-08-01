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

/// Findings folded into an error message before the tail is summarised. The backend already caps
/// its own list (20); this is the second cap, sized for a dialog a human reads rather than a log.
pub const MAX_ERROR_DETAILS: usize = 6;

/// Pull the human-readable failure out of a backend error body: `{"error": …}` plus, when the
/// handler sent one, the `details` array that says *why*.
///
/// T-181.44 — `details` was being dropped on the floor. `create_version` answers 400 with the exact
/// list of things wrong with the payload and `/compiled` answers 500 with the schema findings; a
/// client that keeps only `error` turns both into "invalid mission payload", which names a verdict
/// and not a cause. Only a `details` that is an **array of strings** is folded in — `field_tools`
/// sends a partial mortar solution object there, and that is a payload for the caller to render,
/// not prose.
///
/// Extra findings arrive as `\n`-separated lines so [`ApiErr`] keeps its shape and every existing
/// caller compiles unchanged; [`split_error_lines`] is the reader.
#[allow(dead_code)]
pub fn error_body_message(body: &serde_json::Value) -> Option<String> {
    let error = body.get("error")?.as_str()?;
    let details: Vec<&str> = body
        .get("details")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<&str>>()
        })
        .unwrap_or_default();
    if details.is_empty() {
        return Some(error.to_string());
    }

    let shown = details.len().min(MAX_ERROR_DETAILS);
    let mut out = String::from(error);
    for d in &details[..shown] {
        out.push('\n');
        out.push_str(d);
    }
    if details.len() > shown {
        out.push_str(&format!("\n… and {} more", details.len() - shown));
    }
    Some(out)
}

/// Split an [`error_body_message`] back into its headline and its findings.
#[allow(dead_code)]
pub fn split_error_lines(msg: Option<&str>) -> (Option<String>, Vec<String>) {
    let Some(m) = msg else {
        return (None, Vec::new());
    };
    let mut lines = m.split('\n');
    let head = lines.next().map(str::to_string);
    (head, lines.map(str::to_string).collect())
}

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

    /// What this request sends, and at what cost per attempt.
    ///
    /// The retry closure is `Fn` (it may run twice — original + the post-refresh retry) and hands
    /// back a `'static` future, so whatever it sends has to be **cloned per attempt**. That clone
    /// is the whole reason this enum exists: cloning a [`Body::Json`] duplicates the entire
    /// `serde_json::Value` tree, which on `wasm32` is ~4.7x the document's own bytes (T-591,
    /// measured on a 33.6 MiB / 170k-slot editor payload). Cloning a [`Body::Raw`] bumps a
    /// refcount and copies nothing.
    #[derive(Clone)]
    enum Body {
        /// No body — GET and DELETE.
        None,
        /// A `Value` the client serialises itself, per attempt (`.json()` sets Content-Type).
        Json(serde_json::Value),
        /// An **already-serialised** JSON document. `Rc` so the retry shares the one buffer
        /// instead of duplicating it; Content-Type is set by hand because `.json()` — the thing
        /// that normally sets it — is exactly what this variant exists to skip, and Axum's `Json`
        /// extractor answers **415** without the header.
        Raw(std::rc::Rc<String>),
    }

    /// One request through the api/client.ts contract: bearer inject + single-flight 401 refresh +
    /// exactly one retry (`send_with_refresh`). All public verbs below are thin wrappers so the
    /// contract can never diverge per-verb — including [`api_post_raw`], whose only difference
    /// from [`api_post`] is which [`Body`] arm it hands in.
    async fn request<T: DeserializeOwned + Clone + 'static>(
        store: AuthStore,
        method: gloo_net::http::Method,
        path: &str,
        body: Body,
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
                let built = match &body {
                    Body::Json(b) => req.json(b),
                    // `.body(&str)` is what `.json()` does after serialising — minus the
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
                            // Surface the backend's `{"error": …}` string (T-127 U5 toasts) and,
                            // since T-181.44, the `details` findings behind it.
                            let msg = resp
                                .json::<serde_json::Value>()
                                .await
                                .ok()
                                .and_then(|v| super::error_body_message(&v));
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
            Body::None,
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
            Body::Json(body),
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
            Body::Json(body),
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
            Body::Json(body),
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
            Body::None,
            Consume::Ignore(()),
        )
        .await
    }

    /// POST `path` with a JSON body, ignoring any response body (register/reserve/release/logout —
    /// React invalidates queries and discards the response). Ok(()) on 2xx. T-159.24.
    #[allow(dead_code)] // wired by the T-159.25 suite live-wire
    pub async fn api_post_ok(
        store: AuthStore,
        path: &str,
        body: serde_json::Value,
    ) -> Result<(), super::ApiErr> {
        request(
            store,
            gloo_net::http::Method::POST,
            path,
            Body::Json(body),
            Consume::Ignore(()),
        )
        .await
    }

    /// POST `path` with an **already-serialised** JSON body, ignoring any response body.
    /// Ok(()) on 2xx. **T-591 — the raw-body twin of [`api_post`], and a memory fix, not a feature.**
    ///
    /// [`api_post`] takes an owned `serde_json::Value`, so the mission-upload path (T-117,
    /// `missions.rs`) holds the document as a `Value` tree **three times over** at the moment of
    /// the fetch: the parsed document, the `version_body` wrapper (`json!` clones the payload),
    /// and the per-attempt clone [`Body`] describes — plus the serialised bytes. Measured on
    /// wasm32 layout constants (`Value` 24 B, `String` 12 B, `Map` 12 B) over a 33.6 MiB /
    /// 170k-slot editor payload: **one tree is 4.7x the document**, so that path peaks at **~16x**
    /// in a 32-bit linear heap. Handing over a `String` drops the per-attempt clone
    /// unconditionally and lets the caller drop the wrapper clone too — **~6.7x** once
    /// `missions.rs` builds its body with `to_writer` instead of `to_string(version_body(..))`.
    ///
    /// Ok(()), not `Ok(T)`, **deliberately**: `POST /missions/:id/versions` answers 201 with a
    /// `MissionVersion` whose `json_payload` echoes the entire document back
    /// (`models/mission.rs:128`), so a `T`-generic version would invite the caller to parse a
    /// **fourth** tree out of the response and — as `missions.rs:1920`'s `Ok(_)` arm shows —
    /// throw it away. `Consume::Ignore` never reads that body at all.
    ///
    /// Everything else is [`api_post`]: same [`request`], so the same bearer inject, the same
    /// module `REFRESH_SF` single-flight (refresh tokens are single-use and rotated — a second
    /// refresh path would double-spend one), the same one retry, and the same non-2xx handling
    /// that folds `create_version`'s 400 `details` array into the message T-117 surfaces.
    #[allow(dead_code)] // caller is missions.rs:1919 — a later slice; see this fn's doc + T-591.
    pub async fn api_post_raw(
        store: AuthStore,
        path: &str,
        body: String,
    ) -> Result<(), super::ApiErr> {
        request(
            store,
            gloo_net::http::Method::POST,
            path,
            Body::Raw(std::rc::Rc::new(body)),
            Consume::Ignore(()),
        )
        .await
    }

    /// POST multipart upload — form field `"file"` (CMS `POST /cms/uploads`, T-446).
    ///
    /// Same auth contract as the JSON verbs (Bearer + single-flight 401 refresh + one retry).
    /// Does **not** set `Content-Type` — the browser supplies `multipart/form-data` with the
    /// boundary when the body is a `FormData`.
    pub async fn api_upload_file<T: DeserializeOwned + Clone + 'static>(
        store: AuthStore,
        path: &str,
        file: web_sys::File,
    ) -> Result<T, super::ApiErr> {
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
                                .and_then(|v| super::error_body_message(&v));
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
    api_delete, api_get, api_patch, api_post, api_post_ok, api_post_raw, api_put, api_upload_file,
    bootstrap,
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

    // T-181.44 — the reason a rejection is diagnosable at all. Before this the client kept only
    // `error`, so a 400 listing four bad callsigns arrived as "invalid mission payload".
    #[test]
    fn details_ride_along_with_the_error_string() {
        let body = serde_json::json!({
            "error": "invalid mission payload",
            "details": ["/editor/squads/0/callsign: bad", "/editor/slots/9/role: bad"],
        });
        let msg = error_body_message(&body).expect("message");
        let (head, rows) = split_error_lines(Some(&msg));
        assert_eq!(head.as_deref(), Some("invalid mission payload"));
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("/editor/squads/0/callsign:"));
    }

    #[test]
    fn a_long_findings_list_is_capped_and_the_tail_is_counted() {
        let details: Vec<String> = (0..MAX_ERROR_DETAILS + 4)
            .map(|i| format!("f{i}"))
            .collect();
        let body = serde_json::json!({"error": "invalid mission payload", "details": details});
        let (_, rows) = split_error_lines(error_body_message(&body).as_deref());
        assert_eq!(rows.len(), MAX_ERROR_DETAILS + 1);
        assert_eq!(rows[MAX_ERROR_DETAILS], "… and 4 more");
    }

    // `field_tools` puts a partial mortar SOLUTION in `details`. That is a payload for the caller
    // to render, not prose, and folding it into the message would be gibberish.
    #[test]
    fn non_string_details_are_left_alone() {
        let body = serde_json::json!({
            "error": "target out of range",
            "details": {"range": 812.0, "charge": 3},
        });
        assert_eq!(
            error_body_message(&body).as_deref(),
            Some("target out of range")
        );
        assert!(error_body_message(&serde_json::json!({"detail": "x"})).is_none());
    }

    /// T-446 Class-R — multipart upload helper must stay in the wasm client (source pin; the
    /// function is cfg(wasm32) so native tests cannot call it).
    ///
    /// **Cure 2 (T-601).** There is no runtime signature here that a native harness could observe
    /// honestly: the whole assertion is about `web_sys::FormData` and a `gloo_net` builder, so
    /// cure 1 would mean writing a fake `FormData` and then asserting against the fake. What is
    /// left is a source-shape invariant — "the multipart field is named `file`, matching
    /// `POST /cms/uploads`" — so it gets the scrubber, scoped to the one function, instead of a
    /// whole-file `contains` that any dead copy anywhere in the file could satisfy.
    #[test]
    fn api_upload_file_posts_multipart_file_field() {
        let f = item("pub async fn api_upload_file<");
        assert!(
            f.contains("FormData::new()") && f.contains("append_with_blob_and_filename("),
            "upload must build FormData (perturbation: rename/delete api_upload_file, or post JSON)"
        );
        // The field NAME is the contract with the handler, and it is a string literal — so this
        // one assertion reads `live_source` (literals kept) rather than `live_code`.
        assert!(
            item_src("pub async fn api_upload_file<")
                .contains("append_with_blob_and_filename(\"file\""),
            "the multipart field must be named \"file\" to match POST /cms/uploads"
        );
        assert!(
            f.contains("Request::post(&url)") && f.contains(".body(form)"),
            "upload must POST FormData without forcing a JSON Content-Type"
        );
    }

    /* ───────────────────────────── T-591 — the raw-body POST ───────────────────────────── */

    /// A future that is `Pending` on its FIRST poll and ready after.
    ///
    /// Load-bearing for [`two_concurrent_401s_share_one_refresh`]: `block_on(join(a, b))` polls
    /// `a` to completion before it ever touches `b`, so with an instantly-ready refresh the two
    /// callers are never concurrent, the cell is already cleared when `b` arrives, and a SECOND
    /// refresh is the *correct* answer — the test would pass with the single-flight ripped out.
    /// Parking on the first poll is what forces both callers into the cell at once.
    fn pending_once<T: 'static>(v: T) -> futures::future::LocalBoxFuture<'static, T> {
        let mut polled = false;
        let mut val = Some(v);
        futures::future::poll_fn(move |cx| {
            if polled {
                std::task::Poll::Ready(val.take().expect("polled after Ready"))
            } else {
                polled = true;
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
        })
        .boxed_local()
    }

    /// **The property `api_post_raw` must not be allowed to break.** Refresh tokens are single-use
    /// and rotated (`auth.rs` header), so two requests that 401 together must spend ONE token —
    /// the second presenting the same spent token would 401 and wrongly clear the session.
    ///
    /// Until T-591 nothing tested this: `retries_once_after_refresh` has a single caller, so it is
    /// green whether or not `sf` is consulted at all.
    #[test]
    fn two_concurrent_401s_share_one_refresh() {
        let sends = Rc::new(Cell::new(0));
        let refreshes = Rc::new(Cell::new(0));
        let sf = SingleFlight::<Option<RefreshResponse>>::new();

        // One caller of the retry contract, sharing `sf` with its twin.
        async fn one(
            sf: &SingleFlight<Option<RefreshResponse>>,
            sends: Rc<Cell<u32>>,
            refreshes: Rc<Cell<u32>>,
        ) -> Result<&'static str, ApiErr> {
            send_with_refresh(
                sf,
                move |tok| {
                    let s = sends.clone();
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
                    refreshes.set(refreshes.get() + 1);
                    pending_once(Some(rr("new")))
                },
                |_| {},
            )
            .await
        }

        let (a, b) = block_on(futures::future::join(
            one(&sf, sends.clone(), refreshes.clone()),
            one(&sf, sends.clone(), refreshes.clone()),
        ));

        assert_eq!(a, Ok("ok"));
        assert_eq!(b, Ok("ok"));
        assert_eq!(
            refreshes.get(),
            1,
            "the single-use refresh token must be spent ONCE for two concurrent 401s \
             (perturbation: call refresh() directly instead of sf.run(refresh))"
        );
        assert_eq!(sends.get(), 4, "two originals + two retries");
    }

    /// The other half of single-flight: the cell is cleared once the refresh settles, so a LATER
    /// 401 gets a fresh token rather than replaying a spent one. A cache would be just as green on
    /// the concurrent test above and would resurrect the double-spend it prevents.
    #[test]
    fn the_cell_clears_so_a_later_401_refreshes_again() {
        let refreshes = Rc::new(Cell::new(0));
        let sf = SingleFlight::<Option<RefreshResponse>>::new();
        let once = |sf: &SingleFlight<Option<RefreshResponse>>, refreshes: Rc<Cell<u32>>| {
            block_on(send_with_refresh(
                sf,
                move |tok| {
                    async move {
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
                    refreshes.set(refreshes.get() + 1);
                    pending_once(Some(rr("new")))
                },
                |_| {},
            ))
        };
        let a: Result<&str, ApiErr> = once(&sf, refreshes.clone());
        let b: Result<&str, ApiErr> = once(&sf, refreshes.clone());
        assert_eq!((a, b), (Ok("ok"), Ok("ok")));
        assert_eq!(
            refreshes.get(),
            2,
            "sequential 401s each need their own rotated token — the cell must not cache"
        );
    }

    /// The shipped half of this file with comments, string literals and every construct that
    /// cannot run removed (T-601 — [`crate::arsenal::class_r_scrub`]).
    ///
    /// Before T-601 this was `SRC.split("#[cfg(test)]").next()`: raw text, module docs included.
    /// Every needle these pins look for is discussed in prose somewhere in this file, and every
    /// one of them would have been satisfied by a `// …` line — plus by anything parked in
    /// `if false { … }`, `#[cfg(any())]`, or after a `return;`.
    fn prod() -> String {
        crate::arsenal::class_r_scrub::live_code(include_str!("client.rs"))
    }

    /// Same, but string literals survive — for the handful of assertions where the literal **is**
    /// the contract (a wire field name, a header value, a route).
    fn prod_src() -> String {
        crate::arsenal::class_r_scrub::live_source(include_str!("client.rs"))
    }

    /// The body of the ONE item whose signature is `start`.
    ///
    /// Scoped on purpose. A `prod().contains(…)` check is satisfied by a match ANYWHERE in the
    /// file, so it can report success over code it never looked at — this repo's signature defect.
    ///
    /// T-601 changed two things. It reads the scrubbed source, so a needle in a comment or a dead
    /// block no longer counts; and it takes the **balanced body** of the one match rather than
    /// "everything up to the next doc comment", refusing outright when the signature appears twice.
    /// The old shape would happily have handed back a pristine shadow copy parked in a
    /// never-called `mod` while the real helper was cut — and a shadow copy is not a hypothetical
    /// here, since a second `fn api_post_raw` in an inner module compiles fine.
    fn item(start: &str) -> String {
        crate::arsenal::class_r_scrub::only_item(&prod(), start).to_string()
    }

    /// [`item`], literals kept.
    fn item_src(start: &str) -> String {
        crate::arsenal::class_r_scrub::only_item(&prod_src(), start).to_string()
    }

    /// T-591 — the raw-body POST exists and is the shape T-117 asked for.
    #[test]
    fn api_post_raw_takes_an_already_serialised_string_body() {
        let f = item("pub async fn api_post_raw(");
        assert!(
            f.contains("body: String"),
            "api_post_raw must take an already-serialised String — a Value would reinstate the \
             parse+reserialise pair it exists to remove"
        );
        assert!(
            f.contains("Body::Raw(std::rc::Rc::new(body))"),
            "the body must go in behind an Rc so the 401 retry shares one buffer instead of \
             duplicating the document (perturbation: pass the String by value and clone it)"
        );
        assert!(
            f.contains("Consume::Ignore(())"),
            "the 201 echoes the whole json_payload back (models/mission.rs:128); reading it would \
             be a fourth full copy the caller discards"
        );
    }

    /// **T-591 — the proof that the new POST path cannot double-spend a refresh token.**
    ///
    /// Not "api_post_raw looks fine": a *rule* over the whole wasm client. Every route to
    /// `refresh_via_gloo` goes through `send_with_refresh`, and every `send_with_refresh` is
    /// handed the module `REFRESH_SF` cell — so the three counts are equal. A helper that opened
    /// its own refresh path, or passed a fresh `SingleFlight::new()`, breaks the equality.
    #[test]
    fn every_auth_path_goes_through_the_one_single_flight_cell() {
        let p = prod();
        let cells = p.matches("REFRESH_SF.with(|s| s.clone())").count();
        let guards = p.matches("send_with_refresh(").count();
        let refreshes = p.matches("refresh_via_gloo(store)").count();
        assert!(
            cells >= 2,
            "expected the request + upload auth paths, saw {cells}"
        );
        assert_eq!(
            (cells, guards),
            (cells, cells),
            "every send_with_refresh must be fed by REFRESH_SF: {cells} cells vs {guards} guards"
        );
        assert_eq!(
            refreshes, cells,
            "refresh_via_gloo is reachable ONLY through the single-flight: {refreshes} calls vs \
             {cells} cells (perturbation: add a POST helper that refreshes on its own)"
        );
        assert_eq!(
            p.matches("SingleFlight::new()").count(),
            1,
            "there is exactly ONE cell in the shipped client — a second would be a second token \
             spender"
        );
        let tl = p
            .split("thread_local! {")
            .nth(1)
            .expect("the cell must live in a thread_local")
            .split('}')
            .next()
            .unwrap();
        assert!(
            tl.contains("REFRESH_SF: SingleFlight<Option<RefreshResponse>> = SingleFlight::new()"),
            "that one cell must be the module-level REFRESH_SF (mirrors refresh.ts `inflight`)"
        );
    }

    /// T-591 — `api_post_raw` has no transport of its own: it is a thin wrapper on `request`,
    /// the same function `api_post` uses, which is what makes the test above cover it.
    #[test]
    fn api_post_raw_delegates_to_the_shared_request_helper() {
        let f = item("pub async fn api_post_raw(");
        assert!(
            f.contains("request("),
            "must delegate to the shared request helper"
        );
        for banned in [
            "send_with_refresh",
            "REFRESH_SF",
            "Request::post",
            "RequestBuilder",
        ] {
            assert!(
                !f.contains(banned),
                "api_post_raw must not hand-roll `{banned}` — a second auth path is how a \
                 single-use refresh token gets double-spent"
            );
        }
    }

    /// T-591 — the raw arm must set `Content-Type: application/json` itself.
    ///
    /// `.json()` sets it as a side effect and the raw arm skips `.json()`. `create_version` is a
    /// `Json<CreateVersionInput>` extractor (`handlers/missions.rs:970`), which answers **415**
    /// with no header — every raw upload would fail, and not for a reason the body explains.
    #[test]
    fn the_raw_body_arm_sets_the_json_content_type() {
        // `item_src`, not `item`: the header NAME and VALUE are string literals, and here the
        // literal is the contract with Axum's `Json` extractor rather than a mention of it.
        let f = item_src("async fn request<");
        let raw = f
            .split("Body::Raw(s) =>")
            .nth(1)
            .expect("request() must have a Body::Raw arm")
            .split("Body::None")
            .next()
            .unwrap();
        assert!(
            raw.contains(".header(\"Content-Type\", \"application/json\")"),
            "the Body::Raw arm must set Content-Type or Axum's Json extractor answers 415"
        );
        assert!(
            raw.contains(".body(s.as_str())"),
            "the raw arm must send the buffer as-is — no re-serialise"
        );
    }

    /// T-591 — the raw path must keep surfacing the backend's `details`, which was T-117's whole
    /// point. `api_post_raw` inherits this by sharing `request`, so the pin is on `request`'s own
    /// non-2xx arm: a helper that swallowed a 400's body would turn `create_version`'s exact list
    /// of what is wrong with the payload back into "invalid mission payload".
    #[test]
    fn the_shared_error_arm_still_folds_the_details_array() {
        let f = item("async fn request<");
        let err = f
            .split("} else {")
            .nth(1)
            .expect("request() must have a non-2xx else arm");
        assert!(
            err.contains("super::error_body_message(&v)"),
            "the non-2xx arm must fold `details` via error_body_message (perturbation: read only \
             the `error` string, or drop the body entirely)"
        );
        assert!(
            err.contains("Err((status, msg))"),
            "the caller needs the status too — upload_failure(status, ..) maps 409/413 by it"
        );
    }

    /// **T-601 — the calibration for every source pin in this module.**
    ///
    /// The five pins above are only worth their green if [`prod`] / [`item`] can still say NO.
    /// Each row is an attack on a needle one of them looks for, applied to a synthetic file, and
    /// the scrubbed source must no longer contain it. The last two rows are the interesting ones:
    /// they are shadow-copy attacks, which no amount of dead-code stripping catches — only
    /// refusing ambiguity does.
    #[test]
    fn the_source_pins_reject_every_dead_code_wrapper() {
        use crate::arsenal::class_r_scrub::{live_code, only_item};
        let needle = "Body::Raw(std::rc::Rc::new(body))";
        let attacks: [(&str, String); 12] = [
            (
                "if true == false",
                format!("if true == false {{ {needle}; }}"),
            ),
            ("loop { break; … }", format!("loop {{ break; {needle}; }}")),
            (
                "#[cfg(any())]",
                format!("#[cfg(any())] fn d() {{ {needle}; }}"),
            ),
            ("while false", format!("while false {{ {needle}; }}")),
            ("if !true", format!("if !true {{ {needle}; }}")),
            ("if 1 > 2", format!("if 1 > 2 {{ {needle}; }}")),
            (
                "if std::hint::black_box(false)",
                format!("if std::hint::black_box(false) {{ {needle}; }}"),
            ),
            (
                "const C: bool = false; if C",
                format!("const C: bool = false;\nfn d() {{ if C {{ {needle}; }} }}"),
            ),
            ("return; above", format!("fn d() {{ return; {needle}; }}")),
            (
                "#[cfg(any())] mod shadow",
                format!("#[cfg(any())] mod shadow {{ fn d() {{ {needle}; }} }}"),
            ),
            (
                "match guard",
                format!("match () {{ _ if false => {{ {needle}; }} _ => {{}} }}"),
            ),
            ("comment", format!("// {needle}")),
        ];
        for (label, body) in attacks {
            let forged =
                format!("pub async fn api_post_raw(\n) {{\n    {body}\n}}\n#[cfg(test)]\n");
            assert!(
                !live_code(&forged).contains(needle),
                "{label}: the needle survived scrubbing, so `api_post_raw_takes_an_already_\
                 serialised_string_body` would report a live Rc body over code the build never runs"
            );
        }

        // Shadow copies: a pristine definition at column 0 with the real one moved into a `mod`,
        // and the same trick with no `cfg` anywhere to give it away. Both compile; both feed a
        // whole-file grep the wrong body. Only the ambiguity refusal in `only_body` catches them.
        for (label, forged) in [
            (
                "#[cfg(any())]-free shadow copy in a live mod",
                "pub async fn api_post_raw() { good(); }\n\
                 mod real { pub async fn api_post_raw() { bad(); } }\n#[cfg(test)]\n",
            ),
            (
                "shadow copy nested in an impl",
                "pub async fn api_post_raw() { good(); }\n\
                 impl T { pub async fn api_post_raw() { bad(); } }\n#[cfg(test)]\n",
            ),
        ] {
            let scrubbed = live_code(forged);
            let caught =
                std::panic::catch_unwind(|| only_item(&scrubbed, "pub async fn api_post_raw("))
                    .is_err();
            assert!(
                caught,
                "{label}: two definitions of one helper and the pin picked one without saying so — \
                 a grep cannot tell which body ships, so it must refuse rather than guess"
            );
        }

        // The honest shape still reads as present, or every assertion above pins nothing.
        let live = format!("pub async fn api_post_raw() {{\n    {needle};\n}}\n#[cfg(test)]\n");
        assert!(live_code(&live).contains(needle));
    }
}
