//! Revalidate long-lived event streams on every delivery, at expiry, and during idle periods.

use crate::core::{
    application_state::AppState,
    middleware::{AuthUser, role_rank},
};
use async_stream::stream;
use axum::response::sse::Event;
use futures::{Stream, StreamExt};
use std::{convert::Infallible, time::Duration};

pub fn authorize_event_stream<S>(
    events: S,
    state: AppState,
    user: AuthUser,
    required_role: &'static str,
) -> impl Stream<Item = Result<Event, Infallible>> + Send
where
    S: Stream<Item = Result<Event, Infallible>> + Send + 'static,
{
    stream! {
        futures::pin_mut!(events);
        loop {
            let remaining = (user.session_claims.exp - chrono::Utc::now().timestamp()).clamp(0, 5) as u64;
            let item = tokio::select! {
                item = events.next() => Some(item),
                _ = tokio::time::sleep(Duration::from_secs(remaining)) => None,
            };
            let authority = state.session_authority.authorize(user.session_claims.clone()).await;
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
