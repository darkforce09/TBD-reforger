use super::*;

/// The exact SQL from Makefile:210. If this string moves, T-534's per-binary databases stop
/// being reaped and nobody notices until postgres runs out of them.
#[test]
fn reap_select_is_the_makefile_pattern() {
    assert_eq!(
        reap_select("rust_it"),
        "SELECT datname FROM pg_database WHERE datname = 'rust_it' OR datname LIKE 'rust_it\\_%\\_it' ESCAPE '\\'"
    );
}

/// The guard is the only thing between a stray env var and the dev database.
#[test]
fn the_guard_refuses_the_live_database() {
    // SAFETY: single-threaded assertion on this module's own knob.
    unsafe { std::env::set_var("TBD_IT_BASE_DB", "tbd_reforger") };
    let got = guarded_base();
    unsafe { std::env::remove_var("TBD_IT_BASE_DB") };
    let msg = got.expect_err("tbd_reforger must be refused");
    assert!(
        msg.contains("scratch allow-list"),
        "refusal must name the allow-list: {msg}"
    );
    assert!(msg.contains("tbd_reforger"));
    assert_eq!(
        guarded_base().unwrap(),
        "rust_it",
        "default must be rust_it"
    );
}

/// A red suite must still reap — the case the prune exists for.
#[test]
fn a_failing_suite_still_reports_its_own_rc() {
    assert_eq!(join_rc(101, 0), 101);
    assert_eq!(join_rc(0, 1), 1);
    assert_eq!(join_rc(0, 0), 0);
}
