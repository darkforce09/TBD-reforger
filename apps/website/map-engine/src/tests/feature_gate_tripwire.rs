//! Role: feature gate tripwire.
//! Position: `tests` in the map engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

#[test]
#[expect(
    clippy::assertions_on_constants,
    reason = "Compile-time feature floor is the test subject"
)]
fn map_engine_tests_require_all_features() {
    // T-0xx Phase 2A merged `website-mission-core`'s own tripwire (which asserted
    // `compiler && doc`) into this one. Two crates meant two floors; one crate gets one
    // assertion, and it names every axis. A merged crate with an unmerged tripwire re-opens
    // exactly the vacuous-pass hole `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs` and `wave/touch.rs` document:
    // a bare `cargo test -p website-map-engine` compiles almost none of these modules and
    // passes on code it never read.
    assert!(
        cfg!(feature = "render")
            && cfg!(feature = "world")
            && cfg!(feature = "io")
            && cfg!(feature = "streaming")
            && cfg!(feature = "bvh")
            && cfg!(feature = "scenario")
            && cfg!(feature = "store")
            && cfg!(feature = "editing"),
        "website-map-engine tests require --all-features to include every suite"
    );
}
