use std::time::Duration;

use crate::Run;
use verification_core::NotRun;

// A successful replacement would end the test binary, so only the refusals are exercised here;
// `cargo xtask mod spawn-verify --selftest` runs the success path.

#[test]
fn replace_process_returns_absent_for_a_missing_program() {
    assert!(matches!(
        Run::new("tbd-not-real-9e03").replace_process(),
        NotRun::ToolAbsent(program) if program == "tbd-not-real-9e03"
    ));
}

#[test]
fn replace_process_refuses_a_stdin_body_and_a_timeout() {
    assert!(matches!(
        Run::new("true").stdin("body").replace_process(),
        NotRun::ToolError { .. }
    ));
    assert!(matches!(
        Run::new("true")
            .timeout(Duration::from_secs(1))
            .replace_process(),
        NotRun::ToolError { .. }
    ));
}
