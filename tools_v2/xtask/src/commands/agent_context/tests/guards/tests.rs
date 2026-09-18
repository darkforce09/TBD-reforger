use super::*;
use serde_json::json;

// ---- the guard denies what it should ----

#[test]
fn uncapped_grep_is_denied() {
    assert!(guard_bash("rg 'fn place_at' apps/website/frontend/src").is_some());
    assert!(guard_bash("grep -rn TODO .").is_some());
}

#[test]
fn bare_file_read_is_denied() {
    assert!(guard_bash("cat apps/website/frontend/src/v2/apps/editor/mission_editor.rs").is_some());
    assert!(guard_bash("sed -n '1,200p' tools_v2/xtask/src/main.rs").is_some());
}

// ---- the guard permits what it must (a guard that traps an agent is worse than the
// tokens it saves, so these are the load-bearing cases) ----

#[test]
fn capped_search_is_allowed() {
    assert!(guard_bash("rg 'fn place_at' src | head -50").is_none());
    assert!(guard_bash("rg -m 20 'fn place_at' src").is_none());
    assert!(guard_bash("grep --count TODO src").is_none());
}

#[test]
fn git_is_never_touched() {
    assert!(guard_bash("git log --grep='^wave [0-9]+ CLOSED' -1").is_none());
    assert!(guard_bash("git grep -n foo").is_none());
}

#[test]
fn grep_reading_from_a_pipe_is_allowed() {
    // Already bounded by whatever produced the stream.
    assert!(guard_bash("cargo test 2>&1 | grep FAILED").is_none());
}

#[test]
fn head_as_a_cap_is_allowed() {
    assert!(guard_bash("ls -la | head -20").is_none());
}

#[test]
fn ranged_read_is_always_allowed_even_when_repeated() {
    let s = "test-ranged";
    let _ = std::fs::remove_file(state_path(s));
    let inp = json!({"file_path": "/etc/hostname", "offset": 1, "limit": 10});
    assert!(guard_read(s, &inp).is_none());
    assert!(
        guard_read(s, &inp).is_none(),
        "ranged re-read must stay legal"
    );
}

/// Seed the state file with a read that happened `ms_ago` milliseconds ago, so re-read
/// semantics can be tested without sleeping.
fn seed_read(session: &str, path: &str, ms_ago: u128) {
    let p = state_path(session);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, format!("{}\t{path}\n", now_ms().saturating_sub(ms_ago))).unwrap();
}

#[test]
fn whole_read_of_a_path_read_in_an_earlier_turn_is_denied() {
    let s = "test-reread";
    let _ = std::fs::remove_file(state_path(s));
    let f = std::env::temp_dir().join("tbd-aiguard-reread.txt");
    std::fs::write(&f, "one\ntwo\n").unwrap();
    let inp = json!({ "file_path": f.to_str().unwrap() });
    assert!(guard_read(s, &inp).is_none(), "first read allowed");
    // A genuine re-read happens at least one model turn later.
    seed_read(s, f.to_str().unwrap(), SAME_CALL_WINDOW_MS + 1_000);
    assert!(
        guard_read(s, &inp).is_some(),
        "re-read from an earlier turn must be denied"
    );
}

/// THE REGRESSION THAT MOTIVATED THE TIME WINDOW: with the hook registered at both user
/// and project level, one tool call invokes the guard twice within milliseconds. Before the
/// window existed, the second invocation denied the first read of every file in the project.
#[test]
fn hook_registered_twice_does_not_deny_a_first_read() {
    let s = "test-doublefire";
    let _ = std::fs::remove_file(state_path(s));
    let f = std::env::temp_dir().join("tbd-aiguard-double.txt");
    std::fs::write(&f, "one\ntwo\n").unwrap();
    let inp = json!({ "file_path": f.to_str().unwrap() });
    assert!(guard_read(s, &inp).is_none(), "user-level hook allows");
    assert!(
        guard_read(s, &inp).is_none(),
        "project-level hook fires for the SAME call and must also allow"
    );
    // ...and a third, still inside the window (three hook layers) is fine too.
    assert!(guard_read(s, &inp).is_none());
}

#[test]
fn large_whole_file_read_is_denied_but_ranged_is_not() {
    let s = "test-big";
    let _ = std::fs::remove_file(state_path(s));
    let f = std::env::temp_dir().join("tbd-aiguard-big.txt");
    std::fs::write(&f, "x\n".repeat(BIG_FILE_LINES + 1)).unwrap();
    let whole = json!({ "file_path": f.to_str().unwrap() });
    assert!(guard_read(s, &whole).is_some());
    let ranged = json!({ "file_path": f.to_str().unwrap(), "limit": 50 });
    assert!(guard_read("test-big-2", &ranged).is_none());
}

#[test]
fn missing_file_fails_open() {
    let s = "test-missing";
    let _ = std::fs::remove_file(state_path(s));
    let inp = json!({"file_path": "/nonexistent/nope.rs"});
    assert!(
        guard_read(s, &inp).is_none(),
        "unreadable target must fail open"
    );
}

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
    assert!(is_noise("   Compiling website-frontend v0.1.0"));
    assert!(is_noise("test mission::places_entity ... ok"));
    assert!(!is_noise("test result: FAILED. 0 passed; 3 failed"));
    assert!(!is_noise("error: could not compile"));
}
