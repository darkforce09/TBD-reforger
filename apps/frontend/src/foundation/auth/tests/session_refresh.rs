//! Source pins on the browser half of the session refresh: the refresh request is reachable only
//! inside the cross-tab lock, the lock is a Web Lock, and the peer-rotation channel never touches
//! storage.
//!
//! The session refresh is compiled for `wasm32` only, so these read its scrubbed source natively.

/// The session refresh with comments, string literals and every construct that cannot run removed.
///
/// Reading the raw text would not do: every needle these assertions look for is discussed in prose
/// somewhere in the module, so each would be satisfied by a comment line — and by anything parked
/// in a constant-false block, an uncompiled item, or after an unconditional return.
fn prod() -> String {
    crate::foundation::test_support::class_r_scrub::live_code(
        &crate::foundation::test_support::pins::session_refresh_source(),
    )
}

/// Same, but string literals survive — for the assertions where the literal **is** the contract
/// (an API name reached through reflection).
fn prod_src() -> String {
    crate::foundation::test_support::class_r_scrub::live_source(
        &crate::foundation::test_support::pins::session_refresh_source(),
    )
}

/// The shipped HTTP client, scrubbed the same way.
fn client_prod() -> String {
    crate::foundation::test_support::class_r_scrub::live_code(
        &crate::foundation::test_support::pins::client_source(),
    )
}

/// The body of the ONE item whose signature is `start`, refusing a signature that appears twice,
/// so a pristine shadow copy in a never-called module cannot stand in for the real one.
fn item(start: &str) -> String {
    crate::foundation::test_support::class_r_scrub::only_item(&prod(), start).to_string()
}

/// [`item`], literals kept.
fn item_src(start: &str) -> String {
    crate::foundation::test_support::class_r_scrub::only_item(&prod_src(), start).to_string()
}

/// **The POST is inside the lock.** `refresh_locked` is the only caller of `refresh_via_gloo`,
/// and it reaches it through `with_refresh_lock` after checking session ownership. A helper that called
/// the request directly would be back to per-tab-only serialisation, and
/// the client's `every_auth_path_goes_through_the_one_single_flight_cell` alone would not notice —
/// its needle is `refresh_locked`, which a rename satisfies.
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
        "with_refresh_lock",
        "persisted_belongs_to_session(",
        "refresh_via_gloo(store, persisted.refresh_token)",
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
    let held = item("async fn with_refresh_lock<");
    assert!(
        held.contains("request.call2(") && held.contains("REFRESH_LOCK_NAME"),
        "the critical section must be `navigator.locks.request(REFRESH_LOCK_NAME, cb)`"
    );
    assert!(
        held.contains("future_to_promise"),
        "the lock is held for as long as the callback's promise is pending — the body must be \
         handed back as a promise, or the lock releases before the POST runs"
    );
    // No hand-rolled expiring flag anywhere in the session refresh or the shipped client: the
    // reason Web Locks was chosen is that there is no stale-lock state to expire.
    for banned in [
        "lock_expires",
        "lock_deadline",
        "LOCK_TTL",
        "lock_acquired_at",
    ] {
        assert!(
            !prod().contains(banned) && !client_prod().contains(banned),
            "`{banned}` suggests a storage-flag lock with an expiry — Web Locks needs none"
        );
    }
}

/// **The peer channel never persists the access token.** The rotated pair crosses tabs in
/// memory precisely because it carries an access token, and the access token is never written to
/// storage — the auth module has its own test pinning its absence from the persisted blob. A
/// simplification that routed the pair through storage would put it on disk for every tab and every
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
                 token, and  S5 is that the access token is never persisted"
            );
        }
    }
    assert!(
        item_src("fn subscribe_peer_rotations(").contains("\"BroadcastChannel\""),
        "the channel must be a BroadcastChannel (in-memory, dies with the page)"
    );
    assert!(
        cast.contains("postMessage") || item_src("fn broadcast_rotation(").contains("postMessage"),
        "a rotation must actually be announced, or no waiter can ever adopt"
    );
}
