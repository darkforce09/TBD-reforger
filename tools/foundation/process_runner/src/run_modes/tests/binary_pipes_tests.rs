use std::time::Duration;

use crate::Run;
use verification_core::NotRun;

#[test]
fn binary_output_returns_stdout_byte_for_byte() {
    let input: Vec<u8> = vec![0xff, 0x00, 0xfe, b'\n', 0x80, 0x7f];
    let out = Run::new("cat")
        .stdin_bytes(input.clone())
        .binary_output()
        .unwrap();
    assert_eq!(out.stdout, input, "stdout must never be decoded");
    assert_eq!(out.code, 0);
}

#[test]
fn binary_output_keeps_stderr_and_the_raw_code() {
    let out = Run::new("sh")
        .arg("-c")
        .arg("printf 'x'; echo oops >&2; exit 5")
        .binary_output()
        .unwrap();
    assert_eq!(out.stdout, b"x");
    assert_eq!(out.stderr, "oops\n");
    assert_eq!(out.code, 5);
}

#[test]
fn binary_output_streams_a_large_body_without_deadlocking() {
    // `cat` answers while it reads: 4 MiB in fills the stdout pipe long before stdin is through,
    // which a write-then-read parent would deadlock on.
    let input: Vec<u8> = (0..4 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
    let out = Run::new("cat")
        .stdin_bytes(input.clone())
        .timeout(Duration::from_secs(60))
        .binary_output()
        .unwrap();
    assert_eq!(out.stdout.len(), input.len());
    assert!(out.stdout == input);
}

#[test]
fn binary_output_reports_signals_and_timeouts_honestly() {
    assert!(matches!(
        Run::new("sh").arg("-c").arg("kill -9 $$").binary_output(),
        Err(NotRun::Signalled { signal: 9, .. })
    ));
    assert!(matches!(
        Run::new("sleep")
            .arg("30")
            .timeout(Duration::from_millis(200))
            .binary_output(),
        Err(NotRun::Timeout { .. })
    ));
}
