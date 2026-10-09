use serde_json::json;

use super::*;

fn report(
    action: ProcessAction,
    systemctl: SystemctlObservation,
    dwell_seconds: u64,
    unit_state: Result<(&str, &str), &str>,
) -> ProcessActionReport {
    ProcessActionReport {
        action,
        unit: "tbd-reforger.service".to_owned(),
        systemctl,
        dwell: Duration::from_secs(dwell_seconds),
        unit_state: unit_state
            .map(|(load_state, active_state)| ObservedUnitState {
                load_state: load_state.to_owned(),
                active_state: active_state.to_owned(),
            })
            .map_err(str::to_owned),
    }
}

fn exited(code: i32) -> SystemctlObservation {
    SystemctlObservation::Exited {
        code: Some(code),
        error_output: String::new(),
    }
}

#[test]
fn start_and_restart_succeed_only_when_the_unit_is_active() {
    for action in [ProcessAction::Start, ProcessAction::Restart] {
        let verdict = report(action, exited(0), 8, Ok(("loaded", "active"))).verdict();
        assert!(verdict.succeeded(), "{verdict:?}");
        assert_eq!(
            serde_json::Value::Object(verdict.outcome().unwrap().clone()),
            json!({
                "unit": "tbd-reforger.service",
                "load_state": "loaded",
                "active_state": "active",
                "dwell_milliseconds": 8000,
                "systemctl": "exited with status 0",
            })
        );
        for state in ["failed", "inactive", "activating", "deactivating"] {
            let verdict = report(action, exited(0), 8, Ok(("loaded", state))).verdict();
            assert!(!verdict.succeeded());
            let reason = verdict.failure_reason().unwrap();
            assert!(
                reason.contains(&format!("tbd-reforger.service {state} instead of active")),
                "{reason}"
            );
            assert!(reason.contains("after waiting 8s"), "{reason}");
            assert_eq!(verdict.outcome().unwrap()["active_state"], state);
        }
    }
}

#[test]
fn a_server_that_exits_with_status_zero_after_starting_is_a_failure() {
    let verdict = report(
        ProcessAction::Restart,
        exited(0),
        8,
        Ok(("loaded", "failed")),
    )
    .verdict();
    assert!(!verdict.succeeded());
    assert_eq!(
        verdict.failure_reason(),
        Some(
            "restart left tbd-reforger.service failed instead of active after waiting 8s; \
             systemctl --user restart exited with status 0"
        )
    );
}

#[test]
fn stop_succeeds_only_when_the_unit_is_inactive() {
    let stopped = report(
        ProcessAction::Stop,
        exited(0),
        0,
        Ok(("loaded", "inactive")),
    )
    .verdict();
    assert!(stopped.succeeded());
    let still_running = report(
        ProcessAction::Stop,
        exited(0),
        0,
        Ok(("loaded", "deactivating")),
    );
    let verdict = still_running.verdict();
    assert_eq!(
        verdict.failure_reason(),
        Some(
            "stop left tbd-reforger.service deactivating instead of inactive; \
             systemctl --user stop exited with status 0"
        )
    );
}

#[test]
fn a_unit_that_is_not_loaded_is_never_a_success() {
    for load_state in ["not-found", "masked", "bad-setting", "error"] {
        let verdict = report(
            ProcessAction::Stop,
            exited(5),
            0,
            Ok((load_state, "inactive")),
        )
        .verdict();
        assert!(!verdict.succeeded(), "{load_state}");
        let reason = verdict.failure_reason().unwrap();
        assert!(
            reason.contains(&format!("the unit is not loaded (LoadState={load_state})")),
            "{reason}"
        );
    }
}

#[test]
fn the_exit_status_of_the_verb_does_not_decide_the_verdict() {
    let verdict = report(ProcessAction::Start, exited(1), 8, Ok(("loaded", "active"))).verdict();
    assert!(verdict.succeeded());
    assert_eq!(
        verdict.outcome().unwrap()["systemctl"],
        "exited with status 1"
    );
    let timed_out = SystemctlObservation::TimedOut {
        limit: Duration::from_secs(100),
    };
    let verdict = report(
        ProcessAction::Stop,
        timed_out,
        0,
        Ok(("loaded", "inactive")),
    )
    .verdict();
    assert!(verdict.succeeded());
    assert_eq!(
        verdict.outcome().unwrap()["systemctl"],
        "did not finish within 100s"
    );
}

#[test]
fn an_unreadable_state_is_a_failure_that_names_why() {
    let not_started = SystemctlObservation::NotStarted {
        error: "No such file or directory (os error 2)".to_owned(),
    };
    let verdict = report(
        ProcessAction::Start,
        not_started,
        8,
        Err("systemctl --user show --property=LoadState could not be started: No such file"),
    )
    .verdict();
    assert!(!verdict.succeeded());
    let reason = verdict.failure_reason().unwrap();
    assert!(
        reason.contains("the unit state could not be read"),
        "{reason}"
    );
    assert!(
        reason.contains("systemctl --user start could not be started"),
        "{reason}"
    );
    let outcome = verdict.outcome().unwrap();
    assert!(outcome.get("active_state").is_none());
    assert!(outcome.get("load_state").is_none());
}

#[test]
fn the_error_output_excerpt_is_one_short_line_without_control_characters() {
    assert_eq!(
        error_output_excerpt(b"\nFailed to start unit\tnow.\nSecond line\n"),
        "Failed to start unit now."
    );
    let long = "x".repeat(400);
    assert_eq!(
        error_output_excerpt(long.as_bytes()).len(),
        ERROR_OUTPUT_MAX_BYTES
    );
    let observation = SystemctlObservation::Exited {
        code: None,
        error_output: "killed".to_owned(),
    };
    assert_eq!(observation.describe(), "was ended by a signal: killed");
}
