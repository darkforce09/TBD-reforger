use std::time::{Duration, Instant};

use crate::Run;
use verification_core::NotRun;

/// A scratch file path unique to this test process and `name`.
fn scratch(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("tbd-pr-terminal-{}-{name}", std::process::id()))
}

#[test]
fn terminal_returns_the_raw_exit_code() {
    let code = Run::new("sh").arg("-c").arg("exit 3").terminal().unwrap();
    assert_eq!(code, 3, "raw exit codes must never be collapsed");
}

#[test]
fn terminal_reports_absent_tools_and_signals_honestly() {
    assert!(matches!(
        Run::new("tbd-not-real-7c21").terminal(),
        Err(NotRun::ToolAbsent(_))
    ));
    assert!(matches!(
        Run::new("sh").arg("-c").arg("kill -9 $$").terminal(),
        Err(NotRun::Signalled { signal: 9, .. })
    ));
}

#[test]
fn terminal_honours_cwd_env_and_a_stdin_file() {
    let out = scratch("cwd-env");
    let input = scratch("cwd-env-input");
    std::fs::write(&input, "fed\n").unwrap();
    let code = Run::new("sh")
        .arg("-c")
        .arg("{ pwd; echo \"$TBD_T\"; cat; } > \"$OUT\"")
        .cwd("/tmp")
        .env("TBD_T", "set")
        .env("OUT", out.display().to_string())
        .stdin_file(std::fs::File::open(&input).unwrap())
        .terminal()
        .unwrap();
    let text = std::fs::read_to_string(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&input);
    assert_eq!(code, 0);
    assert_eq!(text, "/tmp\nset\nfed\n");
}

#[test]
fn terminal_child_stays_in_the_callers_process_group() {
    // The terminal's Ctrl-C reaches the foreground group; a child moved out of it would survive
    // the operator's interrupt.
    let out = scratch("pgid");
    Run::new("sh")
        .arg("-c")
        .arg("cut -d' ' -f5 /proc/$$/stat > \"$OUT\"")
        .env("OUT", out.display().to_string())
        .stdin_null()
        .terminal()
        .unwrap();
    let child_group: i32 = std::fs::read_to_string(&out)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let _ = std::fs::remove_file(&out);
    // SAFETY: `getpgrp` cannot fail and touches no memory.
    #[allow(
        unsafe_code,
        reason = "the caller's process group has no safe std call"
    )]
    let own_group = unsafe { libc::getpgrp() };
    assert_eq!(child_group, own_group);
}

#[test]
fn terminal_times_out_by_killing_the_child() {
    let started = Instant::now();
    let got = Run::new("sleep")
        .arg("30")
        .timeout(Duration::from_millis(200))
        .terminal();
    assert!(matches!(got, Err(NotRun::Timeout { .. })));
    assert!(started.elapsed() < Duration::from_secs(10));
}
