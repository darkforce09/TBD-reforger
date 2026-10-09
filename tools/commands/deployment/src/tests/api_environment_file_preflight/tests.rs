//! The `.env` preflight: its probe, its verdicts, and that only a present file lets the rsync run.
use super::*;
use std::cell::Cell;

const REMOTE_DIR: &str = "/home/deploy/tbd/repo";

/// Runs the preflight over `probe_exit` and reports whether the rsync ran and what came back.
fn preflight(probe_exit: Result<i32, u8>) -> (bool, Result<(), u8>) {
    let rsync_ran = Cell::new(false);
    let result = rsync_only_when_present(probe_exit, REMOTE_DIR, || {
        rsync_ran.set(true);
        Ok(())
    });
    (rsync_ran.get(), result)
}

#[test]
fn a_present_file_lets_the_rsync_run() {
    assert_eq!(preflight(Ok(0)), (true, Ok(())));
}

/// The rsync's own failure is still the deploy's status once the file is proven present.
#[test]
fn a_failing_rsync_after_a_present_file_keeps_its_own_code() {
    let result = rsync_only_when_present(Ok(0), REMOTE_DIR, || Err(23));
    assert_eq!(result, Err(23));
}

#[test]
fn a_missing_file_refuses_before_the_rsync() {
    assert_eq!(preflight(Ok(20)), (false, Err(1)));
}

/// ssh's own failure (255), a shell that could not run the probe (127) and a probe that could not
/// be sent at all (the runner's code for a missing ssh program) each refuse before the rsync.
#[test]
fn a_transport_error_refuses_before_the_rsync() {
    assert_eq!(preflight(Ok(255)), (false, Err(1)));
    assert_eq!(preflight(Ok(127)), (false, Err(1)));
    assert_eq!(preflight(Ok(1)), (false, Err(1)));
    assert_eq!(preflight(Err(127)), (false, Err(127)));
}

#[test]
fn the_probe_exit_reads_as_present_missing_or_indeterminate() {
    assert_eq!(classify(0), ApiEnvironmentFile::Present);
    assert_eq!(classify(20), ApiEnvironmentFile::Missing);
    assert_eq!(classify(255), ApiEnvironmentFile::Indeterminate(255));
    assert_eq!(classify(12), ApiEnvironmentFile::Indeterminate(12));
}

/// The probe tests the file at the checkout's current API path for being a regular file and
/// readable, and changes nothing on the host.
#[test]
fn the_probe_tests_the_current_api_path_and_changes_nothing() {
    assert_eq!(
        probe_script(REMOTE_DIR),
        "if [ -f '/home/deploy/tbd/repo/crates/api/api_server/.env' ] && \
         [ -r '/home/deploy/tbd/repo/crates/api/api_server/.env' ]; then exit 0; fi; exit 20"
    );
    assert_eq!(
        remote_path(REMOTE_DIR),
        format!("{REMOTE_DIR}/{API_ENVIRONMENT_FILE}")
    );
}

/// Under a local shell the probe answers 0 for a readable file, and 20 for a missing file, a
/// folder in its place, an unreadable file and a missing checkout.
#[test]
fn the_probe_answers_each_host_state_under_a_local_shell() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = std::env::temp_dir().join(format!("tbd-env-preflight-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let checkout = scratch.join("repo");
    let file = checkout.join(API_ENVIRONMENT_FILE);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let probe = |dir: &std::path::Path| {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(probe_script(&dir.display().to_string()))
            .status()
            .expect("sh runs")
            .code()
    };
    assert_eq!(probe(&checkout), Some(20), "missing file");
    std::fs::create_dir(&file).unwrap();
    assert_eq!(probe(&checkout), Some(20), "a folder in its place");
    std::fs::remove_dir(&file).unwrap();
    std::fs::write(&file, "DATABASE_URL=postgres://\n").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(probe(&checkout), Some(0), "readable file");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let running_as_root = std::fs::read(&file).is_ok();
    if !running_as_root {
        assert_eq!(probe(&checkout), Some(20), "unreadable file");
    }
    assert_eq!(probe(&scratch.join("absent")), Some(20), "missing checkout");
    std::fs::remove_dir_all(&scratch).unwrap();
}

#[test]
fn only_a_present_verdict_continues() {
    assert!(report(ApiEnvironmentFile::Present, REMOTE_DIR).is_ok());
    assert_eq!(report(ApiEnvironmentFile::Missing, REMOTE_DIR), Err(1));
    assert_eq!(
        report(ApiEnvironmentFile::Indeterminate(255), REMOTE_DIR),
        Err(1)
    );
}

/// The refusal of a missing file prints a step that works on a fresh host, whose checkout (and so
/// its `.env.example`) only arrives with the rsync the refusal stops: the template goes over ssh
/// from this checkout, as the website deployment runbook's step 3 writes it. A host whose file
/// sits where the previous folder layout kept it is told to move it instead.
#[test]
fn the_missing_file_refusal_prints_a_step_that_works_on_a_fresh_host() {
    assert_eq!(
        missing_file_operator_step(REMOTE_DIR),
        "       Operator step: on a host that already holds the API's .env where the previous \
         folder layout kept it, move that file (and the .tools folder beside it) into \
         /home/deploy/tbd/repo/crates/api/api_server/, keeping the file's mode. On a fresh host, \
         copy the template from this checkout, then fill it in \
         (documentation/runbooks/website_deployment.md, step 3):\n         ssh <TBD_SSH_HOST> \
         'mkdir -p /home/deploy/tbd/repo/crates/api/api_server && install -m 600 /dev/stdin \
         /home/deploy/tbd/repo/crates/api/api_server/.env' < crates/api/api_server/.env.example\n       \
         Then rerun the deploy."
    );
    let runbook = std::fs::read_to_string(
        tool_test_support::test_repo_root().join("documentation/runbooks/website_deployment.md"),
    )
    .expect("the website deployment runbook reads");
    assert!(
        runbook.contains(
            "ssh <TBD_SSH_HOST> 'mkdir -p <TBD_REMOTE_DIR>/crates/api/api_server && install -m 600 \
             /dev/stdin <TBD_REMOTE_DIR>/crates/api/api_server/.env' < \
             crates/api/api_server/.env.example"
        ),
        "the runbook's step 3 no longer writes the command the refusal prints"
    );
}
