//! Keeps a long-lived event stream authorized while it stays open, and closes it when the process
//! shuts down.
//!
//! **Role:** wraps the event stream of every SSE handler: asks the session authority again before
//! each delivery and at least every five seconds (sooner when the access token expires first),
//! ends the stream with an `authorization_expired` event as soon as the session or its role stops
//! qualifying, and ends it with no further event once process shutdown begins.
//! **Position:** `api_http_layer::middleware`. The audit log feed of `api_administration` and the
//! server status stream of `api_server_infrastructure` pass their streams through [`authorize_event_stream`]; the
//! shutdown it waits on is [`api_configuration::process_lifecycle::process_shutdown`], which
//! `apps/api/src/bin/api.rs` begins on SIGINT or SIGTERM.
//! It reads only the session authority, taken from the caller's state through `FromRef`, so it
//! names no application state type.
//! **Signals & state:** one async stream per connection owning the inner stream, the caller's
//! session claims, the session authority and a shutdown waiter; no shared state.
//! **Invariants:** shutdown outranks a ready delivery and cuts short a re-authorization in flight,
//! so a stream ends within one poll of shutdown beginning and a graceful shutdown never waits on
//! an open stream; the end is the plain end of the body, so a client reconnects with
//! `Last-Event-ID` as it does after any end of stream; a delivery is yielded only after the
//! authority confirmed the session again.

use crate::authentication_primitives::session_authority::SessionAuthority;
use crate::middleware::{AuthUser, role_rank};
use api_configuration::process_lifecycle::process_shutdown;
use async_stream::stream;
use axum::extract::FromRef;
use axum::response::sse::Event;
use futures::{Stream, StreamExt};
use std::{convert::Infallible, future::Future, sync::Arc, time::Duration};

/// Wraps `events` for a caller holding at least `required_role`: re-authorized on every
/// delivery, at expiry and during idle periods, and ended when process shutdown begins.
///
/// `state` is any state the session authority can be taken from through `FromRef`.
pub fn authorize_event_stream<S, St>(
    events: S,
    state: St,
    user: AuthUser,
    required_role: &'static str,
) -> impl Stream<Item = Result<Event, Infallible>> + Send
where
    S: Stream<Item = Result<Event, Infallible>> + Send + 'static,
    Arc<dyn SessionAuthority>: FromRef<St>,
{
    authorize_until_shutdown(
        events,
        Arc::<dyn SessionAuthority>::from_ref(&state),
        user,
        required_role,
        process_shutdown().begun(),
    )
}

/// [`authorize_event_stream`] ended by `shutdown` resolving rather than by the process-wide
/// shutdown.
fn authorize_until_shutdown<S, F>(
    events: S,
    session_authority: Arc<dyn SessionAuthority>,
    user: AuthUser,
    required_role: &'static str,
    shutdown: F,
) -> impl Stream<Item = Result<Event, Infallible>> + Send
where
    S: Stream<Item = Result<Event, Infallible>> + Send + 'static,
    F: Future<Output = ()> + Send + 'static,
{
    stream! {
        futures::pin_mut!(events);
        futures::pin_mut!(shutdown);
        loop {
            let remaining = (user.session_claims.exp - chrono::Utc::now().timestamp()).clamp(0, 5) as u64;
            let item = tokio::select! {
                biased;
                () = &mut shutdown => break,
                item = events.next() => Some(item),
                () = tokio::time::sleep(Duration::from_secs(remaining)) => None,
            };
            let authority = tokio::select! {
                biased;
                () = &mut shutdown => break,
                authority = session_authority.authorize(user.session_claims.clone()) => authority,
            };
            if authority.is_err() || authority.as_ref().is_ok_and(|current| role_rank(&current.role) < role_rank(required_role)) {
                yield Ok(Event::default().event("authorization_expired").data("session permissions changed; reconnect"));
                break;
            }
            match item {
                Some(Some(event)) => yield event,
                Some(None) => break,
                None => {}
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/authorized_event_stream.rs"]
mod tests;
