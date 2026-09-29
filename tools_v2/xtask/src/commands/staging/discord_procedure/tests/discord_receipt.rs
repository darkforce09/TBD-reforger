//! The Discord procedure's receipt, lists and preconditions: a recorded run whose eleven runnable
//! cases hold still ends in a failing receipt with no marker that names the missing account, its
//! real observations and the staged precondition; the recovery list puts everything back; the
//! `preflight --discord` checks name each unmet precondition; unset ids refuse the plan.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::MutexGuard;

use serde_json::{Value, json};

use super::DiscordProcedure;
use super::discord_cases::MISSING_TEST_SUBJECT;
use super::recorded_discord::{
    OPERATOR, PARTNER_GUILD, PARTNER_ROLE, RecordedDiscord, T0, discord_settings, member_line,
};
use crate::commands::staging::operator_coordination::action_list::render;
use crate::commands::staging::procedure_runner::fake_clock::FakeClock;
use crate::commands::staging::procedure_runner::procedure::StagingProcedure;
use crate::commands::staging::procedure_runner::recording::{RecordingInputs, record};
use crate::commands::staging::procedure_runner::runner_support::{scratch_folder, test_settings};
use crate::commands::staging::remote_observers::remote_command::CommandOutput;
use crate::commands::staging::run_identity::{EVIDENCE_DIRECTORY, STAGING_RUNS_DIRECTORY};
use crate::commands::staging::support_commands::preflight::PreflightProbe;
use crate::core::repository_layout::documentation::API_READINESS_REGISTER;
use crate::verifications::api_readiness::operational_recording::EnvironmentEntry;

/// An isolated Git tree whose register declares the real `staging_discord` check; holds the
/// environment lock so no other test moves a fingerprinted variable during the recording.
struct IsolatedRepository {
    root: PathBuf,
    _environment: MutexGuard<'static, ()>,
}

impl IsolatedRepository {
    fn new() -> Self {
        let environment = crate::core::test_environment::lock_env();
        let root = scratch_folder("discord-receipt");
        let status = Command::new("git")
            .args(["init", "--quiet", "--initial-branch=main"])
            .current_dir(&root)
            .status()
            .expect("run git init");
        assert!(status.success(), "git init failed");
        let register_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(API_READINESS_REGISTER);
        let real: Value = serde_json::from_slice(&std::fs::read(register_path).unwrap()).unwrap();
        let definition = real["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["id"] == "staging_discord")
            .cloned()
            .expect("the real register declares staging_discord");
        let register = json!({
            "version": 1,
            "requirements": [{
                "id": "staging_discord",
                "behavior": "Discord acceptance is recorded",
                "implementation": ["apps/module.rs"],
                "checks": ["staging_discord"],
                "assumptions": [],
            }],
            "checks": [definition],
        });
        let repository = Self {
            root,
            _environment: environment,
        };
        repository.write("Cargo.toml", "[workspace]\n");
        repository.write("apps/module.rs", "pub fn recorded() {}\n");
        repository.write(
            API_READINESS_REGISTER,
            &serde_json::to_string_pretty(&register).unwrap(),
        );
        repository
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn evidence(&self, file: &str) -> Vec<u8> {
        std::fs::read(self.root.join(EVIDENCE_DIRECTORY).join(file)).unwrap()
    }
}

impl Drop for IsolatedRepository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The browser inbox of the single run recorded under `root`.
fn run_inbox(root: &Path) -> PathBuf {
    let runs = root.join(STAGING_RUNS_DIRECTORY).join("staging_discord");
    let run = std::fs::read_dir(&runs)
        .unwrap()
        .next()
        .expect("the recording created its run folder")
        .unwrap()
        .path();
    run.join("browser_inbox")
}

#[test]
fn staging_discord_record_writes_a_failing_receipt_naming_the_missing_account() {
    let repository = IsolatedRepository::new();
    let settings = discord_settings();
    let clock = FakeClock::starting_at(T0);
    let root = repository.root.clone();
    let mut host = RecordedDiscord::new(&settings, &clock, move || run_inbox(&root), None);
    let mut output = Vec::new();
    let exit_code = record(
        &DiscordProcedure,
        RecordingInputs {
            root: &repository.root,
            settings: &settings,
            host: &mut host,
            clock: &clock,
            command: ["cargo", "xtask", "staging", "discord", "--record"]
                .map(String::from)
                .to_vec(),
            output: &mut output,
        },
    )
    .unwrap();
    let printed = String::from_utf8(output).unwrap();
    assert_eq!(exit_code, 1, "{printed}");
    let receipt: Value =
        serde_json::from_slice(&repository.evidence("staging_discord.json")).unwrap();
    let log = String::from_utf8(repository.evidence("staging_discord.log")).unwrap();
    let manifest: Value =
        serde_json::from_slice(&repository.evidence("staging_discord.fixture.json")).unwrap();
    assert_eq!(receipt["exit_code"], 1);
    assert!(!log.contains("staging_discord: PASS"), "{log}");
    assert!(
        log.lines()
            .last()
            .unwrap()
            .starts_with("staging_discord: FAIL 11/13"),
        "{log}"
    );
    assert!(
        log.contains(&format!("missing: {MISSING_TEST_SUBJECT}")),
        "{log}"
    );
    assert!(log.contains(&format!(
        "case staging_discord_role_demotion ... NOT RUN (missing: {MISSING_TEST_SUBJECT})"
    )));
    assert!(
        log.contains("case staging_discord_rate_limit_recovery ... ok"),
        "{log}"
    );
    assert!(
        log.contains("observation: partner_role_removal.release_within_60_seconds database"),
        "{log}"
    );
    let staged = "staged_precondition=membership_snapshot_aged_49h";
    assert!(log.contains(&format!("environment: {staged}")), "{log}");
    assert!(
        receipt["environment"]
            .as_array()
            .unwrap()
            .contains(&json!(staged)),
        "{}",
        receipt["environment"]
    );
    let observations = &receipt["observations"];
    assert_eq!(observations["kind"], "discord");
    assert_eq!(observations["scenarios"].as_array().unwrap().len(), 11);
    assert_eq!(manifest["check"], "staging_discord");
    assert!(
        manifest["identities"]["staged_preconditions"][0]
            .as_str()
            .unwrap()
            .contains("49 h old")
    );
    assert_eq!(manifest["identities"]["partner_role_id"], PARTNER_ROLE);
    assert_eq!(
        crate::verifications::api_readiness::verify(
            &repository.root,
            Path::new(EVIDENCE_DIRECTORY),
            false
        )
        .unwrap(),
        1,
        "the judge accepted a failing Discord receipt"
    );
}

#[test]
fn staging_discord_recovery_list_puts_everything_back() {
    let settings = discord_settings();
    let recovery = render(
        "staging_discord recovery",
        &DiscordProcedure.recovery_action_list(&settings),
    );
    assert!(
        recovery.contains("1. [harness] remove the HTTPS_PROXY outage drop-in and restart the API"),
        "{recovery}"
    );
    assert!(
        recovery.contains("staging-discord-outage.conf"),
        "{recovery}"
    );
    assert!(recovery.contains(&format!(
        "give role {PARTNER_ROLE} back to member {OPERATOR}"
    )));
    assert!(recovery.contains("3. [orchestrator (browser)] delete the partner event"));
    let unset = render(
        "staging_discord recovery",
        &DiscordProcedure.recovery_action_list(&test_settings()),
    );
    assert!(
        unset.contains("give role <TBD_STAGING_PARTNER_ROLE_ID unset>"),
        "{unset}"
    );
    let run = render(
        "staging_discord actions",
        &DiscordProcedure.action_list(&settings),
    );
    assert!(run.contains("--hold-seconds"), "{run}");
    // The spend runs the host tool's one argv builder, starting at the time the host computed.
    assert!(run.contains("start_ms=$((next_ms - 300))"), "{run}");
    assert!(
        run.contains(
            "'--start-at-unix-ms' \"$start_ms\" '--hold-seconds' '3' '--max-requests' '50' \
             '--confirm-database' 'tbd_reforger' '--apply'"
        ),
        "{run}"
    );
}

#[test]
fn staging_discord_preflight_names_each_unmet_precondition() {
    let checks = DiscordProcedure.preflight_checks(&discord_settings());
    let judge = |name: &str, stdout: &str| -> Result<String, String> {
        let check = checks.iter().find(|check| check.name == name).unwrap();
        let PreflightProbe::Host { judge, .. } = &check.probe else {
            panic!("{name} is not a host check");
        };
        judge(&CommandOutput {
            exit_code: 0,
            stdout: stdout.to_string(),
            stderr: String::new(),
        })
    };
    assert_eq!(checks.len(), 5);
    judge("discord bot token key set", "set\n").unwrap();
    let token = judge("discord bot token key set", "unset\n").unwrap_err();
    assert!(token.contains("DISCORD_BOT_TOKEN is empty"), "{token}");
    let main = member_line("1", T0, "member", Some(vec!["900"]));
    judge("bot reads the main guild", &main).unwrap();
    let limited = member_line("1", T0, "rate_limited", None);
    assert_eq!(
        judge("bot reads the main guild", &limited).unwrap_err(),
        "bot read answered rate_limited"
    );
    assert!(judge("bot reads the main guild", "outcome=member\n").is_err());
    let partner = "bot reads the partner guild, partner role held";
    let held = member_line(PARTNER_GUILD, T0, "member", Some(vec![PARTNER_ROLE]));
    judge(partner, &held).unwrap();
    let why = judge(
        partner,
        &member_line(PARTNER_GUILD, T0, "member", Some(vec!["900"])),
    );
    assert!(
        why.unwrap_err()
            .contains(&format!("without role {PARTNER_ROLE}"))
    );
    let fresh = format!("member|{}||{}|{T0}\n", T0 - 30_000, T0 + 3_600_000);
    judge("operator snapshot fresh", &fresh).unwrap();
    let stale = format!("member|{}||{}|{T0}\n", T0 - 61_000, T0 + 3_600_000);
    assert!(judge("operator snapshot fresh", &stale).is_err());
    let failing = format!("member|{}|timeout|{}|{T0}\n", T0 - 1_000, T0 + 3_600_000);
    assert!(judge("operator snapshot fresh", &failing).is_err());
    assert!(judge("operator snapshot fresh", "").is_err());
    let dump = "/home/deploy/tbd/backups/tbd_reforger-pre-discord-1.dump\n";
    judge("pre-Discord backup taken", dump).unwrap();
    let none = judge("pre-Discord backup taken", "").unwrap_err();
    assert!(
        none.contains("staging backup --label pre-discord"),
        "{none}"
    );
    let unset = DiscordProcedure.preflight_checks(&test_settings());
    assert_eq!(unset.len(), 1);
    let PreflightProbe::Local(judge) = &unset[0].probe else {
        panic!("the settings check is local");
    };
    assert!(judge().unwrap_err().contains("TBD_STAGING_PARTNER_ROLE_ID"));
}

#[test]
fn staging_discord_plan_is_refused_without_the_partner_role() {
    let refused = DiscordProcedure.plan(&test_settings()).err().unwrap();
    assert!(format!("{refused:#}").contains("TBD_STAGING_PARTNER_ROLE_ID is not set"));
    let plan = DiscordProcedure.plan(&discord_settings()).unwrap();
    plan.validate().unwrap();
    assert_eq!(plan.steps.len(), 11);
    assert_eq!(plan.declared_cases.len(), 13);
    assert_eq!(
        DiscordProcedure.staged_preconditions().unwrap(),
        [EnvironmentEntry::new("staged_precondition", "membership_snapshot_aged_49h").unwrap()]
    );
}
