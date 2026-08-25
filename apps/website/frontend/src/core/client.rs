//! HTTP client retry contract — ports the api/client.ts response interceptor.
//!
//! On a 401 the client refreshes the token **once** (through the single-flight cell so concurrent
//! 401s share one refresh) and retries the original request **once** with the rotated access token;
//! any other status, or a retry that is still 401, propagates. The state machine is generic over
//! the transport so it is unit-tested natively (single-retry, no loop); the wasm client wires
//! `send`/`refresh` to gloo-net on top.
//!
//! **T-155 — refresh is serialized twice.** The single-flight cell is a `thread_local!`, so it only
//! ever covered one tab; two tabs each had their own and could present the same single-use token,
//! killing the loser's session. Refresh now runs under a **Web Locks** critical section keyed on
//! [`REFRESH_LOCK_NAME`], which is per-origin and therefore shared by every tab — and inside that
//! section [`refresh_cross_tab`] re-checks before it spends. Layering, outermost first:
//! `send_with_refresh` → per-tab [`SingleFlight`] → cross-tab Web Lock → the one POST.
//!
//! **T-156 — a 401 is not an empty list.** [`ApiFailure`] / [`Fetched`] name the difference between
//! "the backend sent nothing" and "the backend refused to let you look", so a dead session can no
//! longer render as a blank page. See [`ApiFailure`] for why a 401 reaching here is always the
//! session and never the route.

use crate::core::auth::{RefreshResponse, SingleFlight};
use futures::future::{FutureExt, LocalBoxFuture};

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

/* ══════════════ T-156 — a 401 is a named failure, never an empty result ══════════════ */

/// Why a request produced no data.
///
/// **T-156 — the whole point of this enum is that "empty" is not one of its variants.** When the
/// session died the SPA used to render empty lists and blank pages: the Arsenal showed its
/// non-character guard because the catalog was empty, the Factions palette showed the "No factions
/// yet" author CTA. The operator read that as data loss. It is the house signature defect facing
/// the user — the UI reporting "nothing here" over data it was never allowed to read.
///
/// [`ApiErr`] could already carry the status, but it is a bare `(u16, Option<String>)` tuple and
/// the ergonomic way to consume it is `.ok()`, which flattens every failure into `None` and every
/// `None` into an empty render. Naming the states is what makes the 401 survive the trip to the
/// render site.
///
/// **Why a 401 out of this client means the session, not the route.** The backend answers 401 from
/// exactly one place — the auth middleware, for a missing or unparseable bearer token
/// (`middleware/auth.rs:44,54`; the third at `:115` is the game-server service token, a route the
/// SPA never calls). Insufficient role is **403** (`middleware/auth.rs:83`). And by the time an
/// error escapes [`send_with_refresh`] a 401 has already survived a single-flight refresh **and** a
/// retry with the rotated token. So a 401 here is not "this route refused you" — it is "auth did
/// not work even with a freshly minted token", which is a dead session and nothing else.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApiFailure {
    /// The session is over: refresh + retry both failed. Surface "session expired — log in again",
    /// never an empty state.
    SessionExpired {
        /// The backend's `{"error": …}` string when it sent one.
        message: Option<String>,
    },
    /// The backend answered, and refused for a reason that is not authentication (403, 404, 409,
    /// 413, 500…). The caller maps the status; the message is the human-readable cause.
    Http {
        status: u16,
        message: Option<String>,
    },
    /// The request never reached the backend — network down, CORS, or a body that would not
    /// deserialise. Distinct from [`Self::SessionExpired`] because "you are logged out" is a lie
    /// when the truth is "the network dropped".
    Transport,
}

#[allow(dead_code)]
impl ApiFailure {
    /// True only for a terminal 401. The one predicate a "session expired" banner should gate on.
    pub fn is_session_expired(&self) -> bool {
        matches!(self, Self::SessionExpired { .. })
    }

    /// The backend's failure string, if it sent one.
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::SessionExpired { message } | Self::Http { message, .. } => message.as_deref(),
            Self::Transport => None,
        }
    }
}

impl From<ApiErr> for ApiFailure {
    fn from((status, message): ApiErr) -> Self {
        match status {
            // The client's own sentinel for "never got an answer" (the `Err((0u16, None))` arms in
            // the wasm transport below).
            0 => Self::Transport,
            401 => Self::SessionExpired { message },
            _ => Self::Http { status, message },
        }
    }
}

/// A settled fetch: the data, or a **named** reason there is none.
///
/// **T-156 — this type exists to make the wrong thing awkward to express.** It deliberately has no
/// `Default`, no `unwrap_or_default`, no `unwrap_or_else`, no `ok()` and no `Deref`: every one of
/// those is a one-token way to turn "the server refused to show you your data" into "you have no
/// data", which is the bug. [`Fetched::view`] is the intended reader precisely because it is
/// *total* — it cannot be called without supplying the failure arm, so a render site physically
/// cannot forget the case. `the_fetched_type_offers_no_collapse_to_empty` is the pin that stops the
/// footguns growing back.
///
/// Migration is one token at the call site: `.await.ok()` becomes `.await.into()`.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fetched<T> {
    Data(T),
    Failed(ApiFailure),
}

#[allow(dead_code)]
impl<T> Fetched<T> {
    /// Fold both cases into one value. **Total by construction** — the reason this, and not an
    /// accessor, is the intended reader: there is no arm to leave out.
    pub fn view<R>(
        &self,
        on_data: impl FnOnce(&T) -> R,
        on_failure: impl FnOnce(&ApiFailure) -> R,
    ) -> R {
        match self {
            Self::Data(t) => on_data(t),
            Self::Failed(f) => on_failure(f),
        }
    }

    /// The data, when there is some. Borrowed on purpose: an owned `Option<T>` is one
    /// `.unwrap_or_default()` away from the empty-list bug this type exists to prevent.
    pub fn data(&self) -> Option<&T> {
        match self {
            Self::Data(t) => Some(t),
            Self::Failed(_) => None,
        }
    }

    /// The failure, when there is one.
    pub fn failure(&self) -> Option<&ApiFailure> {
        match self {
            Self::Data(_) => None,
            Self::Failed(f) => Some(f),
        }
    }

    /// True only when the session died — the banner predicate. **Never true for a legitimately
    /// empty `Data`**, which is the whole distinction T-156 is about.
    pub fn is_session_expired(&self) -> bool {
        matches!(self, Self::Failed(f) if f.is_session_expired())
    }
}

impl<T> From<Result<T, ApiErr>> for Fetched<T> {
    fn from(r: Result<T, ApiErr>) -> Self {
        match r {
            Ok(t) => Self::Data(t),
            Err(e) => Self::Failed(e.into()),
        }
    }
}

/* ══════════════ T-155 — the cross-tab refresh mutex ══════════════ */

/// The Web Locks name the refresh critical section is taken under. Web Locks are scoped per
/// **origin**, so every same-origin tab of the SPA contends on this one string.
#[allow(dead_code)]
pub const REFRESH_LOCK_NAME: &str = "tbd-auth-refresh";

/// Does a peer tab's broadcast rotation supersede the token this tab was about to spend?
///
/// The winner broadcasts the pair it just minted. A rotation always changes the refresh token, so
/// "the broadcast carries a refresh token that is not the one I hold" is exactly "a peer rotated
/// after I read mine" — and the negation matters just as much: a tab that already adopted a pair
/// must **not** treat that same pair as a reason to skip its own refresh, or its access token could
/// never be renewed again.
#[allow(dead_code)]
pub fn peer_rotation_supersedes(peer: &RefreshResponse, about_to_spend: Option<&str>) -> bool {
    Some(peer.refresh_token.as_str()) != about_to_spend
}

/// One refresh attempt, serialized **across tabs**.
///
/// **T-155 — the bug this closes.** Refresh tokens are single-use: `/auth/refresh` rotates and
/// revokes. [`SingleFlight`] stops one tab double-spending, but it lives in a `thread_local!` —
/// two tabs of the SPA have two of them, so both can present the same token and the loser gets a
/// 401 that cascades into a dead session for a tab that did nothing wrong (the 2026-07-13
/// incident: registry+compat 401 → refresh 401 → every subsequent request 401).
///
/// The cure is a lock that spans tabs, and then a **double-check inside it**. Holding the lock is
/// not enough on its own: a tab that waits, wins the lock, and then spends the token it read
/// *before* waiting has merely made the double-spend orderly. So under the lock this does, in
/// order:
///
/// 1. **Adopt.** If a peer broadcast a rotation that [`peer_rotation_supersedes`] the token we were
///    about to spend, take its pair and return — **no POST at all**. N tabs, one rotation.
/// 2. **Re-read.** Otherwise take the freshest refresh token from shared storage rather than the
///    stale one held in this tab's signal, which is the copy that goes out of date the moment a
///    peer rotates. `stored()` falling back to `entry_token` covers the storage read failing (the
///    blob is unreadable in some private-mode configurations).
/// 3. **Spend** exactly that token.
///
/// **What a lost race costs — this comment used to understate it.** It said spending a stale token
/// "costs one 401 in this tab, which is strictly better than refusing to refresh at all". The
/// second half is still true; the first half is not. `handlers/auth.rs:149-152` treats presentation
/// of an already-revoked token as **reuse** and calls `revoke_token_family`, which is
/// `UPDATE refresh_tokens SET revoked_at = now() WHERE discord_id = $1 AND revoked_at IS NULL` —
/// every live token for that user, including the pair the *winner* just minted. So the price of a
/// lost race is not one 401 in one tab; it is **every tab logged out, the winner's included**.
/// That is the 2026-07-13 incident class, which is what this function exists to prevent.
///
/// The window is real but narrow, and it takes **both** mitigations missing to reach it. The winner
/// broadcasts before releasing the lock (`refresh_via_gloo`), but it persists to `localStorage`
/// only *after* the lock releases — `on_refreshed` in [`send_with_refresh`] runs once `sf.run`
/// returns. BroadcastChannel delivery is not ordered against Web Locks grant by any spec, so a
/// waiter can be handed the lock, find no peer pair parked yet (step 1 misses), read a
/// `localStorage` the winner has not written yet (step 2 misses), and spend the revoked token.
/// T-155 nonetheless turned a **guaranteed** double-spend into a **rare two-lost-races** event,
/// which is a large genuine improvement — the residual is this narrow interleaving, not the old bug.
///
/// **The native suite below cannot catch it.** The harness at the bottom of this file is
/// single-threaded and deterministic: `block_on(join(a, b))` plus the `pending_once` park means the
/// winner's post-await persist into the shared `storage` cell always completes before the waiter is
/// resumed. The losing interleaving is therefore *unreachable* by the very tests that model the
/// race. Read a green run as "the policy is right", not as "the residual window is closed".
///
/// Generic over the lock, so the policy — which is all of the correctness — is unit-tested
/// natively against a real fair mutex. `with_lock` is `FnOnce(body) -> future`: it must run `body`
/// with the lock held and release when `body` settles, **including when it settles with `None`**.
/// A lock released only on success is a lock that a failed refresh wedges forever.
#[allow(dead_code)]
pub async fn refresh_cross_tab<L, LFut>(
    entry_token: Option<String>,
    with_lock: L,
    peer_pair: impl FnOnce(Option<&str>) -> Option<RefreshResponse> + 'static,
    stored: impl FnOnce() -> Option<String> + 'static,
    post: impl FnOnce(Option<String>) -> LocalBoxFuture<'static, Option<RefreshResponse>> + 'static,
) -> Option<RefreshResponse>
where
    L: FnOnce(LocalBoxFuture<'static, Option<RefreshResponse>>) -> LFut,
    LFut: std::future::Future<Output = Option<RefreshResponse>>,
{
    with_lock(
        async move {
            // ── critical section: at most one tab of this origin is in here ──
            if let Some(adopted) = peer_pair(entry_token.as_deref()) {
                return Some(adopted); // a peer already rotated — spend nothing
            }
            post(stored().or(entry_token)).await
        }
        .boxed_local(),
    )
    .await
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
    use crate::core::auth::{
        load_persisted, persist, AuthStore, RefreshResponse, Session, SingleFlight,
    };
    use crate::core::dto::MeResponse;
    use futures::future::FutureExt;
    use leptos::prelude::*;
    use serde::de::DeserializeOwned;
    // T-155 — the Web Locks + BroadcastChannel bindings reach both objects through `js_sys::Reflect`
    // (see `lock_manager`), and every hop needs the JsValue casts.
    use wasm_bindgen::JsCast;

    const API_BASE: &str = "/api/v1";

    thread_local! {
        // Module-level single-flight cell — mirrors refresh.ts's `inflight`.
        static REFRESH_SF: SingleFlight<Option<RefreshResponse>> = SingleFlight::new();
    }

    /// The one POST that spends the single-use refresh token.
    ///
    /// **T-155 — the token is now a parameter.** It used to read `store.refresh_token` itself, and
    /// that read happened before this tab had any right to spend: a tab that queued behind a peer
    /// would wake up holding a token the peer had already revoked. The freshest token is chosen
    /// inside the critical section by [`super::refresh_cross_tab`] and handed in, so the value
    /// spent is the value that was current at the moment of spending.
    async fn refresh_via_gloo(store: AuthStore, token: Option<String>) -> Option<RefreshResponse> {
        let _ = store;
        let body = serde_json::json!({ "refresh_token": token });
        let req = gloo_net::http::Request::post(&format!("{API_BASE}/auth/refresh"))
            .credentials(web_sys::RequestCredentials::Include)
            .json(&body)
            .ok()?;
        match req.send().await {
            Ok(resp) if (200..300).contains(&resp.status()) => {
                let rotated = resp.json::<RefreshResponse>().await.ok()?;
                // Tell the other tabs before releasing the lock, so a waiter finds the pair
                // already parked and adopts it instead of spending a second rotation.
                broadcast_rotation(&rotated);
                Some(rotated)
            }
            _ => None,
        }
    }

    /* ─────────────────── T-155 — Web Locks + the peer-rotation channel ─────────────────── */

    thread_local! {
        /// The last rotation a peer tab announced. **Memory only, never localStorage** — it carries
        /// an access token, and T-126 S5 is that the access token is never persisted (`auth.rs`
        /// `PersistState` has no `accessToken` field, and `persist_blob_shape_matches_tbd_auth`
        /// pins its absence). A BroadcastChannel message lives in the receiving page's heap and
        /// dies with it, which is the property that makes carrying the whole pair safe here and
        /// unsafe in storage.
        static PEER_ROTATION: std::rc::Rc<std::cell::RefCell<Option<RefreshResponse>>> =
            std::rc::Rc::new(std::cell::RefCell::new(None));
        /// The channel object, kept alive so its `message` listener keeps firing. `None` until the
        /// first [`subscribe_peer_rotations`]; stays `None` where BroadcastChannel is unavailable.
        static PEER_CHANNEL: std::cell::RefCell<Option<js_sys::Object>> =
            const { std::cell::RefCell::new(None) };
    }

    /// `window.navigator.locks`, or `None` where the Web Locks API is not reachable.
    ///
    /// Reached through `js_sys::Reflect` rather than `web_sys::LockManager` deliberately: web-sys
    /// keeps the Web Locks bindings behind `--cfg=web_sys_unstable_apis`, which is a **workspace
    /// RUSTFLAGS change** — a build-wide switch to unstable web-sys, for one call. Reflect costs
    /// this crate no new dependency (`js-sys` is already in `Cargo.toml`) and no build flag.
    ///
    /// `None` happens for real: Web Locks require a **secure context**, so a staging build served
    /// over plain http on a bare IP has no `navigator.locks`. [`with_refresh_lock`] degrades to the
    /// pre-T-155 behaviour there rather than failing the refresh — see its docs.
    fn lock_manager() -> Option<js_sys::Object> {
        let nav = web_sys::window()?.navigator();
        let locks = js_sys::Reflect::get(nav.as_ref(), &"locks".into()).ok()?;
        (!locks.is_undefined() && !locks.is_null()).then(|| locks.unchecked_into())
    }

    /// Run `body` holding the cross-tab refresh lock; release when it settles.
    ///
    /// `navigator.locks.request(name, cb)` holds the lock for exactly as long as the promise `cb`
    /// returns is pending, and the browser releases it when the promise settles **or when the page
    /// holding it goes away**. That last clause is why this is Web Locks and not a `localStorage`
    /// flag: a tab that crashes mid-refresh drops the lock immediately, whereas a storage flag
    /// would sit there and wedge every other tab until something expired it. There is no stale-lock
    /// state to recover from here, and so no expiry to tune.
    ///
    /// The body's value comes back through a cell rather than the promise because it is a Rust
    /// `RefreshResponse`, not a `JsValue`; the promise resolves with `undefined` purely as the
    /// signal to release.
    ///
    /// **No Web Locks → run unlocked.** Refusing to refresh would log every user of an insecure
    /// context out on schedule, which is worse than the race this closes; the per-tab
    /// [`SingleFlight`] still holds, so the fallback is exactly the pre-T-155 behaviour and no
    /// worse. The re-read step in [`super::refresh_cross_tab`] still runs, so even unlocked a tab
    /// spends the freshest token it can see rather than its own stale one.
    async fn with_refresh_lock(
        body: super::LocalBoxFuture<'static, Option<RefreshResponse>>,
    ) -> Option<RefreshResponse> {
        let Some(locks) = lock_manager() else {
            return body.await;
        };
        let Ok(request) = js_sys::Reflect::get(&locks, &"request".into()) else {
            return body.await;
        };
        let Ok(request) = request.dyn_into::<js_sys::Function>() else {
            return body.await;
        };

        let out = std::rc::Rc::new(std::cell::RefCell::new(None::<RefreshResponse>));
        let sink = out.clone();
        // The body is parked in a cell rather than moved straight into the callback so that, if the
        // callback never runs, it is still here to run **unlocked**. `request()` can reject rather
        // than invoke — a document that is not fully active, an opaque origin — and a refresh that
        // silently never happens is a 401, which the user experiences as being logged out. Losing
        // the mutex is a race; losing the refresh is the very bug T-155 exists to stop.
        let pending = std::rc::Rc::new(std::cell::RefCell::new(Some(body)));
        let deferred = pending.clone();
        // `once_into_js`: Web Locks invokes the callback exactly once, and this hands ownership to
        // JS so there is no `Closure` to keep alive across the await.
        let cb = wasm_bindgen::closure::Closure::once_into_js(
            move |_lock: wasm_bindgen::JsValue| -> wasm_bindgen::JsValue {
                let body = deferred.borrow_mut().take();
                wasm_bindgen_futures::future_to_promise(async move {
                    if let Some(body) = body {
                        *sink.borrow_mut() = body.await;
                    }
                    Ok(wasm_bindgen::JsValue::UNDEFINED)
                })
                .into()
            },
        );

        if let Ok(p) = request.call2(&locks, &super::REFRESH_LOCK_NAME.into(), &cb) {
            // Await the OUTER promise: it settles after the lock is released. Its own value is
            // `undefined` — the result travels through `out`.
            let promise: js_sys::Promise = p.unchecked_into();
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        }
        // Borrow, then drop the guard, then await: holding a `RefCell` borrow across an await point
        // is how a re-entrant refresh would panic on an already-borrowed cell.
        let never_ran = pending.borrow_mut().take();
        if let Some(body) = never_ran {
            return body.await;
        }
        let v = out.borrow_mut().take();
        v
    }

    /// The BroadcastChannel every tab of this origin announces rotations on, constructed on first
    /// use. Reflect again, for the same reason as [`lock_manager`]: `web_sys::BroadcastChannel` is
    /// not in this crate's web-sys feature list, and adding it is a `Cargo.toml` change for one
    /// object. Returns `None` where BroadcastChannel is unavailable — the adopt step then simply
    /// never fires and every waiter falls through to the re-read, which is still correct.
    fn peer_channel() -> Option<js_sys::Object> {
        subscribe_peer_rotations();
        PEER_CHANNEL.with(|c| c.borrow().clone())
    }

    /// Open the channel and start recording peer rotations. Idempotent — the second call sees the
    /// cached object and returns, so it is safe to call on every refresh as well as at bootstrap.
    fn subscribe_peer_rotations() {
        if PEER_CHANNEL.with(|c| c.borrow().is_some()) {
            return;
        }
        let Some(win) = web_sys::window() else { return };
        let Ok(ctor) = js_sys::Reflect::get(win.as_ref(), &"BroadcastChannel".into()) else {
            return;
        };
        let Ok(ctor) = ctor.dyn_into::<js_sys::Function>() else {
            return;
        };
        let args = js_sys::Array::of1(&super::REFRESH_LOCK_NAME.into());
        let Ok(chan) = js_sys::Reflect::construct(&ctor, &args) else {
            return;
        };
        let slot = PEER_ROTATION.with(std::clone::Clone::clone);
        let on_message = wasm_bindgen::closure::Closure::<dyn FnMut(wasm_bindgen::JsValue)>::new(
            move |ev: wasm_bindgen::JsValue| {
                let Ok(data) = js_sys::Reflect::get(&ev, &"data".into()) else {
                    return;
                };
                // Serde over the structured clone: the pair crosses as a plain JSON string, so a
                // message from a future build with extra fields is simply ignored rather than
                // half-read.
                if let Some(text) = data.as_string() {
                    if let Ok(pair) = serde_json::from_str::<RefreshResponse>(&text) {
                        *slot.borrow_mut() = Some(pair);
                    }
                }
            },
        );
        let target: web_sys::EventTarget = chan.clone().unchecked_into();
        if target
            .add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref())
            .is_ok()
        {
            // The listener outlives this call and must not be dropped, so leak it deliberately:
            // one closure per page, alive for as long as the channel is.
            on_message.forget();
            PEER_CHANNEL.with(|c| *c.borrow_mut() = Some(chan.unchecked_into()));
        }
    }

    /// Announce a rotation this tab just performed, so waiting peers adopt instead of spending a
    /// second one. Best effort by design — a dropped announcement costs one extra (valid) rotation,
    /// never a spent token.
    fn broadcast_rotation(pair: &RefreshResponse) {
        let Some(chan) = peer_channel() else { return };
        let Ok(post) = js_sys::Reflect::get(&chan, &"postMessage".into()) else {
            return;
        };
        let Ok(post) = post.dyn_into::<js_sys::Function>() else {
            return;
        };
        if let Ok(text) = serde_json::to_string(pair) {
            let _ = post.call1(&chan, &text.into());
        }
    }

    /// The wasm binding of [`super::refresh_cross_tab`]: Web Locks for the critical section, the
    /// BroadcastChannel for the adopt step, `tbd-auth` for the re-read.
    ///
    /// The `stored` re-read goes to **localStorage, not the store's signal**. The signal is this
    /// tab's private copy and goes stale the instant a peer rotates; `tbd-auth` is the shared copy
    /// every tab writes on every rotation, so it is the only honest answer to "what is the current
    /// refresh token".
    async fn refresh_locked(store: AuthStore) -> Option<RefreshResponse> {
        subscribe_peer_rotations();
        super::refresh_cross_tab(
            store.refresh_token.get_untracked(),
            with_refresh_lock,
            |about_to_spend| {
                PEER_ROTATION
                    .with(|p| p.borrow().clone())
                    .filter(|pair| super::peer_rotation_supersedes(pair, about_to_spend))
            },
            || load_persisted().and_then(|p| p.refresh_token),
            move |token| refresh_via_gloo(store, token).boxed_local(),
        )
        .await
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
            move || refresh_locked(store).boxed_local(),
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
            move || refresh_locked(store).boxed_local(),
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
    use std::cell::{Cell, RefCell};
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
    /// cannot run removed (T-601 — [`crate::editor::arsenal::class_r_scrub`]).
    ///
    /// Before T-601 this was `SRC.split("#[cfg(test)]").next()`: raw text, module docs included.
    /// Every needle these pins look for is discussed in prose somewhere in this file, and every
    /// one of them would have been satisfied by a `// …` line — plus by anything parked in
    /// `if false { … }`, `#[cfg(any())]`, or after a `return;`.
    fn prod() -> String {
        crate::editor::arsenal::class_r_scrub::live_code(include_str!("client.rs"))
    }

    /// Same, but string literals survive — for the handful of assertions where the literal **is**
    /// the contract (a wire field name, a header value, a route).
    fn prod_src() -> String {
        crate::editor::arsenal::class_r_scrub::live_source(include_str!("client.rs"))
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
        crate::editor::arsenal::class_r_scrub::only_item(&prod(), start).to_string()
    }

    /// [`item`], literals kept.
    fn item_src(start: &str) -> String {
        crate::editor::arsenal::class_r_scrub::only_item(&prod_src(), start).to_string()
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
    /// Not "api_post_raw looks fine": a *rule* over the whole wasm client. Every route to the
    /// refresh POST goes through `send_with_refresh`, and every `send_with_refresh` is handed the
    /// module `REFRESH_SF` cell — so the three counts are equal. A helper that opened its own
    /// refresh path, or passed a fresh `SingleFlight::new()`, breaks the equality.
    ///
    /// **T-155 renamed the counted needle.** The auth paths now reach `refresh_locked`, not
    /// `refresh_via_gloo` — the cross-tab lock sits between them, and
    /// [`the_refresh_post_is_reachable_only_from_inside_the_cross_tab_lock`] is what pins that the
    /// hop is real rather than a rename.
    #[test]
    fn every_auth_path_goes_through_the_one_single_flight_cell() {
        let p = prod();
        let cells = p.matches("REFRESH_SF.with(|s| s.clone())").count();
        let guards = p.matches("send_with_refresh(").count();
        let refreshes = p.matches("refresh_locked(store)").count();
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
            "the refresh path is reachable ONLY through the single-flight: {refreshes} calls vs \
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

    /* ═════════════ T-155 — the cross-tab refresh mutex (policy, native) ═════════════ */

    /// `/auth/refresh` as it actually behaves: **single-use and rotating**.
    ///
    /// This is the piece that makes the T-155 tests able to fail. A fake that always returns a
    /// fresh pair is green whether or not the tabs coordinate — it never models the one rule the
    /// bug is about. Presenting a token that is not the live one is a **spent** token, and this
    /// answers `None` for it, which is the 401 that killed the 2026-07-13 session.
    struct FakeAuthServer {
        live: RefCell<String>,
        posts: Cell<u32>,
        seq: Cell<u32>,
    }

    impl FakeAuthServer {
        fn new(initial: &str) -> Self {
            Self {
                live: RefCell::new(initial.to_string()),
                posts: Cell::new(0),
                seq: Cell::new(0),
            }
        }

        fn refresh(&self, presented: Option<String>) -> Option<RefreshResponse> {
            self.posts.set(self.posts.get() + 1);
            let presented = presented?;
            if presented != *self.live.borrow() {
                return None; // already rotated away — the double-spend 401
            }
            self.seq.set(self.seq.get() + 1);
            let n = self.seq.get();
            let next = format!("r{n}");
            *self.live.borrow_mut() = next.clone();
            Some(RefreshResponse {
                access_token: format!("a{n}"),
                refresh_token: next,
                expires_at: format!("e{n}"),
            })
        }
    }

    /// Everything one simulated tab needs: the shared server, the shared `tbd-auth` blob, the
    /// shared cross-tab lock, the shared broadcast slot, and a trace of lock events.
    #[derive(Clone)]
    struct Origin {
        server: Rc<FakeAuthServer>,
        storage: Rc<RefCell<Option<String>>>,
        lock: Rc<futures::lock::Mutex<()>>,
        peer: Rc<RefCell<Option<RefreshResponse>>>,
        trace: Rc<RefCell<Vec<String>>>,
    }

    impl Origin {
        fn new(initial: &str) -> Self {
            Self {
                server: Rc::new(FakeAuthServer::new(initial)),
                storage: Rc::new(RefCell::new(Some(initial.to_string()))),
                lock: Rc::new(futures::lock::Mutex::new(())),
                peer: Rc::new(RefCell::new(None)),
                trace: Rc::new(RefCell::new(Vec::new())),
            }
        }

        /// One tab's refresh. `entry` is the tab's own in-memory copy of the refresh token — the
        /// stale one, in the race. `broadcast` mirrors the real client announcing its rotation.
        async fn tab(
            &self,
            who: &'static str,
            entry: &str,
            broadcast: bool,
        ) -> Option<RefreshResponse> {
            let (lock, trace) = (self.lock.clone(), self.trace.clone());
            let (server, storage, peer) =
                (self.server.clone(), self.storage.clone(), self.peer.clone());
            let (out_storage, out_peer) = (storage.clone(), peer.clone());
            let out = super::refresh_cross_tab(
                Some(entry.to_string()),
                move |body| async move {
                    trace.borrow_mut().push(format!("{who}:want"));
                    let held = lock.lock().await;
                    trace.borrow_mut().push(format!("{who}:hold"));
                    let out = body.await;
                    drop(held);
                    trace.borrow_mut().push(format!("{who}:free"));
                    out
                },
                move |about_to_spend| {
                    peer.borrow()
                        .clone()
                        .filter(|p| super::peer_rotation_supersedes(p, about_to_spend))
                },
                move || storage.borrow().clone(),
                move |token| {
                    async move {
                        // Park before answering: without a real await point inside the critical
                        // section `block_on(join(a, b))` runs A to completion before it ever polls
                        // B, the tabs are never concurrent, and a no-lock build passes.
                        pending_once(()).await;
                        server.refresh(token)
                    }
                    .boxed_local()
                },
            )
            .await;
            // What the real client does on a successful rotation: persist the refresh token
            // (`persist`) and, since T-155, tell the peers (`broadcast_rotation`).
            if let Some(r) = &out {
                *out_storage.borrow_mut() = Some(r.refresh_token.clone());
                if broadcast {
                    *out_peer.borrow_mut() = Some(r.clone());
                }
            }
            out
        }
    }

    /// **The T-155 incident, reproduced and cured.** Two tabs hold the same refresh token in their
    /// own signals and both 401 at once. Before the cross-tab lock they both POSTed it; the loser
    /// got a 401 and its session died while it was doing nothing wrong.
    ///
    /// Both must come out with a live session. Note what is *not* asserted: that only one rotation
    /// happens. Without a peer broadcast the honest outcome is two rotations — the waiter re-reads
    /// the rotated token and spends **that**, which the server accepts. One rotation is the job of
    /// the broadcast, tested next.
    #[test]
    fn two_tabs_racing_one_single_use_token_both_keep_their_session() {
        let o = Origin::new("r0");
        let (a, b) = block_on(futures::future::join(
            o.tab("A", "r0", false),
            o.tab("B", "r0", false),
        ));
        assert!(a.is_some(), "tab A must get a rotation");
        assert!(
            b.is_some(),
            "tab B was silently logged out — it re-presented a token tab A had already spent \
             (perturbation: spend `entry_token` instead of re-reading `stored()` under the lock)"
        );
        assert_ne!(
            a.as_ref().map(|r| &r.refresh_token),
            b.as_ref().map(|r| &r.refresh_token),
            "each tab must end on its own rotation, not a shared stale one"
        );
        assert_eq!(o.server.posts.get(), 2, "one POST per tab, both accepted");
    }

    /// The lock is **held across the whole refresh** and released after — not taken and dropped
    /// before the POST, which would serialize nothing.
    #[test]
    fn the_lock_is_held_across_the_post_and_released_after() {
        let o = Origin::new("r0");
        block_on(futures::future::join(
            o.tab("A", "r0", false),
            o.tab("B", "r0", false),
        ));
        let t = o.trace.borrow().clone();
        let idx = |needle: &str| t.iter().position(|e| e == needle).expect(needle);
        // Whoever holds first must free before the other can hold: strict interleaving = mutual
        // exclusion. Either order is fine; overlap is not.
        let (first, second) = if idx("A:hold") < idx("B:hold") {
            ("A", "B")
        } else {
            ("B", "A")
        };
        assert!(
            idx(&format!("{first}:free")) < idx(&format!("{second}:hold")),
            "the second tab entered the critical section before the first left it: {t:?}"
        );
        assert!(
            t.contains(&"A:free".to_string()) && t.contains(&"B:free".to_string()),
            "both tabs must release the lock: {t:?}"
        );
    }

    /// **The waiter adopts rather than spends.** When the winner announces its rotation, the tab
    /// behind it takes that pair and never POSTs — N tabs, one rotation, which is what the ticket
    /// asks for.
    #[test]
    fn the_waiter_adopts_the_peer_rotation_instead_of_spending_a_second_one() {
        let o = Origin::new("r0");
        let (a, b) = block_on(futures::future::join(
            o.tab("A", "r0", true),
            o.tab("B", "r0", true),
        ));
        assert!(a.is_some() && b.is_some(), "both tabs keep their session");
        assert_eq!(
            a.as_ref().map(|r| &r.access_token),
            b.as_ref().map(|r| &r.access_token),
            "the waiter must adopt the winner's pair, access token included"
        );
        assert_eq!(
            o.server.posts.get(),
            1,
            "two tabs, ONE rotation (perturbation: drop the peer_pair adopt step and this becomes 2)"
        );
    }

    /// The adopt step must not fire on the tab's **own** pair, or a tab that already adopted could
    /// never renew its access token again — it would adopt the same expired pair forever.
    #[test]
    fn a_tab_does_not_adopt_the_pair_it_is_already_holding() {
        let held = rr("new"); // refresh_token == "r"
        assert!(
            !peer_rotation_supersedes(&held, Some("r")),
            "the pair this tab already holds is not a peer rotation"
        );
        assert!(
            peer_rotation_supersedes(&held, Some("r-older")),
            "a pair carrying a different refresh token IS a peer rotation"
        );
        assert!(
            peer_rotation_supersedes(&held, None),
            "a tab with no token of its own has nothing that could match"
        );

        // …and end to end: a broadcast that is already this tab's token must not short-circuit
        // the refresh. Tab A rotates r0→r1 and announces it; tab B has already adopted r1 and now
        // needs its own renewal.
        let o = Origin::new("r0");
        block_on(o.tab("A", "r0", true));
        let before = o.server.posts.get();
        let b = block_on(o.tab("B", "r1", true));
        assert!(b.is_some(), "the second tab must still be able to refresh");
        assert_eq!(
            o.server.posts.get(),
            before + 1,
            "holding the announced pair must NOT suppress a later, genuine refresh"
        );
    }

    /// A failed refresh must still release the lock. A lock released only on success is one that
    /// a single network blip wedges for every tab, permanently.
    #[test]
    fn a_failed_refresh_still_releases_the_lock() {
        let o = Origin::new("r0");
        // A session that really is over: this tab's own copy AND the shared blob are both a token
        // the server has rotated away. (Killing only the tab's copy proves nothing — the re-read
        // would find the live one in storage and the refresh would succeed, which is the whole
        // point of `two_tabs_racing_one_single_use_token_both_keep_their_session`.)
        *o.storage.borrow_mut() = Some("rX".into());
        let a = block_on(o.tab("A", "rX", false));
        assert!(a.is_none(), "the fake server must refuse a dead token");
        assert!(
            o.trace.borrow().contains(&"A:free".to_string()),
            "the lock was not released after a failed refresh: {:?}",
            o.trace.borrow()
        );
        // The proof that "released" is real and not just a trace line: the next tab gets through.
        *o.storage.borrow_mut() = Some("r0".into());
        assert!(
            block_on(o.tab("B", "r0", false)).is_some(),
            "a later tab must be able to take the lock the failed one held"
        );
    }

    /* ═════════════ T-155 — the wasm binding (source pins) ═════════════ */

    /// **The POST is inside the lock.** `refresh_locked` is the only caller of `refresh_via_gloo`,
    /// and it reaches it through `with_refresh_lock` + `refresh_cross_tab`. A helper that called
    /// the POST directly would be back to the per-tab-only serialization T-155 exists to fix, and
    /// `every_auth_path_goes_through_the_one_single_flight_cell` alone would not notice — its
    /// needle is `refresh_locked`, which a rename satisfies.
    #[test]
    fn the_refresh_post_is_reachable_only_from_inside_the_cross_tab_lock() {
        let p = prod();
        assert_eq!(
            p.matches("refresh_via_gloo(store,").count(),
            1,
            "the refresh POST must have exactly ONE caller — a second is a second token spender"
        );
        let locked = item("async fn refresh_locked(");
        for needed in [
            "refresh_cross_tab(",
            "with_refresh_lock",
            "refresh_via_gloo(store, token)",
            "peer_rotation_supersedes(",
            "load_persisted()",
        ] {
            assert!(
                locked.contains(needed),
                "refresh_locked must go through `{needed}` (perturbation: POST straight from the \
                 single-flight and the cross-tab race is back)"
            );
        }
    }

    /// **The lock is Web Locks, not a `localStorage` flag.** The distinction is the whole reason
    /// this primitive was chosen: the browser drops a Web Lock when the page holding it dies, so a
    /// tab that crashes mid-refresh wedges nobody. A storage flag would sit there until some
    /// expiry ran, and every other tab would be stuck behind a holder that no longer exists.
    #[test]
    fn the_cross_tab_lock_uses_web_locks_so_a_dead_tab_releases_it() {
        let src = item_src("fn lock_manager(");
        assert!(
            src.contains("\"locks\""),
            "the lock manager must be `navigator.locks` — the API that releases on page death"
        );
        let held = item("async fn with_refresh_lock(");
        assert!(
            held.contains("request.call2(") && held.contains("REFRESH_LOCK_NAME"),
            "the critical section must be `navigator.locks.request(REFRESH_LOCK_NAME, cb)`"
        );
        assert!(
            held.contains("future_to_promise"),
            "the lock is held for as long as the callback's promise is pending — the body must be \
             handed back as a promise, or the lock releases before the POST runs"
        );
        // No hand-rolled expiring flag anywhere in the shipped client: the reason Web Locks was
        // chosen is that there is no stale-lock state to expire.
        for banned in [
            "lock_expires",
            "lock_deadline",
            "LOCK_TTL",
            "lock_acquired_at",
        ] {
            assert!(
                !prod().contains(banned),
                "`{banned}` suggests a storage-flag lock with an expiry — Web Locks needs none"
            );
        }
    }

    /// **The peer channel never persists the access token.** The rotated pair crosses tabs in
    /// memory (BroadcastChannel) precisely because it carries an access token, and T-126 S5 is
    /// that the access token is never written to storage — `auth.rs`'s
    /// `persist_blob_shape_matches_tbd_auth` pins its absence from `tbd-auth`. A "simplification"
    /// that routed the pair through `localStorage` would put it on disk for every tab and every
    /// later visitor.
    #[test]
    fn the_peer_rotation_channel_keeps_the_access_token_out_of_storage() {
        let sub = item("fn subscribe_peer_rotations(");
        let cast = item("fn broadcast_rotation(");
        for f in [&sub, &cast] {
            for banned in ["local_storage", "session_storage", "set_item", "persist("] {
                assert!(
                    !f.contains(banned),
                    "the peer rotation channel must not touch `{banned}` — it carries an access \
                     token, and T-126 S5 is that the access token is never persisted"
                );
            }
        }
        assert!(
            item_src("fn subscribe_peer_rotations(").contains("\"BroadcastChannel\""),
            "the channel must be a BroadcastChannel (in-memory, dies with the page)"
        );
        assert!(
            cast.contains("postMessage")
                || item_src("fn broadcast_rotation(").contains("postMessage"),
            "a rotation must actually be announced, or no waiter can ever adopt"
        );
    }

    /* ═════════════ T-156 — a 401 is not an empty list ═════════════ */

    /// **The T-156 defect, stated as a value.** An empty success and a dead session used to arrive
    /// at the render site as the same thing — `None`, then an empty state. They are now different
    /// values of different variants, and no combinator on [`Fetched`] merges them.
    #[test]
    fn an_empty_result_and_a_401_are_different_values() {
        let empty: Fetched<Vec<u8>> = Ok(Vec::new()).into();
        let dead: Fetched<Vec<u8>> = Err((401u16, None)).into();

        assert_ne!(
            empty, dead,
            "an empty list and an expired session must not be the same value — that equality IS \
             the bug (the Arsenal's non-character guard, the 'No factions yet' CTA)"
        );
        assert_eq!(empty.data(), Some(&Vec::new()));
        assert!(
            !empty.is_session_expired(),
            "a genuinely empty response must never raise the session banner"
        );
        assert!(
            dead.is_session_expired(),
            "a 401 must reach the render site AS a 401 (perturbation: map Err(_) to Data(vec![]))"
        );
        assert_eq!(dead.data(), None, "a 401 carries no data to render");
    }

    /// The other half: not every failure is a session expiry. Saying "log in again" to someone
    /// whose wifi dropped, or who is logged in but lacks the role, is its own wrong answer.
    #[test]
    fn only_a_terminal_401_counts_as_a_session_expiry() {
        let cases: [(ApiErr, ApiFailure); 4] = [
            (
                (401, Some("invalid or expired token".into())),
                ApiFailure::SessionExpired {
                    message: Some("invalid or expired token".into()),
                },
            ),
            // middleware/auth.rs:83 — logged in, wrong role. Not a session problem.
            (
                (403, Some("insufficient role".into())),
                ApiFailure::Http {
                    status: 403,
                    message: Some("insufficient role".into()),
                },
            ),
            (
                (500, None),
                ApiFailure::Http {
                    status: 500,
                    message: None,
                },
            ),
            // The client's own "never reached the backend" sentinel.
            ((0, None), ApiFailure::Transport),
        ];
        for (err, want) in cases {
            let got: ApiFailure = err.clone().into();
            assert_eq!(got, want, "classifying {err:?}");
            assert_eq!(
                got.is_session_expired(),
                err.0 == 401,
                "only the terminal 401 may say 'log in again' — {err:?}"
            );
        }
    }

    /// [`Fetched::view`] is **total**: it cannot be called without an answer for the failure case,
    /// which is what stops a render site quietly falling through to its empty state.
    #[test]
    fn view_forces_the_failure_arm_to_exist_and_runs_it() {
        let dead: Fetched<Vec<u8>> = Err((401u16, None)).into();
        let rendered = dead.view(
            |rows| format!("{} rows", rows.len()),
            |f| {
                if f.is_session_expired() {
                    "session expired — log in again".to_string()
                } else {
                    "could not load".to_string()
                }
            },
        );
        assert_eq!(rendered, "session expired — log in again");

        let empty: Fetched<Vec<u8>> = Ok(Vec::new()).into();
        assert_eq!(
            empty.view(|rows| format!("{} rows", rows.len()), |_| "error".into()),
            "0 rows",
            "an empty success still renders as data — the empty state is correct HERE"
        );
    }

    /// **The pin that keeps the footgun from growing back.** [`Fetched`] earns its keep only while
    /// there is no one-token way to collapse it to "nothing": each of these would restore exactly
    /// the `.ok().unwrap_or_default()` shape that rendered a 401 as an empty list.
    #[test]
    fn the_fetched_type_offers_no_collapse_to_empty() {
        let p = prod();
        let start = p
            .find("impl<T> Fetched<T>")
            .expect("the Fetched impl must exist");
        let block = &p[start..];
        let block = &block[..block.find("\nimpl").unwrap_or(block.len())];
        for banned in [
            "unwrap_or_default",
            "unwrap_or_else",
            "unwrap_or(",
            "fn ok(",
            "fn into_data",
            "impl<T> Default for Fetched",
        ] {
            assert!(
                !block.contains(banned),
                "`{banned}` on Fetched hands a caller an empty value for a failed fetch, which is \
                 the T-156 bug with extra steps"
            );
        }
        assert!(
            !p.contains("impl<T: Default> Fetched<T>"),
            "a Default-bounded impl is where `unwrap_or_default` comes back"
        );
        assert!(
            block.contains("pub fn view<R>"),
            "the total reader must exist, or callers have only the partial accessors"
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
        use crate::editor::arsenal::class_r_scrub::{live_code, only_item};
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
