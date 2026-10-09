use super::*;
use serde_json::json;
use time_source::ManualClock;

/// The instant every test clock reads, so the read-set ages are exact.
const TEST_NOW_UNIX_MS: u64 = 1_790_000_000_000;

fn test_clock() -> ManualClock {
    ManualClock::new(TEST_NOW_UNIX_MS)
}

// ---- a ranged read is always legal ----

#[test]
fn ranged_read_is_always_allowed_even_when_repeated() {
    let s = "test-ranged";
    let clock = test_clock();
    let _ = std::fs::remove_file(state_path(s));
    let inp = json!({"file_path": "/etc/hostname", "offset": 1, "limit": 10});
    assert!(guard_read(s, &inp, &clock).is_none());
    assert!(
        guard_read(s, &inp, &clock).is_none(),
        "ranged re-read must stay legal"
    );
}

/// Seed the state file with a read that happened `ms_ago` milliseconds before the clock's
/// reading, so re-read semantics are tested without sleeping.
fn seed_read(session: &str, path: &str, ms_ago: u64, clock: &dyn Clock) {
    let p = state_path(session);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(
        &p,
        format!("{}\t{path}\n", clock.now_unix_ms().saturating_sub(ms_ago)),
    )
    .unwrap();
}

// ---- the guard denies what it should ----

#[test]
fn whole_read_of_a_path_read_in_an_earlier_turn_is_denied() {
    let s = "test-reread";
    let clock = test_clock();
    let _ = std::fs::remove_file(state_path(s));
    let f = std::env::temp_dir().join("tbd-aiguard-reread.txt");
    std::fs::write(&f, "one\ntwo\n").unwrap();
    let inp = json!({ "file_path": f.to_str().unwrap() });
    assert!(guard_read(s, &inp, &clock).is_none(), "first read allowed");
    // A genuine re-read happens at least one model turn later.
    seed_read(s, f.to_str().unwrap(), SAME_CALL_WINDOW_MS + 1_000, &clock);
    assert!(
        guard_read(s, &inp, &clock).is_some(),
        "re-read from an earlier turn must be denied"
    );
}

/// With the hook registered at both user and project level, one tool call runs the guard twice
/// within milliseconds; the second run must not deny the first read of a file.
#[test]
fn hook_registered_twice_does_not_deny_a_first_read() {
    let s = "test-doublefire";
    let clock = test_clock();
    let _ = std::fs::remove_file(state_path(s));
    let f = std::env::temp_dir().join("tbd-aiguard-double.txt");
    std::fs::write(&f, "one\ntwo\n").unwrap();
    let inp = json!({ "file_path": f.to_str().unwrap() });
    assert!(
        guard_read(s, &inp, &clock).is_none(),
        "user-level hook allows"
    );
    assert!(
        guard_read(s, &inp, &clock).is_none(),
        "project-level hook fires for the SAME call and must also allow"
    );
    // ...and a third, still inside the window (three hook layers) is fine too.
    clock.advance(SAME_CALL_WINDOW_MS - 1);
    assert!(guard_read(s, &inp, &clock).is_none());
}

#[test]
fn large_whole_file_read_is_denied_but_ranged_is_not() {
    let s = "test-big";
    let clock = test_clock();
    let _ = std::fs::remove_file(state_path(s));
    let f = std::env::temp_dir().join("tbd-aiguard-big.txt");
    std::fs::write(&f, "x\n".repeat(BIG_FILE_LINES + 1)).unwrap();
    let whole = json!({ "file_path": f.to_str().unwrap() });
    assert!(guard_read(s, &whole, &clock).is_some());
    let ranged = json!({ "file_path": f.to_str().unwrap(), "limit": 50 });
    assert!(guard_read("test-big-2", &ranged, &clock).is_none());
}

// ---- the guard fails open ----

#[test]
fn missing_file_fails_open() {
    let s = "test-missing";
    let clock = test_clock();
    let _ = std::fs::remove_file(state_path(s));
    let inp = json!({"file_path": "/nonexistent/nope.rs"});
    assert!(
        guard_read(s, &inp, &clock).is_none(),
        "unreadable target must fail open"
    );
}
