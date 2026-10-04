//! Source pins: the production text of the live server panel, for the guard tests that read it.
//!
//! **Role:** each function returns the full text of one logical source file of this area. Where
//! that file is split into shards, the function joins them in declaration order, so a guard that
//! reads the whole file still sees every definition exactly once.
//! **Position:** test-only, inside the area it reads; called from the server intel page's guard
//! tests.
//! **Signals & state:** none; every shard is embedded at compile time.
//! **Invariants:** every `include_str!` is relative to this file and names a file of this area, so
//! the pins keep resolving wherever the area moves; a shard is never listed twice.

use frontend_test_support::source_shards::production_source;

/// The live server panel, as one text: the route component, the connect header, the telemetry
/// grid, and the picker and shell that compose them.
pub(crate) fn server_intel_source() -> String {
    production_source(&[
        include_str!("../server_intel/mod.rs"),
        include_str!("../server_intel/page.rs"),
        include_str!("../server_intel/direct_connect.rs"),
        include_str!("../server_intel/player_census.rs"),
        include_str!("../server_intel/server_list.rs"),
    ])
}
