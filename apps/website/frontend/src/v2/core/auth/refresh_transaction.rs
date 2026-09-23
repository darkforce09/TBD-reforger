//! A single-use credential is removed before its network effect and replaced only on success.

/// All callbacks run while the caller owns the cross-tab Web Lock. Removing the spent credential
/// first prevents storage failure or response loss from leaving it available for a replay. The
/// request future must be lazy: constructing it cannot issue the network request.
pub async fn rotate_with_storage<T>(
    consume: impl FnOnce() -> bool,
    request: impl std::future::Future<Output = Option<T>>,
    persist_successor: impl FnOnce(&T) -> bool,
) -> Option<T> {
    if !consume() {
        return None;
    }
    let successor = request.await?;
    if !persist_successor(&successor) {
        return None;
    }
    Some(successor)
}

/// Bound the network effect while retaining the caller's lock. Dropping a request future alone
/// does not abort browser fetch, so the deadline explicitly signals the transport before return.
pub async fn with_deadline<T>(
    request: impl std::future::Future<Output = Option<T>>,
    deadline: impl std::future::Future<Output = ()>,
    abort: impl FnOnce(),
) -> Option<T> {
    futures::pin_mut!(request, deadline);
    match futures::future::select(request, deadline).await {
        futures::future::Either::Left((result, _)) => result,
        futures::future::Either::Right(((), _)) => {
            abort();
            None
        }
    }
}

#[cfg(test)]
#[path = "tests/refresh_transaction.rs"]
mod tests;
