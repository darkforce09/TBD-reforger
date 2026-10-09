//! The runner on a fake clock: deadlines count from the observed request row, a miss or a
//! contradiction fails the case, the case mapping joins effects, the hard stop and host actions
//! end steps, and every observation is journaled.
use std::path::Path;

use serde_json::json;

use super::fake_clock::FakeClock;
use super::procedure::{ProcedurePlan, ProcedureRun};
use super::runner::{ProcedureRunner, RunContext};
use super::runner_support::{ScriptedHost, scratch_folder};
use super::step::{
    Deadline, DeclaredCase, EffectPredicate, Probe, ProbeVerdict, RequestPredicate, Step, StepId,
    StepKind,
};
use crate::observation_journal::browser_inbox::BrowserInbox;
use crate::observation_journal::journal::{JOURNAL_FILE, ObservationJournal};
use crate::remote_observers::remote_command::RemoteCommand;
use api_readiness_checks::operational_recording::{CaseName, CaseStatus};

const T0: u64 = 1_800_000_000_000;

fn seconds(n: u64) -> u64 {
    T0 + n * 1000
}

/// A database probe answered by the scripted host's recording for `marker`: `done` holds,
/// `wrong` contradicts, anything else is pending.
fn probe(marker: &'static str) -> Probe {
    Probe::host(
        move |_| Ok(RemoteCommand::read("database", format!("psql {marker}"))),
        |text, _| match text.trim() {
            "done" => ProbeVerdict::Satisfied(ProbeVerdict::satisfied("done").measure("seen", 1)),
            "wrong" => ProbeVerdict::Contradicted("the unit is in the wrong state".into()),
            other => ProbeVerdict::Pending(format!("saw {other:?}")),
        },
    )
}

/// A request row answered as `requested_at=<unix ms>`.
fn request() -> RequestPredicate {
    RequestPredicate {
        description: "the stop command row".into(),
        probe: Probe::host(
            |_| Ok(RemoteCommand::read("database", "psql request-row".into())),
            |text, _| match text.trim().strip_prefix("requested_at=") {
                Some(at) => ProbeVerdict::Satisfied(
                    ProbeVerdict::satisfied("row found").at(at.parse().unwrap()),
                ),
                None => ProbeVerdict::Pending("no row".into()),
            },
        ),
    }
}

fn effect(id: &str, case: &str, deadline: Deadline, marker: &'static str) -> EffectPredicate {
    EffectPredicate {
        id: id.into(),
        description: format!("effect {id}"),
        probe: probe(marker),
        deadline,
        case: CaseName::new(case).unwrap(),
    }
}

fn step(id: &str, request: Option<RequestPredicate>, effects: Vec<EffectPredicate>) -> Step {
    Step {
        id: StepId::new(id).unwrap(),
        kind: StepKind::ChromeAction,
        instruction: format!("do {id}"),
        request,
        effects,
    }
}

fn plan(cases: Vec<DeclaredCase>, steps: Vec<Step>, hard_stop_seconds: u64) -> ProcedurePlan {
    ProcedurePlan {
        declared_cases: cases,
        steps,
        hard_stop_seconds,
        poll_interval_seconds: 5,
    }
}

fn run(
    plan: &ProcedurePlan,
    host: &mut ScriptedHost,
    clock: &FakeClock,
    folder: &Path,
) -> (ProcedureRun, String) {
    let mut journal = ObservationJournal::create(folder).unwrap();
    let inbox = BrowserInbox::new(&folder.join("browser_inbox"));
    let mut output = Vec::new();
    let run = ProcedureRunner::new(
        plan,
        RunContext {
            host,
            clock,
            journal: &mut journal,
            inbox: &inbox,
            output: &mut output,
        },
    )
    .run()
    .unwrap();
    (run, String::from_utf8(output).unwrap())
}

fn status(run: &ProcedureRun, case: &str) -> CaseStatus {
    run.cases
        .iter()
        .find(|recorded| recorded.name.as_str() == case)
        .unwrap()
        .status
        .clone()
}

#[test]
fn staging_runner_counts_deadlines_from_the_request_row() {
    let clock = FakeClock::starting_at(T0);
    // The request row appears ten minutes into the step, stamped 590 s in; the effect holds at
    // 700 s: late for the step's start, inside 150 s of the row.
    let mut host = ScriptedHost::new(&clock)
        .answer("request-row", T0, 0, "")
        .answer(
            "request-row",
            seconds(600),
            0,
            &format!("requested_at={}", seconds(590)),
        )
        .answer("unit-stopped", T0, 0, "active")
        .answer("unit-stopped", seconds(700), 0, "done");
    let plan = plan(
        vec![DeclaredCase::runs("server1_stop").unwrap()],
        vec![step(
            "w1_stop",
            Some(request()),
            vec![effect(
                "server1",
                "server1_stop",
                Deadline::from_request_row(150),
                "unit-stopped",
            )],
        )],
        6_900,
    );
    let folder = scratch_folder("runner-request-row");
    let (run, output) = run(&plan, &mut host, &clock, &folder);
    assert_eq!(status(&run, "server1_stop"), CaseStatus::Ok, "{output}");
    assert!(
        output.starts_with("AWAIT w1_stop: do w1_stop\n"),
        "{output}"
    );
    assert!(output.contains("w1_stop.server1 ok (done)"), "{output}");
    assert_eq!(run.measurements.get("seen"), Some(&json!(1)));
    assert!(
        host.calls.iter().all(|call| call.purpose
            == super::super::remote_observers::remote_command::CommandPurpose::Read)
    );
}

#[test]
fn staging_runner_fails_a_case_whose_effect_misses_its_deadline() {
    let clock = FakeClock::starting_at(T0);
    let mut host = ScriptedHost::new(&clock)
        .answer(
            "request-row",
            T0,
            0,
            &format!("requested_at={}", seconds(10)),
        )
        .answer("unit-stopped", T0, 0, "active")
        .answer("unit-stopped", seconds(800), 0, "done");
    let plan = plan(
        vec![DeclaredCase::runs("server1_stop").unwrap()],
        vec![step(
            "w1_stop",
            Some(request()),
            vec![effect(
                "server1",
                "server1_stop",
                Deadline::from_request_row(150),
                "unit-stopped",
            )],
        )],
        6_900,
    );
    let folder = scratch_folder("runner-miss");
    let (run, output) = run(&plan, &mut host, &clock, &folder);
    let CaseStatus::Failed(why) = status(&run, "server1_stop") else {
        panic!("a missed deadline must fail the case: {output}");
    };
    assert!(
        why.contains("w1_stop.server1") && why.contains("deadline passed"),
        "{why}"
    );
    assert!(
        why.contains("\"active\""),
        "the reason names what was last seen: {why}"
    );
    assert!(output.contains("w1_stop.server1 FAILED"), "{output}");
    assert!(
        clock_reached(&clock, 160) && !clock_reached(&clock, 200),
        "the runner stops polling at the deadline"
    );
}

fn clock_reached(clock: &FakeClock, second: u64) -> bool {
    use time_source::Clock as _;
    clock.now_unix_ms() >= seconds(second)
}

#[test]
fn staging_runner_fails_every_effect_when_the_request_never_appears() {
    let clock = FakeClock::starting_at(T0);
    let mut host = ScriptedHost::new(&clock)
        .answer("request-row", T0, 0, "")
        .answer("unit-stopped", T0, 0, "done");
    let plan = plan(
        vec![
            DeclaredCase::runs("server1_stop").unwrap(),
            DeclaredCase::runs("server2_stop").unwrap(),
        ],
        vec![step(
            "w1_stop",
            Some(request()),
            vec![
                effect(
                    "server1",
                    "server1_stop",
                    Deadline::from_request_row(150),
                    "unit-stopped",
                ),
                effect(
                    "server2",
                    "server2_stop",
                    Deadline::from_request_row(150),
                    "unit-stopped",
                ),
            ],
        )],
        6_900,
    );
    let folder = scratch_folder("runner-no-request");
    let (run, _) = run(&plan, &mut host, &clock, &folder);
    for case in ["server1_stop", "server2_stop"] {
        let CaseStatus::Failed(why) = status(&run, case) else {
            panic!("{case} must fail")
        };
        assert!(
            why.contains("the stop command row") && why.contains("900 s"),
            "{why}"
        );
    }
    assert!(clock_reached(&clock, 900));
}

#[test]
fn staging_runner_maps_effects_to_cases() {
    let clock = FakeClock::starting_at(T0);
    let mut host = ScriptedHost::new(&clock)
        .answer("first", T0, 0, "done")
        .answer("second", T0, 0, "wrong")
        .answer("third", T0, 0, "done");
    let plan = plan(
        vec![
            DeclaredCase::runs("joined").unwrap(),
            DeclaredCase::runs("alone").unwrap(),
            DeclaredCase::not_run("two_clients", "second game client").unwrap(),
        ],
        vec![Step {
            kind: StepKind::Observation,
            ..step(
                "w2_start",
                None,
                vec![
                    effect("first", "joined", Deadline::from_step_start(60), "first"),
                    effect("second", "joined", Deadline::from_step_start(60), "second"),
                    effect("third", "alone", Deadline::from_step_start(60), "third"),
                ],
            )
        }],
        6_900,
    );
    let folder = scratch_folder("runner-mapping");
    let (run, _) = run(&plan, &mut host, &clock, &folder);
    let CaseStatus::Failed(why) = status(&run, "joined") else {
        panic!("one contradicted effect fails the case")
    };
    assert!(
        why.contains("w2_start.second") && why.contains("wrong state"),
        "{why}"
    );
    assert_eq!(status(&run, "alone"), CaseStatus::Ok);
    assert_eq!(
        status(&run, "two_clients"),
        CaseStatus::NotRun {
            missing: "second game client".into()
        }
    );
    let names: Vec<&str> = run.cases.iter().map(|case| case.name.as_str()).collect();
    assert_eq!(
        names,
        ["joined", "alone", "two_clients"],
        "declaration order"
    );
}

#[test]
fn staging_runner_stops_at_the_hard_stop() {
    let clock = FakeClock::starting_at(T0);
    let mut host = ScriptedHost::new(&clock).answer("never", T0, 0, "pending");
    let plan = plan(
        vec![
            DeclaredCase::runs("first").unwrap(),
            DeclaredCase::runs("second").unwrap(),
        ],
        vec![
            step(
                "w1",
                None,
                vec![effect(
                    "a",
                    "first",
                    Deadline::from_step_start(600),
                    "never",
                )],
            ),
            step(
                "w2",
                None,
                vec![effect(
                    "b",
                    "second",
                    Deadline::from_step_start(600),
                    "never",
                )],
            ),
        ],
        100,
    );
    let folder = scratch_folder("runner-hard-stop");
    let (run, output) = run(&plan, &mut host, &clock, &folder);
    let CaseStatus::Failed(first) = status(&run, "first") else {
        panic!("{output}")
    };
    assert!(first.contains("hard stop came"), "{first}");
    let CaseStatus::Failed(second) = status(&run, "second") else {
        panic!("{output}")
    };
    assert!(
        second.contains("not reached") && second.contains("100 s hard stop"),
        "{second}"
    );
    assert!(
        !output.contains("AWAIT w2"),
        "a step past the hard stop is not announced: {output}"
    );
}

#[test]
fn staging_runner_fails_a_step_whose_host_action_fails_and_refuses_a_changing_probe() {
    let clock = FakeClock::starting_at(T0);
    let mut host = ScriptedHost::new(&clock).answer("arm-relay", T0, 2, "socket missing");
    let mut failing = step(
        "w13_arm",
        None,
        vec![effect(
            "claim",
            "lost_claim",
            Deadline::from_step_start(60),
            "never",
        )],
    );
    failing.kind = StepKind::HostAction(RemoteCommand::change("relay control", "arm-relay".into()));
    let changing = Step {
        effects: vec![EffectPredicate {
            probe: Probe::host(
                |_| Ok(RemoteCommand::change("database", "psql delete".into())),
                |_, _| ProbeVerdict::Satisfied(ProbeVerdict::satisfied("never judged")),
            ),
            ..effect("write", "changing", Deadline::from_step_start(60), "unused")
        }],
        ..step("w14", None, Vec::new())
    };
    let plan = plan(
        vec![
            DeclaredCase::runs("lost_claim").unwrap(),
            DeclaredCase::runs("changing").unwrap(),
        ],
        vec![failing, changing],
        6_900,
    );
    let folder = scratch_folder("runner-host-action");
    let (run, _) = run(&plan, &mut host, &clock, &folder);
    let CaseStatus::Failed(why) = status(&run, "lost_claim") else {
        panic!("the action failed")
    };
    assert!(why.contains("the host action exited 2"), "{why}");
    let CaseStatus::Failed(why) = status(&run, "changing") else {
        panic!("a changing read is refused")
    };
    assert!(why.contains("would change the host"), "{why}");
    assert_eq!(
        host.calls.len(),
        1,
        "the changing read never reached the host"
    );
    assert_eq!(
        run.journal.len(),
        1,
        "the host action is a deciding observation"
    );
}

#[test]
fn staging_runner_judges_browser_inbox_entries_only_inside_the_window_and_journals_every_observation()
 {
    let clock = FakeClock::starting_at(T0);
    let mut host = ScriptedHost::new(&clock).answer("done-marker", T0, 0, "done");
    let folder = scratch_folder("runner-inbox");
    let inbox_folder = folder.join("browser_inbox");
    std::fs::create_dir_all(&inbox_folder).unwrap();
    // A capture from before the step is never accepted.
    std::fs::write(
        inbox_folder.join("w4_banner.json"),
        json!({ "captured_at_unix_ms": T0 - 1, "output": "Membership data is stale" }).to_string(),
    )
    .unwrap();
    let banner = EffectPredicate {
        probe: Probe::browser_inbox(|text, _| {
            if text.contains("stale") {
                ProbeVerdict::Satisfied(ProbeVerdict::satisfied("banner shown"))
            } else {
                ProbeVerdict::Pending("no banner".into())
            }
        }),
        ..effect(
            "banner",
            "staleness_warning",
            Deadline::from_step_start(30),
            "unused",
        )
    };
    let plan = plan(
        vec![
            DeclaredCase::runs("staleness_warning").unwrap(),
            DeclaredCase::runs("events_read").unwrap(),
        ],
        vec![step(
            "w4_banner",
            None,
            vec![
                banner,
                effect(
                    "events",
                    "events_read",
                    Deadline::from_step_start(30),
                    "done-marker",
                ),
            ],
        )],
        6_900,
    );
    let (stale, output) = run(&plan, &mut host, &clock, &folder);
    assert!(
        matches!(status(&stale, "staleness_warning"), CaseStatus::Failed(_)),
        "{output}"
    );
    assert!(output.contains("browser inbox: save"), "{output}");
    // The same step with a capture inside its window.
    let clock = FakeClock::starting_at(T0);
    let mut host = ScriptedHost::new(&clock).answer("done-marker", T0, 0, "done");
    let folder = scratch_folder("runner-inbox-accepted");
    let inbox_folder = folder.join("browser_inbox");
    std::fs::create_dir_all(&inbox_folder).unwrap();
    std::fs::write(
        inbox_folder.join("w4_banner.json"),
        json!({ "captured_at_unix_ms": T0 + 1, "output": "Membership data is stale" }).to_string(),
    )
    .unwrap();
    let (accepted, _) = run(&plan, &mut host, &clock, &folder);
    assert_eq!(status(&accepted, "staleness_warning"), CaseStatus::Ok);
    let journal = std::fs::read_to_string(folder.join(JOURNAL_FILE)).unwrap();
    assert_eq!(journal.lines().count(), 2, "{journal}");
    assert!(
        journal.contains("\"observer\":\"chrome page read\""),
        "{journal}"
    );
    assert_eq!(
        accepted.journal.len(),
        2,
        "both deciding observations reach the log"
    );
}
