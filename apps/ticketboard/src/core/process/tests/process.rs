use super::*;
use crate::test_support::Scratch;
use std::fs;

/// Executable-enough for resolution tests: the resolver checks `is_file`.
fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "#!/bin/sh\n").unwrap();
}

#[test]
fn cargo_env_wins_when_it_exists() {
    let s = Scratch::new("cargo-env");
    let cargo_bin = s.path().join("toolchain/cargo");
    touch(&cargo_bin);
    let path_cargo = s.path().join("onpath/cargo");
    touch(&path_cargo);
    let got = resolve_cargo_from(
        Some(cargo_bin.as_os_str()),
        Some(path_cargo.parent().unwrap().as_os_str()),
        Some(s.path()),
    );
    assert_eq!(got, cargo_bin);
}

#[test]
fn stale_cargo_env_falls_through_to_path() {
    let s = Scratch::new("cargo-stale");
    let path_cargo = s.path().join("onpath/cargo");
    touch(&path_cargo);
    let missing = s.path().join("gone/cargo");
    let got = resolve_cargo_from(
        Some(missing.as_os_str()),
        Some(path_cargo.parent().unwrap().as_os_str()),
        Some(s.path()),
    );
    assert_eq!(got, path_cargo);
}

#[test]
fn empty_cargo_env_falls_through_to_path() {
    let s = Scratch::new("cargo-empty");
    let path_cargo = s.path().join("onpath/cargo");
    touch(&path_cargo);
    let got = resolve_cargo_from(
        Some(OsStr::new("")),
        Some(path_cargo.parent().unwrap().as_os_str()),
        None,
    );
    assert_eq!(got, path_cargo);
}

#[test]
fn path_scan_takes_the_first_hit_in_order() {
    let s = Scratch::new("cargo-path-order");
    let first = s.path().join("a/cargo");
    let second = s.path().join("b/cargo");
    touch(&first);
    touch(&second);
    let joined = std::env::join_paths([
        s.path().join("empty-has-no-cargo"),
        first.parent().unwrap().to_path_buf(),
        second.parent().unwrap().to_path_buf(),
    ])
    .unwrap();
    let got = resolve_cargo_from(None, Some(&joined), None);
    assert_eq!(got, first);
}

#[test]
fn bare_gui_path_falls_back_to_home_cargo_bin() {
    let s = Scratch::new("cargo-home");
    let home_cargo = s.path().join(".cargo/bin/cargo");
    touch(&home_cargo);
    // A bare PATH (no rustup shims anywhere on it).
    let bare = s.path().join("usr-bin-without-cargo");
    fs::create_dir_all(&bare).unwrap();
    let got = resolve_cargo_from(None, Some(bare.as_os_str()), Some(s.path()));
    assert_eq!(got, home_cargo);
}

#[test]
fn nothing_found_yields_literal_cargo() {
    let s = Scratch::new("cargo-none");
    let empty = s.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    let got = resolve_cargo_from(None, Some(empty.as_os_str()), Some(s.path()));
    assert_eq!(got, PathBuf::from("cargo"));
    assert_eq!(resolve_cargo_from(None, None, None), PathBuf::from("cargo"));
}

#[test]
fn log_ring_keeps_the_last_cap_lines_and_counts_drops() {
    let mut ring = BoundedLog::new(3);
    assert!(ring.is_empty());
    for i in 0..5 {
        ring.push(format!("line {i}"));
    }
    assert_eq!(ring.len(), 3);
    assert_eq!(ring.dropped(), 2);
    let kept: Vec<&str> = ring.lines().collect();
    assert_eq!(kept, vec!["line 2", "line 3", "line 4"]);
    ring.clear();
    assert!(ring.is_empty());
    assert_eq!(ring.dropped(), 0);
}

#[cfg(unix)]
#[test]
fn streams_merged_lines_and_delivers_the_exit_code() {
    let s = Scratch::new("spawn-stream");
    let handle = spawn_streaming(
        "/bin/sh",
        &["-c", "echo out1; echo err1 >&2; echo out2; exit 3"],
        s.path(),
        || {},
    );
    let mut lines = Vec::new();
    let mut code = None;
    for ev in handle.rx.iter() {
        match ev {
            ProcessEvent::Line(l) => lines.push(l),
            ProcessEvent::Exited { code: c } => {
                code = Some(c);
                break;
            }
            ProcessEvent::SpawnFailed(e) => panic!("spawn failed: {e}"),
        }
    }
    assert_eq!(code, Some(Some(3)));
    lines.sort();
    assert_eq!(lines, vec!["err1", "out1", "out2"]);
}

#[cfg(unix)]
#[test]
fn kill_delivers_a_signal_exit() {
    let s = Scratch::new("spawn-kill");
    let handle = spawn_streaming("/bin/sh", &["-c", "sleep 30"], s.path(), || {});
    handle.kill();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        match handle.rx.recv_timeout(left) {
            Ok(ProcessEvent::Exited { code }) => {
                assert_eq!(code, None, "SIGKILL has no exit code");
                break;
            }
            Ok(_) => {}
            Err(e) => panic!("no Exited event after kill: {e}"),
        }
    }
}

#[test]
fn spawn_failure_is_an_event_not_a_panic() {
    let s = Scratch::new("spawn-enoent");
    let handle = spawn_streaming(
        s.path().join("no-such-binary-anywhere"),
        &[],
        s.path(),
        || {},
    );
    match handle.rx.recv_timeout(Duration::from_secs(5)) {
        Ok(ProcessEvent::SpawnFailed(e)) => assert!(!e.is_empty()),
        other => panic!("expected SpawnFailed, got {other:?}"),
    }
}
