//! **Role:** the map engine's feature-floor tripwire: its test fails unless the test build has
//! every feature tier.
//! **Position:** `tests` in the map engine, declared from `lib.rs`.
//! **Signals & state:** none.
//! **Invariants:** the assertion names every feature the manifest declares, so a test build that
//! leaves one out fails instead of passing over modules it never compiled.

#[test]
#[expect(
    clippy::assertions_on_constants,
    reason = "Compile-time feature floor is the test subject"
)]
fn map_engine_tests_require_all_features() {
    // Every module of this crate sits behind a feature tier, so a bare `cargo test -p map_engine`
    // compiles almost none of them and passes on code it never read — the vacuous-pass hole
    // `tools/xtask/src/commands/platform/wave_execution/gate/gate_dispatch.rs` and
    // `tools/xtask/src/commands/platform/wave_execution/touch.rs` guard by passing
    // `--all-features`. One assertion names every tier.
    assert!(
        cfg!(feature = "render")
            && cfg!(feature = "world")
            && cfg!(feature = "streaming")
            && cfg!(feature = "editing"),
        "map_engine tests require --all-features to include every suite"
    );
}
