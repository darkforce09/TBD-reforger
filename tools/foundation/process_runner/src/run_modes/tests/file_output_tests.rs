use std::fs::File;
use std::time::Duration;

use crate::Run;
use verification_core::NotRun;

/// Two fresh scratch files for `name`, and their paths.
fn sinks(name: &str) -> (std::path::PathBuf, std::path::PathBuf, File, File) {
    let base = std::env::temp_dir().join(format!("tbd-pr-files-{}-{name}", std::process::id()));
    let out = base.with_extension("out");
    let err = base.with_extension("err");
    let out_file = File::create(&out).unwrap();
    let err_file = File::create(&err).unwrap();
    (out, err, out_file, err_file)
}

#[test]
fn output_to_files_writes_each_stream_to_its_file() {
    let (out, err, out_file, err_file) = sinks("streams");
    let code = Run::new("sh")
        .arg("-c")
        .arg("printf '\\377\\000bin'; echo diag >&2; exit 4")
        .output_to_files(out_file, err_file)
        .unwrap();
    let stdout = std::fs::read(&out).unwrap();
    let stderr = std::fs::read_to_string(&err).unwrap();
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&err);
    assert_eq!(code, 4);
    assert_eq!(stdout, b"\xff\x00bin");
    assert_eq!(stderr, "diag\n");
}

#[test]
fn output_to_files_gives_the_child_a_null_stdin() {
    let (out, err, out_file, err_file) = sinks("stdin");
    let code = Run::new("cat")
        .timeout(Duration::from_secs(10))
        .output_to_files(out_file, err_file)
        .unwrap();
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&err);
    assert_eq!(code, 0, "cat must see EOF, not this process's terminal");
}

#[test]
fn output_to_files_reports_signals_and_timeouts_honestly() {
    let (out, err, out_file, err_file) = sinks("signal");
    assert!(matches!(
        Run::new("sh")
            .arg("-c")
            .arg("kill -9 $$")
            .output_to_files(out_file, err_file),
        Err(NotRun::Signalled { signal: 9, .. })
    ));
    let (out2, err2, out_file, err_file) = sinks("timeout");
    assert!(matches!(
        Run::new("sleep")
            .arg("30")
            .timeout(Duration::from_millis(200))
            .output_to_files(out_file, err_file),
        Err(NotRun::Timeout { .. })
    ));
    for path in [out, err, out2, err2] {
        let _ = std::fs::remove_file(path);
    }
}
