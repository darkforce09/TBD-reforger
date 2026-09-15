//! Role: feature gate.
//! Position: `tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

#[test]
#[expect(
    clippy::assertions_on_constants,
    reason = "Reject partial feature selections in the test runner"
)]
fn mission_tests_require_compiler_feature() {
    assert!(
        cfg!(feature = "compiler") && cfg!(feature = "doc"),
        "mission-core tests require --all-features"
    );
}
