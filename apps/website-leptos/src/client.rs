//! HTTP client retry contract — ports the api/client.ts response interceptor.
//!
//! On a 401 the client refreshes the token **once** (through the single-flight cell so concurrent
//! 401s share one refresh) and retries the original request **once** with the rotated access token;
//! any other status, or a retry that is still 401, propagates. The state machine is generic over
//! the transport so it is unit-tested natively (single-retry, no loop); the wasm client wires
//! `send`/`refresh` to gloo-net on top.

use crate::auth::{RefreshResponse, SingleFlight};
use futures::future::LocalBoxFuture;

/// A pending request: resolves to `Ok(T)` or `Err(status)`.
pub type Req<T> = LocalBoxFuture<'static, Result<T, u16>>;

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
) -> Result<T, u16> {
    match send(token()).await {
        Err(401) => match sf.run(refresh).await {
            Some(r) => {
                on_refreshed(&r);
                send(Some(r.access_token)).await // the single retry
            }
            None => Err(401),
        },
        other => other,
    }
}

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
        let out: Result<&str, u16> = block_on(send_with_refresh(
            &sf,
            move |tok| {
                let s = s.clone();
                async move {
                    s.set(s.get() + 1);
                    if tok.as_deref() == Some("new") { Ok("ok") } else { Err(401u16) }
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
        let out: Result<&str, u16> = block_on(send_with_refresh(
            &sf,
            move |_tok| {
                let s = s.clone();
                async move {
                    s.set(s.get() + 1);
                    Err(401u16)
                }
                .boxed_local()
            },
            || Some("stale".to_string()),
            || async { Some(rr("new")) }.boxed_local(),
            |_| {},
        ));
        assert_eq!(out, Err(401));
        assert_eq!(sends.get(), 2, "one retry only — no loop");
    }

    // A non-401 error is not retried and does not refresh.
    #[test]
    fn non_401_propagates_without_refresh() {
        let refreshes = Rc::new(Cell::new(0));
        let sf = SingleFlight::<Option<RefreshResponse>>::new();
        let r = refreshes.clone();
        let out: Result<&str, u16> = block_on(send_with_refresh(
            &sf,
            |_tok| async { Err(500u16) }.boxed_local(),
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
        assert_eq!(out, Err(500));
        assert_eq!(refreshes.get(), 0, "non-401 never refreshes");
    }
}
