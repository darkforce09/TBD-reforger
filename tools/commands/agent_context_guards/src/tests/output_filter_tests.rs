use super::*;

// ---- the filter cannot hide a failure ----

#[test]
fn verdict_and_failure_lines_always_survive() {
    for l in [
        "GATE: FAIL",
        "SLICE GATE: PASS",
        "test result: FAILED. 1 passed; 1 failed",
        "thread 'x' panicked at src/lib.rs:1:1",
        "error[E0308]: mismatched types",
        "skip: db unavailable",
        "REFUSING to pass — resolved to NO crate",
    ] {
        assert!(is_load_bearing(l), "must never be filtered: {l}");
    }
}

#[test]
fn only_chatter_is_treated_as_noise() {
    assert!(is_noise("   Compiling frontend v0.1.0"));
    assert!(is_noise("test mission::places_entity ... ok"));
    assert!(!is_noise("test result: FAILED. 0 passed; 3 failed"));
    assert!(!is_noise("error: could not compile"));
}
