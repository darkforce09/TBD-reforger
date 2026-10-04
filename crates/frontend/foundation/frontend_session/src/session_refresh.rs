//! The browser half of the session refresh: the per-tab single-flight cell, the cross-tab lock,
//! the peer-rotation channel, the one request that spends a refresh token, and the cold-start
//! restore.
//!
//! **Role:** decides *who* spends the session's single-use refresh token and *when*, persists the
//! rotated successor, and restores a persisted session on a cold start; implements the
//! transport's [`TokenProvider`] for [`AuthStore`], so every authenticated request refreshes
//! through here.
//! **Position:** compiled for `wasm32` only. Sits above the transport: it is generic over nothing,
//! applies the transport's [`peer_rotation_supersedes`] rule inside its critical section, and
//! sends the refresh and the profile read through the transport's request path; the request verbs
//! reach it only through the [`TokenProvider`] methods. The shell layout starts [`bootstrap`]
//! once at mount, and sign-out and sign-in paths take [`with_refresh_lock`] around their storage
//! writes.
//! **Signals & state:** `REFRESH_SF` holds the per-tab single-flight cell; `PEER_ROTATION` holds
//! the last rotation a peer tab announced; `PEER_CHANNEL` holds the broadcast channel object alive
//! so its listener keeps firing. All three are thread-locals, so each browsing context has its own.
//! Reads and writes the store's session signals untracked, and the persisted session blob.
//! **Invariants:** refresh tokens are single-use — the backend rotates and revokes on every call,
//! and presenting an already-revoked token revokes the whole family, logging every tab out. So the
//! token is chosen *inside* the cross-tab critical section, never before waiting for it, and the
//! successor is persisted while the lock is still held. Credential mutation never falls back to an
//! unlocked read-modify-write.

use futures::future::{FutureExt, LocalBoxFuture};
use leptos::prelude::*;
// The lock manager and the broadcast channel are both reached through reflection, and every hop
// across that boundary needs the JavaScript-value casts.
use wasm_bindgen::JsCast;

use super::session::{clear_persisted, load_persisted, persist, persisted_belongs_to_session};
use super::session_identity::access_token_session_id;
use super::store::AuthStore;
use frontend_api_dtos::{MeResponse, RefreshResponse};
use frontend_transport::client::{API_BASE, SingleFlight, api_get, peer_rotation_supersedes};
use frontend_transport::token_provider::TokenProvider;

/// Name the cross-tab refresh critical section is taken under.
///
/// Web Locks are scoped per origin, so every same-origin tab of the app contends on this one
/// string. The broadcast channel that carries peer rotations uses the same name.
pub const REFRESH_LOCK_NAME: &str = "tbd-auth-refresh";

thread_local! {
    /// The per-tab single-flight cell, so concurrent `401`s in one tab share one refresh.
    static REFRESH_SF: SingleFlight<Option<RefreshResponse>> = SingleFlight::new();
}

/// The one request that spends the single-use refresh token, and announces the result to peers.
///
/// The token is a parameter rather than a field read: the freshest value is chosen inside the
/// critical section by [`refresh_locked`] and handed in, so the value spent is the value that
/// was current at the moment of spending.
async fn refresh_via_gloo(store: AuthStore, token: Option<String>) -> Option<RefreshResponse> {
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
    let rotated = super::refresh_transaction::rotate_with_storage(
        clear_persisted,
        super::refresh_transaction::with_deadline(
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
///
/// Every read-modify-write of the persisted credential runs inside it: the refresh here, the
/// profile persistence, the app shell's sign-out and the sign-in callback's session install.
pub async fn with_refresh_lock<T: Default + 'static>(body: LocalBoxFuture<'static, T>) -> T {
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
    out.borrow_mut().take().unwrap_or_default()
}

/// The broadcast channel, opening it on first use. `None` where the channel API is unavailable,
/// in which case the adopt step never fires and every waiter falls through to the re-read.
fn peer_channel() -> Option<js_sys::Object> {
    subscribe_peer_rotations();
    PEER_CHANNEL.with(|c| c.borrow().clone())
}

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
            if let Some(text) = data.as_string()
                && let Ok(pair) = serde_json::from_str::<RefreshResponse>(&text)
            {
                *slot.borrow_mut() = Some(pair);
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

/// Bind session ownership to the browser: the lock for the critical section, the broadcast
/// channel for the adopt step, and persisted storage for the re-read.
///
/// The re-read goes to persisted storage rather than to the store's signal. The signal is this
/// tab's private copy and goes stale the instant a peer rotates; the persisted blob is the shared
/// copy every tab writes on every rotation, so it is the only honest answer to "what is the current
/// refresh token".
async fn refresh_locked(store: AuthStore) -> Option<RefreshResponse> {
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
                    && access_token_session_id(&pair.access_token)
                        == persisted.session_id.clone().map(String::from)
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

/// The session store as the transport's token source.
///
/// Every request verb captures its generation through here, sends the access token read here, and
/// refreshes through the one per-tab cell and the cross-tab locked refresh, so no request path can
/// open a second route to the single-use refresh token.
impl TokenProvider for AuthStore {
    fn session_restored(&self) -> LocalBoxFuture<'static, ()> {
        let store = *self;
        async move { AuthStore::session_restored(&store).await }.boxed_local()
    }

    fn current_generation(&self) -> u64 {
        AuthStore::current_generation(self)
    }

    fn is_current_generation(&self, generation: u64) -> bool {
        AuthStore::is_current_generation(self, generation)
    }

    fn access_token(&self) -> Option<String> {
        self.access_token.get_untracked()
    }

    fn set_tokens(&self, tokens: RefreshResponse) {
        AuthStore::set_tokens(self, tokens);
    }

    fn refresh_flight(&self) -> SingleFlight<Option<RefreshResponse>> {
        REFRESH_SF.with(|s| s.clone())
    }

    fn refresh(&self) -> LocalBoxFuture<'static, Option<RefreshResponse>> {
        refresh_locked(*self).boxed_local()
    }
}

/// Cold-start bootstrap: hydrate the tokens from persisted storage, then fetch the current user.
///
/// A stale or absent access token handles itself through the usual `401` refresh-and-retry path.
/// When nothing is persisted it settles the restore and stays a guest.
pub async fn bootstrap(store: AuthStore) {
    // A peer may temporarily remove its single-use credential while rotating it. Read only after
    // that peer releases the lock, so a new tab observes the settled result.
    let persisted = with_refresh_lock(async { load_persisted() }.boxed_local()).await;
    let Some(persisted) = persisted.filter(|p| p.refresh_token.is_some()) else {
        store.settle_session_restore();
        store.bootstrapping.set(false);
        return;
    };
    store.restore_persisted(persisted);
    store.bootstrapping.set(true);
    let generation = store.current_generation();
    let profile_request = store.begin_profile_request();
    if let Ok(me) = api_get::<MeResponse>(store, "/me").await
        && store.adopt_profile(profile_request, &me)
    {
        super::session::persist_profile_if_current(&store.persist_state()).await;
    }
    if store.is_current_generation(generation) {
        store.bootstrapping.set(false);
    }
}
