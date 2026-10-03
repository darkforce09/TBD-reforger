//! Token refresh policy: the one retry a `401` is allowed, and the rule a tab inside the cross-tab
//! critical section spends by.
//!
//! **Role:** owns the retry state machine every request verb routes its `401` through, the sharing
//! of one refresh among concurrent `401`s through a single-flight cell, and the generic cross-tab
//! policy that decides *which* token a tab spends once it holds the lock.
//! **Position:** sits between the request verbs and their `TokenProvider`; the verbs hand it the
//! provider's cell and refresh. The browser half of the refresh — the per-tab cell, the Web Lock,
//! the peer-rotation channel and the refresh request — is the session refresh above the
//! transport, which applies [`peer_rotation_supersedes`] inside its critical section.
//! **Signals & state:** none; pure functions over the closures and the cell they are handed.
//! **Invariants:** refresh tokens are single-use — the backend rotates and revokes on every call,
//! and presenting an already-revoked token revokes the whole family, logging every tab out. So the
//! token is chosen *inside* the critical section, never before waiting for it. Layering, outermost
//! first: the retry state machine, the per-tab single flight, the cross-tab lock, then one POST.

#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::transport::client::SingleFlight;
#[cfg(any(target_arch = "wasm32", test))]
use crate::foundation::transport::dto::RefreshResponse;
#[cfg(test)]
use futures::future::FutureExt;
#[cfg(any(target_arch = "wasm32", test))]
use futures::future::LocalBoxFuture;

#[cfg(any(target_arch = "wasm32", test))]
use super::errors::ApiErr;
#[cfg(any(target_arch = "wasm32", test))]
use super::Req;

/// Does a peer tab's broadcast rotation supersede the token this tab was about to spend?
///
/// A rotation always changes the refresh token, so "the broadcast carries a token that is not the
/// one I hold" is exactly "a peer rotated after I read mine". The negation matters just as much: a
/// tab that already adopted a pair must not treat that same pair as a reason to skip its own
/// refresh, or its access token could never be renewed again.
#[cfg(any(target_arch = "wasm32", test))]
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
/// success is a lock that a failed refresh wedges forever. The tests are its only caller: the
/// browser session refresh applies [`peer_rotation_supersedes`] inside its own critical section.
#[cfg(test)]
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
/// as `(401, None)` — there is no second refresh and no loop. The tests are its only caller; the
/// request verbs call [`send_with_refresh_for_generation`] with their session generation.
#[cfg(test)]
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
#[cfg(any(target_arch = "wasm32", test))]
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

#[cfg(test)]
#[path = "tests/refresh_generation.rs"]
mod tests;
