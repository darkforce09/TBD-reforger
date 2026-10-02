//! Unit coverage for the authorized event stream: deliveries pass after re-authorization, a role
//! that stops qualifying ends the stream with `authorization_expired`, and shutdown closes the
//! stream with no further event whether it is idle, holding a ready delivery, or waiting on the
//! session authority.
//!
//! Every case races a signal of its own through `authorize_until_shutdown`; none begins the
//! process-wide shutdown, which the library's other unit tests share. The session authority is a
//! stub, and the pool never connects.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures::future::BoxFuture;
use futures::stream;
use tokio::time::timeout;

use super::*;
use crate::core::authentication_primitives::Claims;
use crate::core::authentication_primitives::session_authority::SessionAuthority;
use crate::core::configuration::Config;
use crate::core::database::connect_lazy;
use crate::core::error_handling::api_error::ApiError;
use crate::core::process_lifecycle::ShutdownSignal;

/// A database URL nothing listens on; no case reaches the pool.
const UNUSED_DATABASE: &str = "postgres://authorized-event-stream/unused";

/// How long a case waits to show the stream is still open.
const OPEN_WINDOW: Duration = Duration::from_millis(100);

/// The bound on the stream's next item once it must answer.
const ANSWER_BOUND: Duration = Duration::from_secs(1);

/// Answers every session with the role it was built with and counts the calls.
struct GrantingAuthority {
    role: &'static str,
    calls: Arc<AtomicUsize>,
}

impl SessionAuthority for GrantingAuthority {
    fn authorize(&self, claims: Claims) -> BoxFuture<'static, Result<AuthUser, ApiError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let current = member(claims, self.role);
        Box::pin(async move { Ok(current) })
    }
}

/// Never answers: a re-authorization that stays in flight.
struct StalledAuthority {
    calls: Arc<AtomicUsize>,
}

impl SessionAuthority for StalledAuthority {
    fn authorize(&self, _claims: Claims) -> BoxFuture<'static, Result<AuthUser, ApiError>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(std::future::pending())
    }
}

fn session_claims() -> Claims {
    let now = chrono::Utc::now().timestamp();
    Claims {
        role: "admin".into(),
        arma_linked: true,
        sub: "event-stream-member".into(),
        iss: "authorized-event-stream-tests".into(),
        aud: "authorized-event-stream-tests".into(),
        sid: uuid::Uuid::new_v4(),
        iat: now,
        exp: now + 900,
    }
}

fn member(claims: Claims, role: &str) -> AuthUser {
    AuthUser {
        discord_id: claims.sub.clone(),
        role: role.into(),
        arma_linked: true,
        session_claims: claims,
        membership_stale: false,
        membership_override_active: false,
        can_manage_membership_override: false,
    }
}

fn state_with(authority: Arc<dyn SessionAuthority>) -> AppState {
    let pool = connect_lazy(UNUSED_DATABASE).expect("lazy pool");
    let mut state = AppState::new(
        pool,
        Config::for_tests(UNUSED_DATABASE, "authorized-event-stream-secret"),
    );
    state.session_authority = authority;
    state
}

fn granting(role: &'static str) -> (AppState, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let authority = GrantingAuthority {
        role,
        calls: Arc::clone(&calls),
    };
    (state_with(Arc::new(authority)), calls)
}

/// One ready event, then an inner stream that stays open.
fn one_event_then_open(data: &str) -> impl Stream<Item = Result<Event, Infallible>> + Send + use<> {
    stream::iter([Ok(Event::default().data(data))]).chain(stream::pending())
}

/// The SSE text an event renders to.
fn rendered(item: Option<Result<Event, Infallible>>) -> String {
    match item {
        Some(Ok(event)) => format!("{event:?}"),
        None => panic!("expected an event, the stream ended"),
    }
}

#[tokio::test]
async fn a_delivery_is_yielded_after_the_authority_confirms_the_session() {
    let (state, calls) = granting("admin");
    let signal = ShutdownSignal::new();
    let mut events = Box::pin(authorize_until_shutdown(
        one_event_then_open("first delivery"),
        state,
        member(session_claims(), "admin"),
        "admin",
        signal.begun(),
    ));

    let first = timeout(ANSWER_BOUND, events.next())
        .await
        .expect("the ready delivery is yielded");
    assert!(rendered(first).contains("first delivery"));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "one re-authorization per delivery"
    );
    assert!(
        timeout(OPEN_WINDOW, events.next()).await.is_err(),
        "the stream stays open while the inner stream does"
    );
}

#[tokio::test]
async fn shutdown_closes_an_idle_stream_without_an_event() {
    let (state, calls) = granting("admin");
    let signal = ShutdownSignal::new();
    let mut events = Box::pin(authorize_until_shutdown(
        stream::pending(),
        state,
        member(session_claims(), "admin"),
        "admin",
        signal.begun(),
    ));
    assert!(
        timeout(OPEN_WINDOW, events.next()).await.is_err(),
        "an idle stream stays open before shutdown"
    );

    signal.begin();

    let after = timeout(ANSWER_BOUND, events.next())
        .await
        .expect("the stream answers promptly once shutdown begins");
    assert!(
        after.is_none(),
        "shutdown ends the stream with no event: {:?}",
        after.map(rendered_ok)
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "closing needs no re-authorization"
    );
}

#[tokio::test]
async fn a_begun_shutdown_outranks_a_ready_delivery() {
    let (state, calls) = granting("admin");
    let signal = ShutdownSignal::new();
    signal.begin();
    let mut events = Box::pin(authorize_until_shutdown(
        one_event_then_open("never delivered"),
        state,
        member(session_claims(), "admin"),
        "admin",
        signal.begun(),
    ));

    let first = timeout(ANSWER_BOUND, events.next())
        .await
        .expect("the stream answers at once");
    assert!(
        first.is_none(),
        "a stream opened after shutdown began ends before any delivery: {:?}",
        first.map(rendered_ok)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn shutdown_cuts_short_a_reauthorization_in_flight() {
    let calls = Arc::new(AtomicUsize::new(0));
    let state = state_with(Arc::new(StalledAuthority {
        calls: Arc::clone(&calls),
    }));
    let signal = ShutdownSignal::new();
    let mut events = Box::pin(authorize_until_shutdown(
        one_event_then_open("held by the authority"),
        state,
        member(session_claims(), "admin"),
        "admin",
        signal.begun(),
    ));
    assert!(
        timeout(OPEN_WINDOW, events.next()).await.is_err(),
        "the delivery waits on the re-authorization"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the re-authorization is in flight"
    );

    signal.begin();

    let after = timeout(ANSWER_BOUND, events.next())
        .await
        .expect("shutdown does not wait for the session authority");
    assert!(
        after.is_none(),
        "the held delivery is dropped and the stream ends: {:?}",
        after.map(rendered_ok)
    );
}

#[tokio::test]
async fn a_role_below_the_requirement_ends_with_authorization_expired() {
    let (state, _calls) = granting("enlisted");
    let signal = ShutdownSignal::new();
    let mut events = Box::pin(authorize_until_shutdown(
        one_event_then_open("withheld"),
        state,
        member(session_claims(), "admin"),
        "admin",
        signal.begun(),
    ));

    let first = timeout(ANSWER_BOUND, events.next())
        .await
        .expect("the stream answers");
    let first = rendered(first);
    assert!(first.contains("authorization_expired"), "{first}");
    assert!(!first.contains("withheld"), "the delivery is not yielded");
    let after = timeout(ANSWER_BOUND, events.next())
        .await
        .expect("the stream ends after the expiry event");
    assert!(after.is_none());
}

#[tokio::test]
async fn the_inner_stream_ending_ends_the_wrapper() {
    let (state, _calls) = granting("admin");
    let signal = ShutdownSignal::new();
    let mut events = Box::pin(authorize_until_shutdown(
        stream::empty(),
        state,
        member(session_claims(), "admin"),
        "admin",
        signal.begun(),
    ));
    let after = timeout(ANSWER_BOUND, events.next())
        .await
        .expect("the stream answers");
    assert!(after.is_none());
}

/// The SSE text of a yielded item, for failure messages.
fn rendered_ok(item: Result<Event, Infallible>) -> String {
    rendered(Some(item))
}
