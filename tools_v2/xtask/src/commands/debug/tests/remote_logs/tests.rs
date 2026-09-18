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
