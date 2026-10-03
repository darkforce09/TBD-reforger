use std::fs::File;
use std::time::{Duration, Instant};

use crate::{Run, wait_for};
use verification_core::NotRun;

fn scratch(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("tbd-pr-detached-{}-{name}", std::process::id()))
}

/// Field `index` (1-based) of `/proc/<pid>/stat`.
fn stat_field(pid: u32, index: usize) -> i64 {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    // The command name may hold spaces; the fields after its closing parenthesis do not.
    let after = &text[text.rfind(')').unwrap() + 2..];
    after.split(' ').nth(index - 3).unwrap().parse().unwrap()
}

#[test]
fn spawn_detached_returns_at_once_with_the_childs_pid() {
    let marker = scratch("marker");
    let started = Instant::now();
    let pid = Run::new("sh")
        .arg("-c")
        .arg("sleep 0.5; echo $$ > \"$OUT\"")
        .env("OUT", marker.display().to_string())
        .spawn_detached()
        .unwrap();
    assert!(
        started.elapsed() < Duration::from_millis(400),
        "nothing may wait on the child"
    );
    wait_for(
        "detached marker",
        Duration::from_secs(10),
        Duration::from_millis(20),
        || std::fs::read_to_string(&marker).is_ok_and(|text| text.ends_with('\n')),
    )
    .unwrap();
    let written: u32 = std::fs::read_to_string(&marker)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let _ = std::fs::remove_file(&marker);
    assert_eq!(written, pid);
}

#[test]
fn spawn_detached_leads_a_new_session() {
    let pid = Run::new("sleep").arg("2").spawn_detached().unwrap();
    // Field 6 is the session id: a session leader's equals its own pid.
    assert_eq!(stat_field(pid, 6), i64::from(pid));
    assert_eq!(
        stat_field(pid, 5),
        i64::from(pid),
        "and leads its own process group"
    );
}

#[test]
fn spawn_detached_to_files_writes_the_childs_output() {
    let log = scratch("log");
    let file = File::create(&log).unwrap();
    let clone = file.try_clone().unwrap();
    Run::new("sh")
        .arg("-c")
        .arg("echo out; echo err >&2; echo done")
        .spawn_detached_to_files(file, clone)
        .unwrap();
    wait_for(
        "detached log",
        Duration::from_secs(10),
        Duration::from_millis(20),
        || std::fs::read_to_string(&log).is_ok_and(|text| text.contains("done")),
    )
    .unwrap();
    let text = std::fs::read_to_string(&log).unwrap();
    let _ = std::fs::remove_file(&log);
    assert_eq!(text, "out\nerr\ndone\n");
}

#[test]
fn spawn_detached_reports_an_absent_program() {
    assert!(matches!(
        Run::new("tbd-not-real-5a10").spawn_detached(),
        Err(NotRun::ToolAbsent(_))
    ));
}
