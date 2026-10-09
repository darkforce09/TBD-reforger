use super::*;

// ---- the guard denies what it should ----

#[test]
fn uncapped_grep_is_denied() {
    assert!(guard_bash("rg 'fn place_at' apps/frontend/src").is_some());
    assert!(guard_bash("grep -rn TODO .").is_some());
}

#[test]
fn bare_file_read_is_denied() {
    assert!(
        guard_bash(
            "cat crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs"
        )
        .is_some()
    );
    assert!(guard_bash("sed -n '1,200p' tools/xtask/src/main.rs").is_some());
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
