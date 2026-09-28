use super::*;

fn fixture(name: &str, lines: &[&str]) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("t855-{name}-{}", std::process::id()));
    write_log(&p, lines);
    p
}

#[test]
fn selftest_pass() {
    assert_eq!(cmd_selftest(), 0);
}

#[test]
fn healthy_is_partial() {
    let p = fixture(
        "healthy",
        &[
            "SCRIPT : [TBD][Mission] loaded id=msn_x name='N' slots=7 source=profile",
            "SCRIPT : [TBD][Slots] Slot-1 s (a:b:c:0) kit kit:x at <1, 2, 3>",
            "SCRIPT : [TBD][Loadout][Slot] slot=a:b:c:0 loadout pass complete gear=1/1 cargo=0/0",
            "SCRIPT : [TBD][Stage] LOADING -> LOBBY",
        ],
    );
    assert_eq!(check_log_quiet(&p), 2);
    let _ = fs::remove_file(&p);
}

#[test]
fn stale_fails() {
    let p = fixture(
        "stale",
        &[
            "SCRIPT : [TBD] Mission loaded from backend: something",
            "SCRIPT : [TBD] SpawnManager: built slot spawn",
            "SCRIPT : [TBD] Stage → LOBBY",
        ],
    );
    assert_eq!(check_log_quiet(&p), 1);
    let _ = fs::remove_file(&p);
}

#[test]
fn missing_file_is_environment() {
    let p = PathBuf::from("/tmp/t855-no-such-log-file-ever");
    assert_eq!(check_log(&p), 3);
}

#[test]
fn errors_present_fail() {
    let p = fixture(
        "errs",
        &[
            "SCRIPT : [TBD][Mission] loaded id=msn_x name='N' slots=7 source=profile",
            "SCRIPT : [TBD][Slots] Slot-1 s (a:b:c:0) kit kit:x at <1, 2, 3>",
            "SCRIPT : [TBD][Stage] LOADING -> LOBBY",
            "SCRIPT : Can't compile SomeClass",
        ],
    );
    assert_eq!(check_log_quiet(&p), 1);
    let _ = fs::remove_file(&p);
}

/// The newest-log script runs in bash against a scratch profile whose name needs quoting: it
/// skips a newer run without a `console.log`, prints the newest one that has it and exits 0, and
/// exits 1 when no run has one.
#[test]
fn newest_console_log_script_prints_the_newest_log_and_exits_zero() {
    let root = tempfile_dir("t-newest-console-log").expect("scratch dir");
    let profile = root.join("it's profile");
    let older = profile.join("logs/logs_2026-01-01_10-00-00");
    fs::create_dir_all(&older).expect("older run");
    write_log(&older.join("console.log"), &["line"]);
    fs::create_dir_all(profile.join("logs/logs_2026-01-01_11-00-00")).expect("newer run");
    let run = |script: String| {
        std::process::Command::new("bash")
            .arg("-c")
            .arg(script)
            .output()
            .expect("bash runs")
    };

    let found = run(super::execution::newest_console_log_script(
        profile.to_str().expect("utf-8"),
    ));
    assert_eq!(found.status.code(), Some(0), "{found:?}");
    assert_eq!(
        String::from_utf8_lossy(&found.stdout).trim(),
        older.join("console.log").to_str().expect("utf-8")
    );

    let empty = root.join("empty profile");
    fs::create_dir_all(&empty).expect("empty profile");
    let none = run(super::execution::newest_console_log_script(
        empty.to_str().expect("utf-8"),
    ));
    assert_eq!(none.status.code(), Some(1), "{none:?}");
    fs::remove_dir_all(&root).expect("clean up");
}
