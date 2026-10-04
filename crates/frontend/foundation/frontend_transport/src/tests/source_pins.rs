//! Source pins: the production text of the HTTP client and the telemetry stream consumer, for the
//! guard tests that read it.
//!
//! **Role:** each function returns the full text of one logical source file of this area. Where
//! that file is split into shards, the function joins them in declaration order, so a guard that
//! reads the whole file still sees every definition exactly once.
//! **Position:** test-only, inside the area it reads; called from the transport's own guard tests,
//! and the session's guard over the refresh path through the client.
//! **Signals & state:** none; every shard is embedded at compile time.
//! **Invariants:** every `include_str!` is relative to this file and names a file of this area, so
//! the pins keep resolving wherever the area moves; a shard is never listed twice.

use frontend_test_support::source_shards::{production_shard, production_source};

/// The HTTP client, as one text: the error body readers, the refusal decoder, the refresh policy
/// and the request verbs.
pub(crate) fn client_source() -> String {
    production_source(&[
        include_str!("../client/mod.rs"),
        include_str!("../client/errors.rs"),
        include_str!("../client/fetched.rs"),
        include_str!("../client/refusals.rs"),
        include_str!("../client/refresh.rs"),
        include_str!("../client/requests.rs"),
    ])
}

/// The telemetry stream consumer.
pub(crate) fn sse_source() -> String {
    production_shard(include_str!("../sse.rs"))
}
