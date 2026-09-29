//! Staging recordings in isolated Git trees: a passing run is judged held, and every failing run
//! exits 1 with its real observations, no success marker, and a receipt the judge refuses.
use super::*;
use crate::core::repository_layout::documentation::API_READINESS_REGISTER;
use crate::verifications::api_readiness::{case_count, now, verify};
use serde_json::json;
use std::{
    fs,
    process::Command,
    sync::{
        MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

const EVIDENCE: &str = "target/api-readiness";

const FLEET_SCENARIOS: [&str; 9] = [
    "start",
    "stop",
    "restart",
    "kick",
    "custom_console",
    "same_terrain",
    "cross_terrain",
    "lost_acknowledgement",
    "identity_link",
];

/// An isolated repository whose register declares one staging check; holds the environment
/// lock so no other test moves a fingerprinted variable between `begin` and `finish`.
struct Staging {
    root: PathBuf,
    _environment: MutexGuard<'static, ()>,
}

impl Staging {
    /// Declares `check` with its definition from the real acceptance register.
    fn new(check: StagingCheck) -> Self {
        Self::with_definition(check, real_definition(check))
    }

    fn with_definition(check: StagingCheck, definition: serde_json::Value) -> Self {
        let environment = crate::core::test_environment::lock_env();
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "tbd-operational-recording-{}-{unique}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let output = Command::new("git")
            .args(["init", "--quiet", "--initial-branch=main"])
            .current_dir(&root)
            .output()
            .expect("run isolated Git fixture command");
        assert!(output.status.success(), "git init failed");
        let staging = Self {
            root,
            _environment: environment,
        };
        staging.write("Cargo.toml", "[workspace]\n");
        staging.write("apps/module.rs", "pub fn recorded() {}\n");
        let register = json!({
            "version": 1,
            "requirements": [{
                "id": check.id(),
                "behavior": "staging acceptance is recorded",
                "implementation": ["apps/module.rs"],
                "checks": [check.id()],
                "assumptions": [],
            }],
            "checks": [definition],
        });
        staging.write(
            API_READINESS_REGISTER,
            &serde_json::to_string_pretty(&register).unwrap(),
        );
        staging
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn evidence(&self, file: &str) -> PathBuf {
        self.root.join(EVIDENCE).join(file)
    }

    fn begin(&self, check: StagingCheck) -> Result<RecordingSession> {
        RecordingSession::begin(&self.root, Path::new(EVIDENCE), check, argv(check))
    }

    /// Records `outcome` from a fresh session of `check`.
    fn record(&self, check: StagingCheck, outcome: RecordedOutcome) -> RecordedReceipt {
        self.begin(check).unwrap().finish(outcome).unwrap()
    }

    /// Judges the written receipt of `check` as `verify` does, against the current tree.
    fn judge(&self, check: StagingCheck) -> (Result<u64>, Receipt, String) {
        let register = register::read(&self.root).unwrap();
        let definition = register
            .checks
            .iter()
            .find(|candidate| candidate.id == check.id())
            .unwrap();
        let (receipt, log) = evidence::read(&self.root.join(EVIDENCE), definition).unwrap();
        let verdict = evidence::validate(
            definition,
            &receipt,
            &log,
            &fingerprint::source(&self.root).unwrap(),
            &fingerprint::configuration(&self.root).unwrap(),
            now(),
        );
        (verdict, receipt, log)
    }

    /// Asserts the FAIL contract and returns the log: exit 1 in the result and the receipt, no
    /// success marker, the real observations kept, a verdict naming `reason`, and a judge refusal.
    fn assert_refused(&self, recorded: &RecordedReceipt, reason: &str) -> String {
        assert_eq!(recorded.exit_code, 1, "{}", recorded.summary);
        assert!(
            recorded.summary.starts_with("staging_fleet: FAIL ")
                && recorded.summary.contains(reason),
            "{} does not name {reason}",
            recorded.summary
        );
        let (verdict, receipt, log) = self.judge(StagingCheck::Fleet);
        assert!(verdict.is_err(), "the judge accepted a failing receipt");
        assert_eq!(receipt.exit_code, 1);
        assert!(!log.contains("staging_fleet: PASS"), "marker in:\n{log}");
        assert_eq!(log.lines().last(), Some(recorded.summary.as_str()));
        assert!(
            matches!(receipt.observations, Some(Observations::Fleet { client_count, .. }) if client_count > 0),
            "the failing receipt dropped the real observations"
        );
        assert_eq!(verify(&self.root, Path::new(EVIDENCE), false).unwrap(), 1);
        log
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn real_definition(check: StagingCheck) -> serde_json::Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let register: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(API_READINESS_REGISTER)).unwrap()).unwrap();
    register["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["id"] == check.id())
        .cloned()
        .expect("the real register declares every staging check")
}

fn argv(check: StagingCheck) -> Vec<String> {
    let procedure = check.id().trim_start_matches("staging_");
    ["cargo", "xtask", "staging", procedure, "--record"]
        .map(String::from)
        .to_vec()
}

fn case(name: &str, status: CaseStatus) -> RecordedCase {
    RecordedCase {
        name: CaseName::new(name).unwrap(),
        status,
    }
}

fn passing_cases() -> Vec<RecordedCase> {
    ["server1_stop", "identity_link", "kick"]
        .map(|name| case(name, CaseStatus::Ok))
        .to_vec()
}

fn fleet_observations(fixture_sha256: &str, client_count: u64) -> Observations {
    Observations::Fleet {
        server_ids: (1..=5).map(|n| format!("server-{n}")).collect(),
        client_count,
        scenarios: FLEET_SCENARIOS.map(String::from).to_vec(),
        fixture_sha256: fixture_sha256.to_owned(),
    }
}

fn manifest() -> FixtureManifest {
    FixtureManifest::new(&json!({
        "procedure": "fleet",
        "server_ids": ["server-1", "server-2", "server-3", "server-4", "server-5"],
        "cases": ["server1_stop", "identity_link", "kick"],
    }))
    .unwrap()
}

/// A fleet outcome with the given cases, two clients and every scenario.
fn fleet_outcome(cases: Vec<RecordedCase>) -> RecordedOutcome {
    let fixture_manifest = manifest();
    RecordedOutcome {
        cases,
        environment: vec![
            EnvironmentEntry::new("staging_host", "dooley").unwrap(),
            EnvironmentEntry::new("api_binary_sha256", &"b".repeat(64)).unwrap(),
        ],
        observations: fleet_observations(fixture_manifest.sha256(), 2),
        fixture_manifest,
        journal: vec![
            ObservationRecord::new(
                "w1_stop",
                "database reader",
                "fleet_commands row 41 succeeded",
                &"c".repeat(64),
            )
            .unwrap(),
        ],
    }
}

#[test]
fn every_staging_check_of_the_real_register_can_begin_a_recording() {
    for check in [
        StagingCheck::Fleet,
        StagingCheck::Discord,
        StagingCheck::Load,
    ] {
        let staging = Staging::new(check);
        let session = staging.begin(check).unwrap();
        let prefix = format!("{}-{}-", session.started_unix_seconds, std::process::id());
        assert!(
            session.run_id().starts_with(&prefix),
            "{}",
            session.run_id()
        );
    }
}

#[test]
fn a_passing_recording_is_judged_held() {
    let staging = Staging::new(StagingCheck::Fleet);
    let session = staging.begin(StagingCheck::Fleet).unwrap();
    let run_id = session.run_id().to_owned();
    let recorded = session.finish(fleet_outcome(passing_cases())).unwrap();
    assert_eq!(
        recorded,
        RecordedReceipt {
            exit_code: 0,
            summary: "staging_fleet: PASS 3/3".into()
        }
    );
    let (verdict, receipt, log) = staging.judge(StagingCheck::Fleet);
    assert_eq!(verdict.unwrap(), 3);
    assert_eq!(
        verify(&staging.root, Path::new(EVIDENCE), false).unwrap(),
        0
    );
    assert_eq!(log.matches("staging_fleet: PASS").count(), 1);
    assert_eq!(log.lines().last(), Some("staging_fleet: PASS 3/3"));
    let header = log.lines().next().unwrap();
    assert!(
        header.starts_with(&format!(
            "staging-run: staging_fleet run={run_id} started={} ",
            receipt.started_unix_seconds
        )) && header.ends_with("command=cargo xtask staging fleet --record"),
        "{header}"
    );
    for line in [
        "environment: staging_host=dooley",
        "observation: w1_stop database reader fleet_commands row 41 succeeded sha256=",
        "case staging_fleet_identity_link ... ok",
    ] {
        assert!(log.contains(line), "missing {line:?} in:\n{log}");
    }
    assert_eq!(receipt.command, argv(StagingCheck::Fleet));
    assert_eq!(receipt.environment[0], "staging_host=dooley");
    let tools: Vec<_> = receipt
        .tool_versions
        .iter()
        .map(|line| line.split(' ').next().unwrap())
        .collect();
    assert_eq!(tools, ["rustc", "cargo", "git"]);
    assert!(receipt.property_runs.is_empty());
}

#[test]
fn a_failed_case_fails_the_run() {
    let staging = Staging::new(StagingCheck::Fleet);
    let mut cases = passing_cases();
    cases[2].status = CaseStatus::Failed("no kick outcome within 60 s".into());
    let recorded = staging.record(StagingCheck::Fleet, fleet_outcome(cases));
    let log = staging.assert_refused(&recorded, "1 failed");
    assert!(recorded.summary.starts_with("staging_fleet: FAIL 2/3 "));
    assert!(log.contains("case staging_fleet_kick ... FAILED (no kick outcome within 60 s)\n"));
}

#[test]
fn a_case_not_run_fails_the_run_and_names_the_missing_dependency() {
    let staging = Staging::new(StagingCheck::Fleet);
    let mut cases = passing_cases();
    for name in [
        "kick_targets_one_of_two_clients",
        "same_terrain_carries_two_clients",
    ] {
        cases.push(case(
            name,
            CaseStatus::NotRun {
                missing: "second game client".into(),
            },
        ));
    }
    let recorded = staging.record(StagingCheck::Fleet, fleet_outcome(cases));
    let log = staging.assert_refused(&recorded, "2 not run");
    assert!(log.contains(
        "case staging_fleet_kick_targets_one_of_two_clients ... NOT RUN (missing: second game client)\n"
    ));
    assert_eq!(
        log.lines()
            .filter(|line| *line == "missing: second game client")
            .count(),
        1
    );
}

#[test]
fn measurements_the_operational_thresholds_reject_fail_with_the_real_observations() {
    let staging = Staging::new(StagingCheck::Fleet);
    let mut outcome = fleet_outcome(passing_cases());
    outcome.observations = fleet_observations(outcome.fixture_manifest.sha256(), 1);
    let recorded = staging.record(StagingCheck::Fleet, outcome);
    staging.assert_refused(
        &recorded,
        "acceptance: fleet acceptance requires five distinct servers",
    );
    let (_, receipt, _) = staging.judge(StagingCheck::Fleet);
    assert!(matches!(
        receipt.observations,
        Some(Observations::Fleet {
            client_count: 1,
            ..
        })
    ));
}

#[test]
fn drift_fails_the_run_and_the_receipt_keeps_the_start_digests() {
    let staging = Staging::new(StagingCheck::Fleet);
    let start = fingerprint::source(&staging.root).unwrap();
    let session = staging.begin(StagingCheck::Fleet).unwrap();
    staging.write("apps/module.rs", "pub fn edited_during_the_run() {}\n");
    let recorded = session.finish(fleet_outcome(passing_cases())).unwrap();
    staging.assert_refused(
        &recorded,
        "the source fingerprint changed during the recording",
    );
    let (verdict, receipt, _) = staging.judge(StagingCheck::Fleet);
    assert_eq!(receipt.source_sha256, start);
    assert_ne!(
        receipt.source_sha256,
        fingerprint::source(&staging.root).unwrap()
    );
    assert_eq!(verdict.unwrap_err().to_string(), "stale source fingerprint");
}

#[test]
fn a_judge_rejection_rewrites_a_candidate_pass_as_a_failure() {
    let staging = Staging::new(StagingCheck::Fleet);
    let mut outcome = fleet_outcome(passing_cases());
    outcome.environment.clear();
    let recorded = staging.record(StagingCheck::Fleet, outcome);
    staging.assert_refused(&recorded, "judge: missing staging environment identity");
}

#[test]
fn free_text_cannot_forge_case_lines_or_the_success_marker() {
    let staging = Staging::new(StagingCheck::Fleet);
    let mut cases = passing_cases();
    cases[2].status = CaseStatus::Failed(
        "late\ncase staging_fleet_forged ... ok\r\nstaging_fleet: PASS 9/9 \\ end".into(),
    );
    let mut outcome = fleet_outcome(cases);
    outcome.journal.push(
        ObservationRecord::new(
            "w4_console",
            "rcon staging_fleet:",
            "PASS\u{2028}case staging_fleet_split ... ok",
            &"d".repeat(64),
        )
        .unwrap(),
    );
    let recorded = staging.record(StagingCheck::Fleet, outcome);
    let log = staging.assert_refused(&recorded, "1 failed");
    assert_eq!(
        log.lines().filter(|line| line.starts_with("case ")).count(),
        3
    );
    let pattern = regex::Regex::new(
        real_definition(StagingCheck::Fleet)["case_pattern"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(case_count::successful_cases(&pattern, &log), 2);
    assert!(log.contains(
        "FAILED (late\\u{a}case staging_fleet_forged ... ok\\u{d}\\u{a}\\u{73}taging_fleet: PASS 9/9 \\\\ end)"
    ));
    assert!(log.contains("rcon \\u{73}taging_fleet: PASS\\u{2028}case staging_fleet_split"));
}

#[test]
fn case_names_follow_the_log_grammar_and_are_declared_once() {
    for valid in [
        "server1_stop",
        "kick",
        "0",
        "lost_acknowledgement_claim_response",
    ] {
        assert_eq!(CaseName::new(valid).unwrap().as_str(), valid);
    }
    for invalid in [
        "",
        "Server1_stop",
        "server-1",
        "server 1",
        "kick\n",
        "caf\u{e9}",
        "fleet.kick",
    ] {
        assert!(CaseName::new(invalid).is_err(), "accepted {invalid:?}");
    }
    let staging = Staging::new(StagingCheck::Fleet);
    let mut cases = passing_cases();
    cases.push(case("kick", CaseStatus::Ok));
    let recorded = staging.record(StagingCheck::Fleet, fleet_outcome(cases));
    staging.assert_refused(&recorded, "case kick is declared twice");
}

#[test]
fn the_manifest_digest_binds_the_written_fixture_and_the_cited_observations() {
    let rebuilt = manifest();
    assert_eq!(rebuilt, manifest(), "serialization is deterministic");
    let other = FixtureManifest::new(&json!({"procedure": "fleet", "cases": []})).unwrap();
    assert_ne!(manifest().sha256(), other.sha256());
    let staging = Staging::new(StagingCheck::Fleet);
    let recorded = staging.record(StagingCheck::Fleet, fleet_outcome(passing_cases()));
    assert_eq!(recorded.exit_code, 0);
    let written = fs::read(staging.evidence("staging_fleet.fixture.json")).unwrap();
    assert_eq!(fingerprint::digest(&written), manifest().sha256());
    let (_, receipt, log) = staging.judge(StagingCheck::Fleet);
    assert!(log.contains(&format!(
        "fixture: sha256={} manifest=staging_fleet.fixture.json\n",
        manifest().sha256()
    )));
    assert!(matches!(
        receipt.observations,
        Some(Observations::Fleet { ref fixture_sha256, .. }) if fixture_sha256 == manifest().sha256()
    ));
    let mut outcome = fleet_outcome(passing_cases());
    outcome.observations = fleet_observations(other.sha256(), 2);
    let recorded = staging.record(StagingCheck::Fleet, outcome);
    staging.assert_refused(&recorded, "the observations cite fixture");
}

#[test]
fn beginning_a_recording_removes_the_earlier_receipt() {
    let staging = Staging::new(StagingCheck::Fleet);
    staging.record(StagingCheck::Fleet, fleet_outcome(passing_cases()));
    assert_eq!(
        verify(&staging.root, Path::new(EVIDENCE), false).unwrap(),
        0
    );
    let _open = staging.begin(StagingCheck::Fleet).unwrap();
    assert!(!staging.evidence("staging_fleet.json").exists());
    assert_eq!(
        verify(&staging.root, Path::new(EVIDENCE), false).unwrap(),
        2
    );
}

#[test]
fn only_operational_checks_recorded_under_run_discipline_can_begin() {
    let refusal = |result: Result<RecordingSession>| format!("{:#}", result.unwrap_err());
    for (field, value, reason) in [
        (
            "class",
            json!("implementation"),
            "staging_fleet is not an operational check without a local command",
        ),
        (
            "command",
            json!(["cargo", "test"]),
            "staging_fleet is not an operational check without a local command",
        ),
        (
            "success_marker",
            json!("fleet complete"),
            "staging_fleet must use `staging_fleet: PASS`",
        ),
    ] {
        let mut definition = real_definition(StagingCheck::Fleet);
        definition[field] = value;
        let staging = Staging::with_definition(StagingCheck::Fleet, definition);
        let refused = refusal(staging.begin(StagingCheck::Fleet));
        assert!(refused.starts_with(reason), "{field}: {refused}");
    }
    let staging = Staging::new(StagingCheck::Fleet);
    assert_eq!(
        refusal(staging.begin(StagingCheck::Discord)),
        "the acceptance register declares no staging_discord"
    );
    assert_eq!(
        refusal(RecordingSession::begin(
            &staging.root,
            Path::new(EVIDENCE),
            StagingCheck::Fleet,
            Vec::new()
        )),
        "a recording names the command that runs it"
    );
    let clean = [("PATH".into(), "/usr/bin".into())];
    assert!(ensure_run_discipline(clean.clone()).is_ok());
    for name in [
        "TEST_DATABASE_URL",
        "DEPLOY_ENV",
        "PROPTEST_CASES",
        "PROPTEST_RNG_SEED",
    ] {
        let mut variables = clean.to_vec();
        variables.push((name.into(), "secret-value".into()));
        let refusal = ensure_run_discipline(variables).unwrap_err().to_string();
        assert!(
            refusal.starts_with(name) && !refusal.contains("secret-value"),
            "{refusal}"
        );
    }
}

#[test]
fn tool_identity_refuses_a_tool_that_fails_or_prints_no_version() {
    use crate::verifications::api_readiness::tool_identity;
    let root = std::env::temp_dir();
    assert!(
        tool_identity::version(&root, "git")
            .unwrap()
            .starts_with("git version ")
    );
    // GNU `test` treats `--version` as a non-empty string: exit 0, no output.
    assert_eq!(
        format!("{:#}", tool_identity::version(&root, "test").unwrap_err()),
        "test printed no version"
    );
    // GNU `false` prints its version and still exits 1.
    assert_eq!(
        format!("{:#}", tool_identity::version(&root, "false").unwrap_err()),
        "cannot identify false"
    );
}

#[test]
fn environment_entries_and_observation_records_refuse_unsafe_values() {
    assert!(EnvironmentEntry::new("workshop_version", "1.4.0.53").is_ok());
    for (key, value) in [
        ("discord_bot_token", "x"),
        ("jwt_secret", "x"),
        ("rcon_password", "x"),
        ("host_agent_credential", "x"),
        ("Staging_Host", "dooley"),
        ("staging host", "dooley"),
        ("", "dooley"),
        ("staging_host", " "),
    ] {
        assert!(
            EnvironmentEntry::new(key, value).is_err(),
            "accepted {key:?}"
        );
    }
    let digest = "e".repeat(64);
    assert!(ObservationRecord::new("W1.stop-2", "chrome page read", "banner", &digest).is_ok());
    for (step, observer, summary, artifact) in [
        ("w1 stop", "observer", "summary", digest.clone()),
        ("", "observer", "summary", digest.clone()),
        ("w1", " ", "summary", digest.clone()),
        ("w1", "observer", "", digest.clone()),
        ("w1", "observer", "summary", "e".repeat(63)),
        ("w1", "observer", "summary", "E".repeat(64)),
    ] {
        assert!(
            ObservationRecord::new(step, observer, summary, &artifact).is_err(),
            "accepted {step:?} {observer:?} {summary:?} {artifact}"
        );
    }
}
