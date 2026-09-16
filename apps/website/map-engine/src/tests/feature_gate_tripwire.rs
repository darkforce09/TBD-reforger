//! Role: feature gate tripwire.
//! Position: `tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

#[test]
#[expect(
    clippy::assertions_on_constants,
    reason = "Compile-time feature floor is the test subject"
)]
fn map_engine_core_tests_require_doc_feature() {
    assert!(
        cfg!(feature = "render")
            && cfg!(feature = "world")
            && cfg!(feature = "io")
            && cfg!(feature = "streaming")
            && cfg!(feature = "bvh"),
        "website-map-engine tests require --all-features to include every graphics suite"
    );
}
