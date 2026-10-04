//! Source pins: the production text of the session store, the content gates and the browser session
//! refresh, for the guard tests that read it.
//!
//! **Role:** each function returns the full text of one logical source file of this area. Where
//! that file is split into shards, the function joins them in declaration order, so a guard that
//! reads the whole file still sees every definition exactly once.
//! **Position:** test-only, inside the area it reads; called from the session's guard tests.
//! **Signals & state:** none; every shard is embedded at compile time.
//! **Invariants:** every `include_str!` is relative to this file and names a file of this area, so
//! the pins keep resolving wherever the area moves; a shard is never listed twice.

use frontend_test_support::source_shards::{production_shard, production_source};

/// The session store, everything it is built from, and the two content gates that read it.
pub(crate) fn auth_source() -> String {
    production_source(&[
        include_str!("../lib.rs"),
        include_str!("../gates.rs"),
        include_str!("../route_guard.rs"),
        include_str!("../session.rs"),
        include_str!("../store.rs"),
    ])
}

/// The two content gates alone, for the pins that ban a call anywhere in the gates' own file.
///
/// Native only, like the gate tests that read it.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn gates_source() -> String {
    production_shard(include_str!("../gates.rs"))
}

/// The browser half of the session refresh: the per-tab cell, the cross-tab lock, the peer
/// rotation channel, the refresh request, the token provider and the cold-start restore.
pub(crate) fn session_refresh_source() -> String {
    production_shard(include_str!("../session_refresh.rs"))
}
