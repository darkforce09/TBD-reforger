//! The browser half of the live audit stream: the page-owned handle, the reconnect loop, one
//! connection, and its reader.
//!
//! **Role:** opens `GET /api/v1/admin/audit-logs/stream` as a bearer-authenticated fetch over a
//! readable stream, feeds the bytes to the event-stream parser and the parent module's tracker,
//! hands every step to the page's callbacks, and reconnects until the handle is aborted.
//! **Position:** compiled for `wasm32` only, under [`super`], whose pure half decides what each
//! parsed item, each answer status and each drop means; the audit logs page calls
//! [`open_audit_stream`] and owns the handle it returns.
//! **Signals & state:** the handle shares a stop flag and the abort controller of the connection
//! in flight with the spawned task; the task owns the tracker and the parser. It writes no signal
//! itself.
//! **Invariants:** every connection has its own abort controller, parked in the handle before the
//! fetch, so an abort always reaches the connection in flight. The task checks the stop flag after
//! every await and before every callback, so no callback runs once the handle is aborted. A
//! missing access token is treated as a `401`, because a cold start restores only the refresh
//! token and the revalidation mints the access token. `authorization_expired` revalidates and
//! reconnects at once, without a backoff wait.

use super::{
    connect_verdict, AuditStreamState, AuditStreamStep, AuditStreamTracker, ConnectVerdict,
    OfflineReason, AUDIT_STREAM_PATH,
};
use crate::v2::core::api::dto::administration::{
    AuditLogEntry, AuditStreamReady, AuditStreamReset,
};
use crate::v2::core::api::sse_frames::SseParser;
use crate::v2::core::auth::AuthStore;
use leptos::prelude::GetUntracked;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::JsCast;

/// The page's hooks into one audit stream. None of them runs once the stream is aborted.
pub struct AuditStreamCallbacks {
    /// The connection state changed.
    pub on_state: Box<dyn Fn(AuditStreamState)>,
    /// `ready` arrived; the flag says whether the history must be (re)loaded now.
    pub on_ready: Box<dyn Fn(AuditStreamReady, bool)>,
    /// A published audit line arrived.
    pub on_row: Box<dyn Fn(AuditLogEntry)>,
    /// `reset` arrived; the history must be reloaded.
    pub on_reset: Box<dyn Fn(AuditStreamReset)>,
    /// An event did not decode; the text names its type and the reason.
    pub on_rejected: Box<dyn Fn(String)>,
}

/// The page-owned handle of one audit stream: aborting it ends the stream for good.
#[derive(Clone)]
pub struct AuditStreamHandle {
    /// Set once by [`AuditStreamHandle::abort`]; the task checks it after every await.
    stopped: Rc<Cell<bool>>,
    /// The abort controller of the connection in flight, if any.
    controller: Rc<RefCell<Option<web_sys::AbortController>>>,
}

impl AuditStreamHandle {
    /// Abort the open connection and stop reconnecting; no callback runs afterwards. Idempotent.
    pub fn abort(&self) {
        self.stopped.set(true);
        self.abort_connection();
    }

    /// Whether [`AuditStreamHandle::abort`] has run.
    fn is_stopped(&self) -> bool {
        self.stopped.get()
    }

    /// Abort the connection in flight without stopping the stream.
    fn abort_connection(&self) {
        if let Some(controller) = self.controller.borrow_mut().take() {
            controller.abort();
        }
    }
}

/// Open the audit stream and drive `callbacks` from it until the caller aborts the handle.
pub fn open_audit_stream(store: AuthStore, callbacks: AuditStreamCallbacks) -> AuditStreamHandle {
    let handle = AuditStreamHandle {
        stopped: Rc::new(Cell::new(false)),
        controller: Rc::new(RefCell::new(None)),
    };
    leptos::task::spawn_local(run_stream(store, callbacks, handle.clone()));
    handle
}

/// How one connection attempt answered.
enum Attempt {
    /// A 2xx answer and its body reader.
    Open(web_sys::ReadableStreamDefaultReader),
    /// Any other answer's status, `0` for a network failure.
    Refused(u16),
    /// The handle aborted the attempt.
    Aborted,
}

/// How one open connection ended.
enum ConnectionEnd {
    /// The handle aborted it.
    Aborted,
    /// The body ended or failed.
    Closed,
    /// The server announced `authorization_expired`.
    AuthorizationExpired,
}

/// The reconnect loop: one connection at a time until the handle aborts or the stream stops.
async fn run_stream(store: AuthStore, callbacks: AuditStreamCallbacks, handle: AuditStreamHandle) {
    store.session_restored().await;
    let generation = store.current_generation();
    let mut tracker = AuditStreamTracker::new();
    let mut revalidated = false;
    let set_state = |state: AuditStreamState| {
        if !handle.is_stopped() {
            (callbacks.on_state)(state);
        }
    };
    loop {
        if handle.is_stopped() {
            return;
        }
        if !store.is_current_generation(generation) {
            set_state(AuditStreamState::Offline(OfflineReason::SignedOut));
            return;
        }
        let last_event_id = tracker.begin_connection();
        // A cold start restores only the refresh token; no access token is answered as the API
        // would answer it, so the revalidation below mints one.
        let attempt = match store.access_token.get_untracked() {
            Some(token) => connect(&handle, &token, last_event_id.as_deref()).await,
            None => Attempt::Refused(401),
        };
        let delay = match attempt {
            Attempt::Aborted => return,
            Attempt::Open(reader) => {
                revalidated = false;
                match pump(&handle, &reader, &mut tracker, &callbacks).await {
                    ConnectionEnd::Aborted => return,
                    ConnectionEnd::Closed => tracker.next_delay_ms(js_sys::Math::random()),
                    ConnectionEnd::AuthorizationExpired => {
                        set_state(AuditStreamState::Reconnecting);
                        revalidated = true;
                        revalidate_session(store).await;
                        continue;
                    }
                }
            }
            Attempt::Refused(status) => match connect_verdict(status, revalidated) {
                ConnectVerdict::Stop(reason) => {
                    set_state(AuditStreamState::Offline(reason));
                    return;
                }
                ConnectVerdict::Revalidate => {
                    revalidated = true;
                    revalidate_session(store).await;
                    continue;
                }
                ConnectVerdict::ForgetCursorAndBackOff => {
                    tracker.forget_cursor();
                    tracker.next_delay_ms(js_sys::Math::random())
                }
                ConnectVerdict::Stream | ConnectVerdict::BackOff => {
                    tracker.next_delay_ms(js_sys::Math::random())
                }
            },
        };
        set_state(AuditStreamState::Reconnecting);
        gloo_timers::future::TimeoutFuture::new(delay).await;
    }
}

/// Revalidate the session through the client's request path: fetching the viewer's own profile
/// spends a `401` on the shared single-flight refresh, so afterwards the store holds a rotated
/// access token, the unexpired one it had, or no session at all.
async fn revalidate_session(store: AuthStore) {
    let _ = crate::v2::core::api::client::api_get::<crate::v2::core::api::dto::MeResponse>(
        store, "/me",
    )
    .await;
}

/// The request of one connection: the bearer token, `Last-Event-ID` when resuming, and an abort
/// controller of its own, since an aborted controller cannot be reused.
fn stream_request(
    token: &str,
    last_event_id: Option<&str>,
) -> Option<(web_sys::Request, web_sys::AbortController)> {
    let controller = web_sys::AbortController::new().ok()?;
    let headers = web_sys::Headers::new().ok()?;
    headers
        .set("Authorization", &format!("Bearer {token}"))
        .ok()?;
    headers.set("Accept", "text/event-stream").ok()?;
    if let Some(id) = last_event_id {
        headers.set("Last-Event-ID", id).ok()?;
    }
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    init.set_headers(&headers);
    init.set_signal(Some(&controller.signal()));
    let url = format!(
        "{}{AUDIT_STREAM_PATH}",
        crate::v2::core::api::client::API_BASE
    );
    let request = web_sys::Request::new_with_str_and_init(&url, &init).ok()?;
    Some((request, controller))
}

/// Open one connection, parking its abort controller in the handle first.
async fn connect(handle: &AuditStreamHandle, token: &str, last_event_id: Option<&str>) -> Attempt {
    let (Some((request, controller)), Some(window)) =
        (stream_request(token, last_event_id), web_sys::window())
    else {
        return Attempt::Refused(0);
    };
    *handle.controller.borrow_mut() = Some(controller);
    let answer = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await;
    if handle.is_stopped() {
        return Attempt::Aborted;
    }
    let Some(response) = answer
        .ok()
        .and_then(|value| value.dyn_into::<web_sys::Response>().ok())
    else {
        return Attempt::Refused(0);
    };
    if !response.ok() {
        return Attempt::Refused(response.status());
    }
    match response.body() {
        Some(body) => Attempt::Open(body.get_reader().unchecked_into()),
        None => Attempt::Refused(0),
    }
}

/// Read one open connection to its end, handing every step to the callbacks.
async fn pump(
    handle: &AuditStreamHandle,
    reader: &web_sys::ReadableStreamDefaultReader,
    tracker: &mut AuditStreamTracker,
    callbacks: &AuditStreamCallbacks,
) -> ConnectionEnd {
    let mut parser = SseParser::new();
    loop {
        let chunk = wasm_bindgen_futures::JsFuture::from(reader.read()).await;
        if handle.is_stopped() {
            return ConnectionEnd::Aborted;
        }
        let Ok(chunk) = chunk else {
            return ConnectionEnd::Closed;
        };
        let done = js_sys::Reflect::get(&chunk, &"done".into())
            .ok()
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        if done {
            return ConnectionEnd::Closed;
        }
        let Some(bytes) = js_sys::Reflect::get(&chunk, &"value".into())
            .ok()
            .and_then(|value| value.dyn_into::<js_sys::Uint8Array>().ok())
        else {
            continue;
        };
        for item in parser.feed(&bytes.to_vec()) {
            if handle.is_stopped() {
                return ConnectionEnd::Aborted;
            }
            match tracker.observe(item) {
                AuditStreamStep::Ready {
                    ready,
                    history_required,
                } => {
                    (callbacks.on_state)(AuditStreamState::Live);
                    (callbacks.on_ready)(ready, history_required);
                }
                AuditStreamStep::Row(entry) => (callbacks.on_row)(entry),
                AuditStreamStep::Reset(reset) => (callbacks.on_reset)(reset),
                AuditStreamStep::AuthorizationExpired => {
                    handle.abort_connection();
                    return ConnectionEnd::AuthorizationExpired;
                }
                AuditStreamStep::Rejected { event, error } => {
                    let text = format!("A live `{event}` event could not be read: {error}");
                    leptos::logging::warn!("audit stream: {text}");
                    (callbacks.on_rejected)(text);
                }
                AuditStreamStep::Nothing => {}
            }
        }
    }
}
