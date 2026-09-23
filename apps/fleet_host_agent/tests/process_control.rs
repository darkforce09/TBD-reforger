//! Process control against a stand-in systemctl program the tests write: the agent runs it with
//! a fixed argument vector, and the verdict follows the unit state read back after the dwell,
//! never the exit status of the verb. A mission restart first points the dedicated server's
//! config at the deployment's scenario, changing nothing else in it, and never restarts the unit
//! when the config cannot be switched.

#[path = "test_support/fake_systemctl.rs"]
mod fake_systemctl;

use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use fake_systemctl::{FakeSystemctl, UnitScenario};
use fleet_host_agent::command_execution::{FleetActionExecutor, HostActionExecutor, HostCommand};
use fleet_host_agent::dedicated_server_config::DedicatedServerConfig;
use fleet_host_agent::process_control::{
    ProcessAction, ProcessControl, ProcessControlSettings, SystemdUnitName,
};
use fleet_host_agent::rcon::{RconClient, RconSettings, RconTimings};
use fleet_host_agent::secret_text::SecretText;
use serde_json::{Value, json};
use tokio::time::{Instant, sleep};

const UNIT: &str = "tbd-reforger.service";
const CAMPAIGN: &str = "{ECC61978EDCC2B5A}Missions/23_Campaign.conf";
const DEV_POC: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";

/// A server config in the operator's own layout, with keys the agent knows nothing about.
const SERVER_CONFIG: &str = "{\n\
    \t\"bindPort\": 2001,\n\
    \t\"game\": {\n\
    \t\t\"name\": \"TBD Staging\",\n\
    \t\t\"scenarioId\": \"{ECC61978EDCC2B5A}Missions/23_Campaign.conf\",\n\
    \t\t\"futureKey\": { \"unknownToTheAgent\": [2.50, 1e3] }\n\
    \t},\n\
    \t\"operating\": { \"lobbyPlayerSynchronise\": true }\n\
    }\n";

const HEALTHY_START: UnitScenario = UnitScenario {
    load_state: "loaded",
    active_state: "inactive",
    active_state_after_verb: "active",
    verb_exit: "0",
    show_exit: 0,
};

fn control(program: PathBuf, dwell: Duration) -> ProcessControl {
    ProcessControl::new(ProcessControlSettings {
        systemctl_program: program,
        unit: SystemdUnitName::parse(UNIT).unwrap(),
        start_dwell: dwell,
        verb_timeout: Duration::from_secs(5),
        state_read_timeout: Duration::from_secs(5),
    })
}

#[tokio::test]
async fn process_control_start_succeeds_when_the_unit_is_active_after_the_dwell() {
    let fake = FakeSystemctl::install(&HEALTHY_START).await;
    let dwell = Duration::from_millis(300);
    let started = Instant::now();
    let report = control(fake.program(), dwell)
        .perform(ProcessAction::Start)
        .await;
    assert!(
        started.elapsed() >= dwell,
        "the state is read after the dwell"
    );
    let verdict = report.verdict();
    assert!(verdict.succeeded(), "{verdict:?}");
    let outcome = verdict.outcome().unwrap();
    assert_eq!(outcome["active_state"], "active");
    assert_eq!(outcome["load_state"], "loaded");
    assert_eq!(outcome["dwell_milliseconds"], 300);
    assert_eq!(outcome["systemctl"], "exited with status 0");
}

#[tokio::test]
async fn process_control_runs_systemctl_with_a_fixed_argument_vector() {
    let fake = FakeSystemctl::install(&HEALTHY_START).await;
    control(fake.program(), Duration::ZERO)
        .perform(ProcessAction::Restart)
        .await;
    assert_eq!(
        fake.invocations(),
        vec![
            "--user restart tbd-reforger.service",
            "--user show --property=LoadState --value tbd-reforger.service",
            "--user show --property=ActiveState --value tbd-reforger.service",
        ]
    );
}

#[tokio::test]
async fn process_control_restart_reads_the_unit_state_only_after_the_dwell() {
    // The restart itself succeeds and leaves the unit active; the server then dies during the
    // dwell and systemd reports it failed, while the verb's exit status still reads 0.
    let fake = FakeSystemctl::install(&HEALTHY_START).await;
    let control = control(fake.program(), Duration::from_millis(800));
    let (report, ()) = tokio::join!(control.perform(ProcessAction::Restart), async {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !fake
            .invocations()
            .iter()
            .any(|line| line.contains(" restart "))
        {
            assert!(Instant::now() < deadline, "the restart never ran");
            sleep(Duration::from_millis(10)).await;
        }
        fake.set_active_state("failed");
    });
    let verdict = report.verdict();
    assert!(!verdict.succeeded(), "{verdict:?}");
    assert_eq!(
        verdict.failure_reason(),
        Some(
            "restart left tbd-reforger.service failed instead of active after waiting 800ms; \
             systemctl --user restart exited with status 0"
        )
    );
    assert_eq!(verdict.outcome().unwrap()["active_state"], "failed");
}

#[tokio::test]
async fn process_control_stop_succeeds_when_the_unit_is_inactive() {
    let fake = FakeSystemctl::install(&UnitScenario {
        active_state: "active",
        active_state_after_verb: "inactive",
        ..HEALTHY_START
    })
    .await;
    let report = control(fake.program(), Duration::from_secs(8))
        .perform(ProcessAction::Stop)
        .await;
    assert_eq!(report.dwell, Duration::ZERO, "stop has no dwell");
    let verdict = report.verdict();
    assert!(verdict.succeeded(), "{verdict:?}");
    assert_eq!(verdict.outcome().unwrap()["active_state"], "inactive");
}

#[tokio::test]
async fn process_control_stop_of_a_unit_that_is_not_loaded_fails() {
    // systemctl reports a unit that does not exist as inactive.
    let fake = FakeSystemctl::install(&UnitScenario {
        load_state: "not-found",
        active_state: "inactive",
        active_state_after_verb: "inactive",
        verb_exit: "5",
        show_exit: 0,
    })
    .await;
    let verdict = control(fake.program(), Duration::ZERO)
        .perform(ProcessAction::Stop)
        .await
        .verdict();
    assert!(!verdict.succeeded(), "{verdict:?}");
    let reason = verdict.failure_reason().unwrap();
    assert!(
        reason.contains("the unit is not loaded (LoadState=not-found)"),
        "{reason}"
    );
    assert!(reason.contains("exited with status 5"), "{reason}");
}

#[tokio::test]
async fn process_control_verb_exit_status_does_not_decide_the_verdict() {
    let fake = FakeSystemctl::install(&UnitScenario {
        active_state: "active",
        verb_exit: "1",
        ..HEALTHY_START
    })
    .await;
    let verdict = control(fake.program(), Duration::ZERO)
        .perform(ProcessAction::Start)
        .await
        .verdict();
    assert!(verdict.succeeded(), "{verdict:?}");
    assert_eq!(
        verdict.outcome().unwrap()["systemctl"],
        "exited with status 1"
    );
}

#[tokio::test]
async fn process_control_hung_verb_is_bounded_by_its_timeout() {
    let fake = FakeSystemctl::install(&UnitScenario {
        active_state_after_verb: "activating",
        verb_exit: "hang",
        ..HEALTHY_START
    })
    .await;
    let control = ProcessControl::new(ProcessControlSettings {
        systemctl_program: fake.program(),
        unit: SystemdUnitName::parse(UNIT).unwrap(),
        start_dwell: Duration::ZERO,
        verb_timeout: Duration::from_millis(300),
        state_read_timeout: Duration::from_secs(5),
    });
    let started = Instant::now();
    let verdict = control.perform(ProcessAction::Start).await.verdict();
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "the hung verb was abandoned"
    );
    assert!(!verdict.succeeded());
    let reason = verdict.failure_reason().unwrap();
    assert!(reason.contains("activating instead of active"), "{reason}");
    assert!(reason.contains("did not finish within 300ms"), "{reason}");
}

#[tokio::test]
async fn process_control_unreadable_unit_state_is_a_failure() {
    let fake = FakeSystemctl::install(&UnitScenario {
        show_exit: 1,
        ..HEALTHY_START
    })
    .await;
    let verdict = control(fake.program(), Duration::ZERO)
        .perform(ProcessAction::Start)
        .await
        .verdict();
    assert!(!verdict.succeeded());
    let reason = verdict.failure_reason().unwrap();
    assert!(
        reason.contains("the unit state could not be read"),
        "{reason}"
    );
    assert!(
        reason.contains("systemctl --user show --property=LoadState exited with status 1"),
        "{reason}"
    );
}

#[tokio::test]
async fn process_control_missing_program_is_a_failure_that_names_it() {
    // Spawning forks too, so this test takes its turn through a stand-in it does not run.
    let fake = FakeSystemctl::install(&HEALTHY_START).await;
    let verdict = control(fake.program().with_file_name("absent"), Duration::ZERO)
        .perform(ProcessAction::Restart)
        .await
        .verdict();
    assert!(!verdict.succeeded());
    let reason = verdict.failure_reason().unwrap();
    assert!(
        reason.contains("systemctl --user restart could not be started"),
        "{reason}"
    );
}

/// A host executor over the stand-in systemctl and the server config at `config`. Its RCON
/// client points at a port nothing listens on; a mission restart never uses it.
async fn mission_host(fake: &FakeSystemctl, config: &Path, dwell: Duration) -> HostActionExecutor {
    let rcon = RconClient::start(RconSettings {
        server: "127.0.0.1:9".parse().unwrap(),
        password: SecretText::new("range-master"),
        timings: RconTimings::default(),
    })
    .await
    .unwrap();
    HostActionExecutor::new(
        control(fake.program(), dwell),
        rcon,
        DedicatedServerConfig::new(config.to_path_buf()),
    )
}

fn mission_restart() -> HostCommand {
    HostCommand::from_claim(
        "restart_with_mission",
        &json!({
            "deployment_id": "5f1c9a52-2d64-4f4e-9d1e-3b7b0a6f9c21",
            "artifact_id": "0b9f2c7e-8a41-4c3d-b6f5-1e2d3c4b5a69",
            "artifact_sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
            "scenario_id": DEV_POC,
        }),
    )
    .unwrap()
}

/// A directory holding `server.json` with `contents`.
fn server_config_file(contents: &str) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    fs::write(&path, contents).unwrap();
    (directory, path)
}

/// The names in `directory`, sorted.
fn entries(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn process_control_mission_restart_switches_the_scenario_then_restarts_the_unit() {
    let fake = FakeSystemctl::install(&HEALTHY_START).await;
    let (directory, config) = server_config_file(SERVER_CONFIG);
    fs::set_permissions(&config, Permissions::from_mode(0o640)).unwrap();
    let dwell = Duration::from_millis(300);
    let host = mission_host(&fake, &config, dwell).await;
    let started = Instant::now();
    let verdict = host.execute(&mission_restart()).await;
    assert!(started.elapsed() >= dwell, "the dwell of restart applies");
    assert!(verdict.succeeded(), "{verdict:?}");
    assert_eq!(
        Value::Object(verdict.outcome().unwrap().clone()),
        json!({
            "scenario_id": DEV_POC,
            "unit_active_state": "active",
            "config_path": config.display().to_string(),
        })
    );
    assert_eq!(
        fs::read_to_string(&config).unwrap(),
        SERVER_CONFIG.replace(CAMPAIGN, DEV_POC),
        "only game.scenarioId changed; every other key and byte stayed"
    );
    assert_eq!(
        fs::metadata(&config).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(entries(directory.path()), vec!["server.json"]);
    assert_eq!(
        fake.invocations(),
        vec![
            "--user restart tbd-reforger.service",
            "--user show --property=LoadState --value tbd-reforger.service",
            "--user show --property=ActiveState --value tbd-reforger.service",
        ]
    );
}

#[tokio::test]
async fn process_control_mission_restart_that_fails_says_the_config_names_the_new_scenario() {
    let fake = FakeSystemctl::install(&UnitScenario {
        active_state_after_verb: "failed",
        ..HEALTHY_START
    })
    .await;
    let (_directory, config) = server_config_file(SERVER_CONFIG);
    let verdict = mission_host(&fake, &config, Duration::ZERO)
        .await
        .execute(&mission_restart())
        .await;
    assert!(!verdict.succeeded());
    assert_eq!(
        verdict.failure_reason(),
        Some(
            format!(
                "the server config {} now names {DEV_POC}, but the restart failed: restart left \
                 tbd-reforger.service failed instead of active; systemctl --user restart exited \
                 with status 0",
                config.display()
            )
            .as_str()
        )
    );
    assert_eq!(
        Value::Object(verdict.outcome().unwrap().clone()),
        json!({
            "scenario_id": DEV_POC,
            "config_path": config.display().to_string(),
            "config_updated": true,
            "unit_active_state": "failed",
            "unit_load_state": "loaded",
            "systemctl": "exited with status 0",
        })
    );
    assert_eq!(
        fs::read_to_string(&config).unwrap(),
        SERVER_CONFIG.replace(CAMPAIGN, DEV_POC)
    );
}

#[tokio::test]
async fn process_control_mission_restart_refuses_a_malformed_config_without_restarting() {
    let fake = FakeSystemctl::install(&HEALTHY_START).await;
    for malformed in [
        "{\"game\": {\"scenarioId\": \"x\"",
        "{\"game\": {\"name\": \"no scenario\"}}",
        "{\"game\": {\"scenarioId\": 42}}",
        "not json",
    ] {
        let (directory, config) = server_config_file(malformed);
        let verdict = mission_host(&fake, &config, Duration::ZERO)
            .await
            .execute(&mission_restart())
            .await;
        assert!(!verdict.succeeded());
        let reason = verdict.failure_reason().unwrap();
        assert!(reason.ends_with("; the unit was not restarted"), "{reason}");
        assert_eq!(verdict.outcome().unwrap()["config_updated"], false);
        assert_eq!(fs::read_to_string(&config).unwrap(), malformed);
        assert_eq!(entries(directory.path()), vec!["server.json"]);
    }
    assert!(fake.invocations().is_empty(), "nothing was restarted");
}

#[tokio::test]
async fn process_control_mission_restart_leaves_no_partial_file_when_the_write_fails() {
    let fake = FakeSystemctl::install(&HEALTHY_START).await;
    let (directory, config) = server_config_file(SERVER_CONFIG);
    // The new file cannot be created beside the config.
    fs::set_permissions(directory.path(), Permissions::from_mode(0o500)).unwrap();
    let probe = directory.path().join("probe");
    if fs::write(&probe, "").is_ok() {
        // A privileged user writes into a read-only directory anyway: nothing fails to observe.
        fs::remove_file(&probe).unwrap();
        fs::set_permissions(directory.path(), Permissions::from_mode(0o700)).unwrap();
        return;
    }
    let verdict = mission_host(&fake, &config, Duration::ZERO)
        .await
        .execute(&mission_restart())
        .await;
    fs::set_permissions(directory.path(), Permissions::from_mode(0o700)).unwrap();
    assert!(!verdict.succeeded());
    let reason = verdict.failure_reason().unwrap();
    assert!(reason.contains("cannot be replaced"), "{reason}");
    assert!(reason.ends_with("; the unit was not restarted"), "{reason}");
    assert_eq!(fs::read_to_string(&config).unwrap(), SERVER_CONFIG);
    assert_eq!(
        entries(directory.path()),
        vec!["server.json"],
        "no partial file"
    );
    assert!(fake.invocations().is_empty(), "nothing was restarted");
}
