//! The Discord procedure's eleven steps on the fake clock against a recorded run: the recorded
//! run holds the eleven runnable cases in time and leaves the two subject-account cases not run,
//! and each planted defect fails only the case it breaks, naming what was observed.
use super::DiscordProcedure;
use super::discord_cases::{MISSING_TEST_SUBJECT, scenarios};
use super::recorded_discord::{Defect, RecordedDiscord, T0, discord_settings};
use crate::observation_journal::browser_inbox::BrowserInbox;
use crate::observation_journal::journal::ObservationJournal;
use crate::procedure_runner::fake_clock::FakeClock;
use crate::procedure_runner::procedure::{ProcedureRun, StagingProcedure};
use crate::procedure_runner::runner::RunContext;
use crate::procedure_runner::runner_support::scratch_folder;
use crate::remote_observers::remote_command::CommandPurpose;
use api_readiness_checks::operational_recording::CaseStatus;

/// Runs the Discord plan against a recorded run with `defect`: the run, the printed lines and the
/// host-changing commands in the order they ran.
fn run(defect: Option<Defect>) -> (ProcedureRun, String, Vec<String>) {
    let settings = discord_settings();
    let plan = DiscordProcedure.plan(&settings).unwrap();
    plan.validate().unwrap();
    let clock = FakeClock::starting_at(T0);
    let folder = scratch_folder("discord-run");
    let inbox_folder = folder.join("browser_inbox");
    let inbox = BrowserInbox::new(&inbox_folder);
    let mut host = RecordedDiscord::new(&settings, &clock, move || inbox_folder.clone(), defect);
    let mut journal = ObservationJournal::create(&folder).unwrap();
    let mut output = Vec::new();
    let run = DiscordProcedure
        .run(
            &plan,
            RunContext {
                host: &mut host,
                clock: &clock,
                journal: &mut journal,
                inbox: &inbox,
                output: &mut output,
            },
        )
        .unwrap();
    let _ = std::fs::remove_dir_all(&folder);
    let changes = host
        .calls
        .iter()
        .filter(|command| command.purpose != CommandPurpose::Read)
        .map(|command| command.observer.to_string())
        .collect();
    (run, String::from_utf8(output).unwrap(), changes)
}

/// Every runnable case but `failed` is ok, the two subject-account cases are not run, and the
/// failed case's reason holds `why`.
fn only_failed(defect: Defect, failed: &str, why: &str) {
    let (run, output, _) = run(Some(defect));
    for case in &run.cases {
        let name = case.name.as_str();
        match (&case.status, name) {
            (CaseStatus::NotRun { missing }, "role_demotion" | "departure_to_guest") => {
                assert_eq!(missing, MISSING_TEST_SUBJECT);
            }
            (CaseStatus::Failed(reason), _) if name == failed => {
                assert!(
                    reason.contains(why),
                    "{defect:?}: {name} failed for {reason}"
                );
            }
            (CaseStatus::Ok, _) if name != failed => {}
            (status, _) => panic!("{defect:?}: {name} is {status:?}\n{output}"),
        }
    }
    assert!(
        !scenarios(&run.cases)
            .iter()
            .any(|scenario| scenario == failed)
    );
}

#[test]
fn staging_discord_recorded_run_holds_eleven_cases_and_leaves_two_not_run() {
    let (run, output, changes) = run(None);
    assert_eq!(run.cases.len(), 13);
    let ok = run
        .cases
        .iter()
        .filter(|case| case.status == CaseStatus::Ok)
        .count();
    assert_eq!(ok, 11, "{:?}\n{output}", run.cases);
    for name in ["role_demotion", "departure_to_guest"] {
        let case = run
            .cases
            .iter()
            .find(|case| case.name.as_str() == name)
            .unwrap();
        assert_eq!(
            case.status,
            CaseStatus::NotRun {
                missing: MISSING_TEST_SUBJECT.to_string()
            }
        );
    }
    assert_eq!(scenarios(&run.cases).len(), 11);
    assert_eq!(
        changes,
        ["outage drop-in", "host tool", "outage drop-in", "host tool"],
        "the drop-in, the aging, the removal and the spend are the only changes"
    );
    assert!(output.contains("AWAIT partner_registration: create the partner-only event"));
    assert!(output.contains("partner_registration.json"), "{output}");
    let journal = format!("{:?}", run.journal);
    assert!(
        journal.contains("snapshot_aging.snapshot_aged")
            && journal.contains("staged precondition: verified_at set 49 h back")
            && journal.contains("(not elapsed time)"),
        "{journal}"
    );
    assert_eq!(run.measurements["rate_limited_before"], 2.0);
}

#[test]
fn staging_discord_registration_not_refused_fails_partner_membership() {
    only_failed(
        Defect::RegistrationNotRefused,
        "partner_membership",
        "does not name MEMBERSHIP_VERIFICATION_REQUIRED",
    );
}

#[test]
fn staging_discord_unaudited_release_fails_eligibility_release() {
    only_failed(
        Defect::ReleaseUnaudited,
        "eligibility_release",
        "release_audited: an event.reservation_released audit row for the registration: its \
         deadline passed 900 s after the request row",
    );
}

#[test]
fn staging_discord_release_later_than_60_seconds_fails_propagation() {
    only_failed(
        Defect::LateRelease,
        "partner_role_propagation_within_60_seconds",
        "observed 1000 ms after its 60 s deadline",
    );
}

#[test]
fn staging_discord_no_unavailable_outcome_fails_network_outage() {
    only_failed(
        Defect::NoUnavailableOutcome,
        "network_outage",
        "no unavailable outcome logged yet",
    );
}

#[test]
fn staging_discord_absent_banner_fails_staleness_warning() {
    only_failed(
        Defect::BannerAbsent,
        "staleness_warning",
        "lacks [\"Discord verification is delayed\"]",
    );
}

#[test]
fn staging_discord_guest_during_outage_fails_cached_grace() {
    only_failed(
        Defect::GuestDuringOutage,
        "cached_grace",
        "cached_grace: the cached admin role still applies: the saved page read lacks",
    );
}

#[test]
fn staging_discord_events_unavailable_fails_non_blocking_during_outage() {
    only_failed(
        Defect::EventsUnavailable,
        "non_blocking_during_outage",
        "no /api/v1/events line with 200",
    );
}

#[test]
fn staging_discord_missing_override_row_fails_admin_override() {
    only_failed(
        Defect::OverrideMissing,
        "admin_override",
        "the operator's grace override row: not observed within 900 s",
    );
}

#[test]
fn staging_discord_unsynced_partner_role_fails_outage_recovery() {
    only_failed(
        Defect::PartnerRoleNotSynced,
        "outage_recovery",
        "partner role not synced yet",
    );
}

#[test]
fn staging_discord_counter_at_its_baseline_fails_rate_limit() {
    only_failed(
        Defect::CounterNotMoving,
        "rate_limit",
        "rate_limited_counted: the rate-limited count moved: its deadline passed 180 s after the \
         step's start (last seen: rate_limited count 2)",
    );
}

#[test]
fn staging_discord_no_verification_after_rate_limit_fails_rate_limit_recovery() {
    only_failed(
        Defect::NoVerificationAfterRateLimit,
        "rate_limit_recovery",
        "last_error \"rate limited by Discord\"",
    );
}
