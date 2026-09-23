//! Token refresh: the policy, the per-tab single flight, and the cross-tab critical section.
//!
//! **Role:** owns everything that decides *when* a refresh token is spent and *who* spends it, and
//! the one retry a `401` is allowed.
//! **Position:** sits between the request verbs and the auth store; every verb routes its `401`
//! through here.
//! **Signals & state:** `REFRESH_SF` holds the per-tab single-flight cell; `PEER_ROTATION` holds
//! the last rotation a peer tab announced; `PEER_CHANNEL` holds the broadcast channel object alive
//! so its listener keeps firing. All three are thread-locals, so each browsing context has its own.
//! Reads `AuthStore::refresh_token` and `AuthStore::access_token` untracked.
//! **Invariants:** refresh tokens are single-use — the backend rotates and revokes on every call,
//! and presenting an already-revoked token revokes the whole family, logging every tab out. So the
//! token is chosen *inside* the critical section, never before waiting for it. Layering, outermost
//! first: the retry state machine, the per-tab single flight, the cross-tab lock, then one POST.

#[cfg(target_arch = "wasm32")]
use crate::v2::core::auth::load_persisted;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
// The lock manager and the broadcast channel are both reached through reflection, and every hop
// across that boundary needs the JavaScript-value casts.
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
use super::API_BASE;
#[cfg(target_arch = "wasm32")]
use crate::v2::core::auth::AuthStore;
use crate::v2::core::auth::{RefreshResponse, SingleFlight};
use futures::future::{FutureExt, LocalBoxFuture};

use super::errors::ApiErr;
use super::Req;

/// Name the cross-tab refresh critical section is taken under.
///
/// Web Locks are scoped per origin, so every same-origin tab of the app contends on this one
/// string. The broadcast channel that carries peer rotations uses the same name.
#[allow(dead_code)]
pub const REFRESH_LOCK_NAME: &str = "tbd-auth-refresh";

/// Does a peer tab's broadcast rotation supersede the token this tab was about to spend?
///
/// A rotation always changes the refresh token, so "the broadcast carries a token that is not the
/// one I hold" is exactly "a peer rotated after I read mine". The negation matters just as much: a
/// tab that already adopted a pair must not treat that same pair as a reason to skip its own
/// refresh, or its access token could never be renewed again.
#[allow(dead_code)]
pub fn peer_rotation_supersedes(peer: &RefreshResponse, about_to_spend: Option<&str>) -> bool {
    Some(peer.refresh_token.as_str()) != about_to_spend
}

/// One refresh attempt, serialised across every tab of this origin.
///
/// Holding a lock is not enough on its own: a tab that waits, wins the lock, and then spends the
/// token it read *before* waiting has merely made the double-spend orderly. So inside the critical
/// section this runs three steps in order.
///
/// 1. **Adopt.** If a peer broadcast a rotation that [`peer_rotation_supersedes`] the token this
///    tab was about to spend, take its pair and return with no request at all.
/// 2. **Re-read.** Otherwise take the freshest refresh token from shared storage rather than the
///    stale copy held in this tab's own signal. The fallback to `entry_token` covers the storage
///    read failing, which happens in some private-browsing configurations.
/// 3. **Spend** exactly that token.
///
/// A lost race is expensive, which is why the double-check exists: the backend treats presentation
/// of an already-revoked token as reuse and revokes every live token for that user, including the
/// pair the winner just minted. The price is not one failed request in one tab, it is every tab
/// logged out.
///
/// The browser writer persists the successor while holding the cross-tab lock.
///
/// Generic over the lock so that the policy — which is all of the correctness — can be tested
/// natively against a real fair mutex. `with_lock` must run its body with the lock held and release
/// when the body settles, **including when it settles with `None`**: a lock released only on
/// success is a lock that a failed refresh wedges forever.
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

/// Send a request, and on a `401` refresh once through `sf` and retry the request exactly once
/// with the rotated access token.
///
/// `on_refreshed` is called with the new pair before the retry so the store and its persisted copy
/// are current. Any other status propagates unchanged, and a retry that is still `401` is returned
/// as `(401, None)` — there is no second refresh and no loop.
#[allow(dead_code)]
pub async fn send_with_refresh<T>(
    sf: &SingleFlight<Option<RefreshResponse>>,
    send: impl Fn(Option<String>) -> Req<T>,
    token: impl Fn() -> Option<String>,
    refresh: impl FnOnce() -> LocalBoxFuture<'static, Option<RefreshResponse>>,
    on_refreshed: impl FnOnce(&RefreshResponse),
) -> Result<T, ApiErr> {
    send_with_refresh_for_generation(sf, 0, || true, send, token, refresh, on_refreshed).await
}

/// Share refresh only among requests from the same authenticated session generation.
#[allow(dead_code)]
pub async fn send_with_refresh_for_generation<T>(
    sf: &SingleFlight<Option<RefreshResponse>>,
    generation: u64,
    is_current: impl Fn() -> bool,
    send: impl Fn(Option<String>) -> Req<T>,
    token: impl Fn() -> Option<String>,
    refresh: impl FnOnce() -> LocalBoxFuture<'static, Option<RefreshResponse>>,
    on_refreshed: impl FnOnce(&RefreshResponse),
) -> Result<T, ApiErr> {
    match send(token()).await {
        Err((401, _)) if !is_current() => Err((401, None)),
        Err((401, _)) => match sf.run_keyed(generation, refresh).await {
            Some(r) if is_current() => {
                on_refreshed(&r);
                send(Some(r.access_token)).await // the single retry
            }
            _ => Err((401, None)),
        },
        other => other,
    }
}

/* The browser half: the single-flight cell, the lock, and the peer-rotation channel. */

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// The per-tab single-flight cell, so concurrent `401`s in one tab share one refresh.
    pub(super) static REFRESH_SF: SingleFlight<Option<RefreshResponse>> = SingleFlight::new();
}

#[cfg(target_arch = "wasm32")]
/// The one request that spends the single-use refresh token, and announces the result to peers.
///
/// The token is a parameter rather than a field read: the freshest value is chosen inside the
/// critical section by [`refresh_cross_tab`] and handed in, so the value spent is the value that
/// was current at the moment of spending.
async fn refresh_via_gloo(store: AuthStore, token: Option<String>) -> Option<RefreshResponse> {
    use crate::v2::core::auth::session::{clear_persisted, persist};
    use crate::v2::core::auth::session_identity::access_token_session_id;
    let generation = store.current_generation();
    let expected_session = store.current_session_id();
    let current = load_persisted()?;
    if current.refresh_token != token || token.as_deref().is_none_or(str::is_empty) {
        store.clear_session();
        return None;
    }
    let body = serde_json::json!({ "refresh_token": token });
    let abort = web_sys::AbortController::new().ok()?;
    let req = gloo_net::http::Request::post(&format!("{API_BASE}/auth/refresh"))
        .abort_signal(Some(&abort.signal()))
        .credentials(web_sys::RequestCredentials::Include)
        .json(&body)
        .ok()?;
    let rotated = crate::v2::core::auth::refresh_transaction::rotate_with_storage(
        clear_persisted,
        crate::v2::core::auth::refresh_transaction::with_deadline(
            async move {
                let response = req.send().await.ok()?;
                if !(200..300).contains(&response.status()) {
                    return None;
                }
                response.json::<RefreshResponse>().await.ok()
            },
            gloo_timers::future::TimeoutFuture::new(30_000),
            move || abort.abort(),
        ),
        |rotated| {
            let Some(session_id) = access_token_session_id(&rotated.access_token) else {
                return false;
            };
            if !store.is_current_generation(generation)
                || expected_session
                    .as_ref()
                    .is_some_and(|expected| *expected != session_id)
                || load_persisted().is_some()
            {
                return false;
            }
            store.set_tokens(rotated.clone());
            persist(&store.persist_state())
        },
    )
    .await;
    if let Some(rotated) = &rotated {
        broadcast_rotation(rotated);
    } else if store.is_current_generation(generation) {
        store.clear_session();
    }
    rotated
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// The last rotation a peer tab announced.
    ///
    /// Memory only, never persisted storage: it carries an access token, and the access token is
    /// deliberately never written to disk. A broadcast message lives in the receiving page's heap
    /// and dies with it, which is the property that makes carrying the whole pair safe here and
    /// unsafe in storage.
    static PEER_ROTATION: std::rc::Rc<std::cell::RefCell<Option<RefreshResponse>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    /// The channel object, kept alive so its message listener keeps firing. `None` until the first
    /// subscription, and `None` for good where the channel API is unavailable.
    static PEER_CHANNEL: std::cell::RefCell<Option<js_sys::Object>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
/// `window.navigator.locks`, or `None` where the Web Locks API is not reachable.
///
/// Reached through reflection rather than through typed bindings because those bindings sit behind
/// an unstable build flag, which would be a workspace-wide switch for one call.
///
/// `None` happens for real: Web Locks require a secure context, so a build served over plain HTTP
/// has no lock manager at all.
fn lock_manager() -> Option<js_sys::Object> {
    let nav = web_sys::window()?.navigator();
    let locks = js_sys::Reflect::get(nav.as_ref(), &"locks".into()).ok()?;
    (!locks.is_undefined() && !locks.is_null()).then(|| locks.unchecked_into())
}

#[cfg(target_arch = "wasm32")]
/// Run `body` holding the cross-tab refresh lock, releasing when it settles.
///
/// The browser holds the lock for exactly as long as the promise the callback returns is pending,
/// and releases it when that promise settles **or when the page holding it goes away**. That last
/// clause is why this is a lock rather than a flag in shared storage: a tab that crashes mid-
/// refresh drops the lock immediately, whereas a flag would sit there and wedge every other tab.
/// There is no stale-lock state to recover from, and so no expiry to tune.
///
/// The body's value comes back through a cell rather than through the promise because it is a Rust
/// value, not a JavaScript one; the promise resolves with `undefined` purely as the release signal.
///
/// A missing or rejected lock leaves credentials unchanged and returns the default result.
/// Credential mutation never falls back to an unlocked read-modify-write.
pub(crate) async fn with_refresh_lock<T: Default + 'static>(body: LocalBoxFuture<'static, T>) -> T {
    let Some(locks) = lock_manager() else {
        return T::default();
    };
    let Ok(request) = js_sys::Reflect::get(&locks, &"request".into()) else {
        return T::default();
    };
    let Ok(request) = request.dyn_into::<js_sys::Function>() else {
        return T::default();
    };
    let out = std::rc::Rc::new(std::cell::RefCell::new(None::<T>));
    let sink = out.clone();
    let pending = std::rc::Rc::new(std::cell::RefCell::new(Some(body)));
    let deferred = pending.clone();
    // The closure remains alive until the browser releases the lock or rejects the request.
    let cb = wasm_bindgen::closure::Closure::once(
        move |_lock: wasm_bindgen::JsValue| -> wasm_bindgen::JsValue {
            let body = deferred.borrow_mut().take();
            wasm_bindgen_futures::future_to_promise(async move {
                if let Some(body) = body {
                    *sink.borrow_mut() = Some(body.await);
                }
                Ok(wasm_bindgen::JsValue::UNDEFINED)
            })
            .into()
        },
    );
    let Ok(p) = request.call2(&locks, &REFRESH_LOCK_NAME.into(), cb.as_ref()) else {
        return T::default();
    };
    let Ok(promise) = p.dyn_into::<js_sys::Promise>() else {
        return T::default();
    };
    if wasm_bindgen_futures::JsFuture::from(promise).await.is_err() {
        return T::default();
    }
    let value = out.borrow_mut().take().unwrap_or_default();
    value
}

#[cfg(target_arch = "wasm32")]
/// The broadcast channel, opening it on first use. `None` where the channel API is unavailable,
/// in which case the adopt step never fires and every waiter falls through to the re-read.
fn peer_channel() -> Option<js_sys::Object> {
    subscribe_peer_rotations();
    PEER_CHANNEL.with(|c| c.borrow().clone())
}

#[cfg(target_arch = "wasm32")]
/// Open the channel and start recording peer rotations.
///
/// Idempotent — a second call sees the cached object and returns — so it is safe to call on every
/// refresh as well as at bootstrap.
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
    let args = js_sys::Array::of1(&REFRESH_LOCK_NAME.into());
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

#[cfg(target_arch = "wasm32")]
/// Announce a rotation this tab just performed, so waiting peers adopt it instead of spending a
/// second one.
///
/// Best effort by design: a dropped announcement costs one extra valid rotation, never a spent
/// token.
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

#[cfg(target_arch = "wasm32")]
/// Bind session ownership to the browser: the lock for the critical section, the broadcast
/// channel for the adopt step, and persisted storage for the re-read.
///
/// The re-read goes to persisted storage rather than to the store's signal. The signal is this
/// tab's private copy and goes stale the instant a peer rotates; the persisted blob is the shared
/// copy every tab writes on every rotation, so it is the only honest answer to "what is the current
/// refresh token".
pub(super) async fn refresh_locked(store: AuthStore) -> Option<RefreshResponse> {
    use crate::v2::core::auth::session::persisted_belongs_to_session;
    use crate::v2::core::auth::session_identity::access_token_session_id;
    subscribe_peer_rotations();
    let generation = store.current_generation();
    let expected = store.persist_state();
    with_refresh_lock(
        async move {
            if !store.is_current_generation(generation) {
                return None;
            }
            let Some(persisted) = load_persisted() else {
                store.clear_session();
                return None;
            };
            if !persisted_belongs_to_session(&persisted, &expected)
                || persisted.refresh_token.as_deref().is_none_or(str::is_empty)
            {
                store.clear_session();
                return None;
            }
            let peer = PEER_ROTATION.with(|p| p.borrow().clone()).filter(|pair| {
                Some(&pair.refresh_token) == persisted.refresh_token.as_ref()
                    && access_token_session_id(&pair.access_token) == persisted.session_id
                    && peer_rotation_supersedes(pair, expected.refresh_token.as_deref())
            });
            if let Some(peer) = peer {
                return Some(peer);
            }
            refresh_via_gloo(store, persisted.refresh_token).await
        }
        .boxed_local(),
    )
    .await
}

#[cfg(test)]
#[path = "tests/refresh_generation.rs"]
mod tests;
