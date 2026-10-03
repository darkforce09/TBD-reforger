use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use crate::Run;
use verification_core::NotRun;

#[test]
fn stream_lines_delivers_both_streams_and_the_exit_code() {
    let (child, lines) = Run::new("sh")
        .arg("-c")
        .arg("echo one; echo two >&2; printf 'three\\r\\n'; printf 'four'; exit 6")
        .stream_lines()
        .unwrap();
    let code = child.wait().unwrap();
    let mut got: Vec<String> = lines.iter().collect();
    got.sort();
    assert_eq!(got, ["four", "one", "three", "two"]);
    assert_eq!(code, 6);
}

#[test]
fn stream_lines_decodes_invalid_utf8_lossily() {
    let (child, lines) = Run::new("sh")
        .arg("-c")
        .arg("printf 'a\\377b\\n'")
        .stream_lines()
        .unwrap();
    child.wait().unwrap();
    let got: Vec<String> = lines.iter().collect();
    assert_eq!(got, ["a\u{fffd}b"]);
}

#[test]
fn kill_reaches_the_grandchild_and_closes_the_channel() {
    // `sleep` holds the pipes too: a kill of the direct child alone would leave the channel open.
    let (mut child, lines) = Run::new("sh")
        .arg("-c")
        .arg("sleep 30 & echo started; wait")
        .stream_lines()
        .unwrap();
    assert_eq!(
        lines.recv_timeout(Duration::from_secs(10)).unwrap(),
        "started"
    );
    child.kill();
    let deadline = Instant::now() + Duration::from_secs(10);
    let outcome = loop {
        match child.try_wait() {
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            other => break other,
        }
    };
    assert!(matches!(outcome, Err(NotRun::Signalled { signal: 9, .. })));
    assert_eq!(
        lines.recv_timeout(Duration::from_secs(10)),
        Err(RecvTimeoutError::Disconnected)
    );
    child.kill(); // after the reap: a no-op, never a signal to a reused pid
}

#[test]
fn stream_lines_enforces_its_timeout() {
    let (child, _lines) = Run::new("sleep")
        .arg("30")
        .timeout(Duration::from_millis(200))
        .stream_lines()
        .unwrap();
    assert!(matches!(child.wait(), Err(NotRun::Timeout { .. })));
}

#[test]
fn stream_lines_reports_an_absent_program() {
    assert!(matches!(
        Run::new("tbd-not-real-2b44").stream_lines(),
        Err(NotRun::ToolAbsent(_))
    ));
}
