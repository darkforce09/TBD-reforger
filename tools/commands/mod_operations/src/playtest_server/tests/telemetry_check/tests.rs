use super::*;

fn reading(backlog: u64) -> TelemetryQueueReading {
    TelemetryQueueReading {
        backlog,
        capacity: 512,
        dropped_total: 2,
        oldest_age_seconds: 7,
        reported_at: "2026-09-26T10:00:00Z".into(),
    }
}

fn status(match_id: Option<&str>, queue: Option<TelemetryQueueReading>) -> ServerTelemetryStatus {
    ServerTelemetryStatus {
        current_match_id: match_id.map(str::to_string),
        telemetry_queue: queue,
    }
}

fn observed(matches: &[&str], last: Option<TelemetryQueueReading>) -> TelemetryObservations {
    TelemetryObservations {
        matches_seen: matches.iter().map(|id| id.to_string()).collect(),
        last_reading: last,
        read_before_stop: true,
    }
}

#[test]
fn every_match_seen_is_remembered_once_and_the_latest_reading_wins() {
    let mut observations = TelemetryObservations::default();
    observations.record(&status(None, Some(reading(4))));
    observations.record(&status(Some("m-1"), None));
    observations.record(&status(Some("m-2"), Some(reading(1))));
    observations.record(&status(Some("m-1"), Some(reading(0))));
    assert_eq!(observations.matches_seen, vec!["m-1", "m-2"]);
    // A read without a reading keeps the stored one.
    observations.record(&status(None, None));
    assert_eq!(observations.last_reading, Some(reading(0)));
}

#[test]
fn a_drained_queue_with_events_for_every_match_passes() {
    let verdict = release_verdict(
        &observed(&["m-1", "m-2"], Some(reading(0))),
        &[("m-1".into(), Ok(true)), ("m-2".into(), Ok(true))],
    );
    assert_eq!(verdict, TelemetryVerdict::Passed);
}

#[test]
fn without_a_match_nothing_is_required_beyond_a_reading() {
    // A backlog with no match seen is not a failure: nothing was required of it.
    assert_eq!(
        release_verdict(&observed(&[], Some(reading(3))), &[]),
        TelemetryVerdict::NoMatchObserved
    );
    assert_eq!(
        release_verdict(&observed(&[], None), &[]),
        TelemetryVerdict::Failed(vec![
            "the runtime never reported a telemetry_queue reading".into()
        ])
    );
}

#[test]
fn a_match_without_events_or_with_a_backlog_fails_and_names_why() {
    let verdict = release_verdict(
        &observed(&["m-1", "m-2", "m-3", "m-4"], Some(reading(5))),
        &[
            ("m-1".into(), Ok(true)),
            ("m-2".into(), Ok(false)),
            ("m-3".into(), Err("GET … answered 404".into())),
        ],
    );
    assert_eq!(
        verdict,
        TelemetryVerdict::Failed(vec![
            "the last reading (2026-09-26T10:00:00Z) still holds a backlog of 5".into(),
            "match m-2 holds no acknowledged event".into(),
            "the events of match m-3 are unreadable: GET … answered 404".into(),
            "the events of match m-4 were not read".into(),
        ])
    );
}

#[test]
fn a_match_seen_without_any_reading_fails() {
    let verdict = release_verdict(&observed(&["m-1"], None), &[("m-1".into(), Ok(true))]);
    assert!(matches!(verdict, TelemetryVerdict::Failed(reasons) if reasons.len() == 1));
}

#[test]
fn a_failed_check_fails_a_successful_run_only_when_required() {
    let failed = TelemetryVerdict::Failed(vec!["x".into()]);
    assert_eq!(fold_exit_code(0, &failed, true), 1);
    assert_eq!(fold_exit_code(0, &failed, false), 0);
    // A run that already failed keeps its own code.
    assert_eq!(fold_exit_code(3, &failed, true), 3);
    assert_eq!(fold_exit_code(0, &TelemetryVerdict::Passed, true), 0);
    assert_eq!(
        fold_exit_code(0, &TelemetryVerdict::NoMatchObserved, true),
        0
    );
    assert_eq!(fold_exit_code(1, &TelemetryVerdict::Passed, true), 1);
}

#[test]
fn a_reading_is_described_on_one_line() {
    assert_eq!(
        describe_reading(&reading(3)),
        "backlog 3/512, dropped 2, oldest 7 s (reported 2026-09-26T10:00:00Z)"
    );
}

#[test]
fn the_drain_wait_continues_only_while_a_reading_holds_a_backlog() {
    assert!(!backlog_remains(&observed(&[], None)));
    assert!(!backlog_remains(&observed(&["m-1"], Some(reading(0)))));
    assert!(backlog_remains(&observed(&[], Some(reading(2)))));
}

#[test]
fn a_server_that_exited_on_its_own_is_not_judged_on_its_backlog() {
    // No pre-stop read: the server crashed, so the queue could not drain.
    let mut crashed = observed(&["m-1"], Some(reading(4)));
    crashed.read_before_stop = false;
    assert_eq!(
        release_verdict(&crashed, &[("m-1".into(), Ok(true))]),
        TelemetryVerdict::Passed
    );
    // ...but a crash with no reading ever observed still fails.
    crashed.last_reading = None;
    assert!(matches!(
        release_verdict(&crashed, &[("m-1".into(), Ok(true))]),
        TelemetryVerdict::Failed(_)
    ));
}
