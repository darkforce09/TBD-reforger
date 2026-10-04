//! Source pins: the production text of the shared visual primitives, for the guard tests that read
//! it.
//!
//! **Role:** each function returns the full text of one logical source file of this area. Where
//! that file is split into shards, the function joins them in declaration order, so a guard that
//! reads the whole file still sees every definition exactly once.
//! **Position:** test-only, inside the area it reads; called from the UI primitives' guard tests.
//! **Signals & state:** none; every shard is embedded at compile time.
//! **Invariants:** every `include_str!` is relative to this file and names a file of this area, so
//! the pins keep resolving wherever the area moves; a shard is never listed twice.

use frontend_test_support::source_shards::production_source;

/// The shared visual primitives, as one text: the small components, the three form controls, the
/// two overlay surfaces, and the registry they share.
pub(crate) fn ui_source() -> String {
    production_source(&[
        include_str!("../lib.rs"),
        include_str!("../badge.rs"),
        include_str!("../dialog.rs"),
        include_str!("../icons.rs"),
        include_str!("../modal_stack.rs"),
        include_str!("../page_header.rs"),
        include_str!("../search_box.rs"),
        include_str!("../select.rs"),
        include_str!("../sheet.rs"),
        include_str!("../slider.rs"),
    ])
}
