use super::*;

#[test]
fn native_hostrun_wraps_in_timeout_only() {
    let h = Host {
        bridge: false,
        timeout_secs: 1200,
    };
    assert_eq!(
        h.hostrun_argv(&v(&["cargo", "check"])),
        v(&["timeout", "1200", "cargo", "check"])
    );
}

#[test]
// `set_var` is `unsafe` in edition 2024; the environment lock below makes it sound here.
#[allow(unsafe_code)]
fn bridged_hostrun_forwards_the_whitelist_because_distrobox_does_not() {
    // MEASURED 2026-07-26: distrobox-host-exec does not forward the environment. If this
    // assertion ever loosens, every worktree silently builds its own 44 GB target dir.
    let _env = tool_test_support::lock_env();
    let previous_target_dir = std::env::var_os("CARGO_TARGET_DIR");
    // SAFETY: every test of this binary that changes the environment holds the lock taken above.
    unsafe { std::env::set_var("CARGO_TARGET_DIR", "/tmp/ctd") };
    unsafe { std::env::set_var("TEST_DATABASE_URL", "postgres://x") };
    let h = Host {
        bridge: true,
        timeout_secs: 60,
    };
    assert_eq!(
        h.hostrun_argv(&v(&["cargo", "test"])),
        v(&[
            "distrobox-host-exec",
            "timeout",
            "60",
            "env",
            "CARGO_TARGET_DIR=/tmp/ctd",
            "TEST_DATABASE_URL=postgres://x",
            "cargo",
            "test",
        ])
    );
    unsafe { std::env::remove_var("TEST_DATABASE_URL") };
    match previous_target_dir {
        Some(value) => unsafe { std::env::set_var("CARGO_TARGET_DIR", value) },
        None => unsafe { std::env::remove_var("CARGO_TARGET_DIR") },
    }
}

#[test]
fn checkrun_second_env_wins_over_the_baked_in_one() {
    let h = Host {
        bridge: false,
        timeout_secs: 5,
    };
    assert_eq!(
        h.checkrun_argv("/gate/check", &v(&["cargo", "clippy"])),
        v(&[
            "timeout",
            "5",
            "env",
            "CARGO_TARGET_DIR=/gate/check",
            "CARGO_INCREMENTAL=0",
            "cargo",
            "clippy",
        ])
    );
}

#[test]
fn timeout_rc_124_survives_as_124() {
    // The step runners branch on exactly this number.
    let (_, rc) = capture(&v(&["timeout", "1", "sleep", "5"]));
    assert_eq!(
        rc, 124,
        "timeout(1) must surface as 124, not as a generic failure"
    );
}
