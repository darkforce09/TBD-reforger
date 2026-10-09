//! The claim loop against a stand-in of the platform API's executor routes: polling after 204,
//! `executing` acknowledged before any effect, a stale fencing token abandoning the command,
//! result reports retried until acknowledged, commands refused without acting, and a mission
//! restart that changes the dedicated server's config only after `executing` was acknowledged.

#[path = "test_support/fake_ledger_api.rs"]
mod fake_ledger_api;
#[path = "test_support/fake_systemctl.rs"]
mod fake_systemctl;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use fake_ledger_api::{EventLog, FENCING_TOKEN, Failure, FakeLedgerApi, LedgerEvent};
use fake_systemctl::{FakeSystemctl, UnitScenario};
use game_server_host_agent::action_verdict::ActionVerdict;
use game_server_host_agent::command_execution::{
    FleetActionExecutor, HostActionExecutor, HostCommand,
};
use game_server_host_agent::dedicated_server_config::DedicatedServerConfig;
use game_server_host_agent::ledger_client::{BackoffPolicy, CommandLoop, LedgerApi, LedgerTimings};
use game_server_host_agent::process_control::{
    ProcessControl, ProcessControlSettings, SystemdUnitName,
};
use game_server_host_agent::rcon::{RconClient, RconSettings, RconTimings};
use game_server_host_agent::secret_text::SecretText;
use serde_json::{Map, Value, json};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep};
use uuid::Uuid;

const CREDENTIAL: &str = "tbdm_0123456789abcdef0123456789abcdef_\
     0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const DEV_POC: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";
const SERVER_CONFIG: &str = "{\"bindPort\": 2001, \"game\": {\"name\": \"TBD Staging\", \
     \"scenarioId\": \"{ECC61978EDCC2B5A}Missions/23_Campaign.conf\", \"maxPlayers\": 64}}\n";

/// An executor that records each effect in the stand-in's event log and returns a fixed verdict.
struct RecordingExecutor {
    log: EventLog,
    verdict: ActionVerdict,
}

impl RecordingExecutor {
    fn succeeding(api: &FakeLedgerApi) -> Self {
        let mut outcome = Map::new();
        outcome.insert("active_state".to_owned(), Value::from("active"));
        Self {
            log: api.event_log(),
            verdict: ActionVerdict::success(outcome),
        }
    }
}

impl FleetActionExecutor for RecordingExecutor {
    async fn execute(&self, command: &HostCommand) -> ActionVerdict {
        self.log.record_effect(command.action_name());
        self.verdict.clone()
    }
}

/// The claim loop running on its own task until stopped.
struct RunningAgent {
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl RunningAgent {
    fn start<E: FleetActionExecutor + 'static>(api: &FakeLedgerApi, executor: E) -> Self {
        let ledger = LedgerApi::new(&api.base_url(), &SecretText::new(CREDENTIAL))
            .expect("the stand-in API base URL builds a ledger client");
        let quick = BackoffPolicy {
            initial: Duration::from_millis(20),
            maximum: Duration::from_millis(80),
        };
        let timings = LedgerTimings {
            poll_interval: Duration::from_millis(50),
            claim_retry: quick,
            report_retry: quick,
        };
        let command_loop = CommandLoop::new(ledger, executor, timings);
        let (stop, stopped) = oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            command_loop
                .run(async {
                    let _ = stopped.await;
                })
                .await;
        });
        Self { stop, task }
    }

    async fn stop(self) {
        let _ = self.stop.send(());
        self.task.await.expect("the claim loop ends cleanly");
    }
}

/// Waits until the event log satisfies `condition` and returns it, failing after five seconds.
async fn events_once(
    api: &FakeLedgerApi,
    what: &str,
    condition: impl Fn(&[LedgerEvent]) -> bool,
) -> Vec<LedgerEvent> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let events = api.events();
        if condition(&events) {
            return events;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {what}: {events:#?}"
        );
        sleep(Duration::from_millis(10)).await;
    }
}

fn result_reported(events: &[LedgerEvent]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, LedgerEvent::Result { answered: 200, .. }))
}

/// Every event except the claims.
fn steps(events: &[LedgerEvent]) -> Vec<LedgerEvent> {
    events
        .iter()
        .filter(|event| !matches!(event, LedgerEvent::Claim { .. }))
        .cloned()
        .collect()
}

fn executing(command_id: Uuid, answered: u16) -> LedgerEvent {
    LedgerEvent::Executing {
        command_id,
        body: json!({"fencing_token": FENCING_TOKEN}),
        answered,
    }
}

fn effect(action: &str) -> LedgerEvent {
    LedgerEvent::Effect {
        action: action.to_owned(),
    }
}

fn succeeded_result(command_id: Uuid, answered: u16) -> LedgerEvent {
    LedgerEvent::Result {
        command_id,
        body: json!({
            "fencing_token": FENCING_TOKEN,
            "succeeded": true,
            "outcome": {"active_state": "active"},
        }),
        answered,
    }
}

#[tokio::test]
async fn host_agent_ledger_reports_executing_before_the_effect_and_the_result_after_it() {
    let api = FakeLedgerApi::start().await;
    let command = api.enqueue("start", json!({}));
    let agent = RunningAgent::start(&api, RecordingExecutor::succeeding(&api));
    let events = events_once(&api, "the result", result_reported).await;
    agent.stop().await;
    assert_eq!(
        steps(&events),
        vec![
            executing(command, 200),
            effect("start"),
            succeeded_result(command, 200),
        ]
    );
}

#[tokio::test]
async fn host_agent_ledger_stale_fencing_token_abandons_the_command_without_acting() {
    let api = FakeLedgerApi::start().await;
    api.fail_executing_reports(&[Failure::StaleFencingToken]);
    let abandoned = api.enqueue("restart", json!({}));
    let agent = RunningAgent::start(&api, RecordingExecutor::succeeding(&api));
    events_once(&api, "the stale executing report", |events| {
        events.contains(&executing(abandoned, 409))
    })
    .await;
    // The loop goes on claiming: a later command is performed as usual.
    let next = api.enqueue("stop", json!({}));
    let events = events_once(&api, "the next result", result_reported).await;
    agent.stop().await;
    assert_eq!(
        steps(&events),
        vec![
            executing(abandoned, 409),
            executing(next, 200),
            effect("stop"),
            succeeded_result(next, 200),
        ],
        "the abandoned command was neither performed nor reported again"
    );
}

#[tokio::test]
async fn host_agent_ledger_retries_the_result_report_until_acknowledged() {
    let api = FakeLedgerApi::start().await;
    api.fail_result_reports(&[
        Failure::ServiceUnavailable,
        Failure::ServiceUnavailable,
        Failure::ServiceUnavailable,
    ]);
    let command = api.enqueue("restart", json!({}));
    let agent = RunningAgent::start(&api, RecordingExecutor::succeeding(&api));
    let events = events_once(&api, "the acknowledged result", result_reported).await;
    agent.stop().await;
    assert_eq!(
        steps(&events),
        vec![
            executing(command, 200),
            effect("restart"),
            succeeded_result(command, 503),
            succeeded_result(command, 503),
            succeeded_result(command, 503),
            succeeded_result(command, 200),
        ],
        "the effect ran once; the same result was reported until acknowledged"
    );
}

fn mission_arguments() -> Value {
    json!({
        "deployment_id": "5f1c9a52-2d64-4f4e-9d1e-3b7b0a6f9c21",
        "artifact_id": "0b9f2c7e-8a41-4c3d-b6f5-1e2d3c4b5a69",
        "artifact_sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        "scenario_id": DEV_POC,
    })
}

/// The game host's executor over the stand-in systemctl and the server config at `config`. Its
/// RCON client points at a port nothing listens on; a mission restart never uses it.
async fn mission_host(systemctl: &FakeSystemctl, config: &Path) -> HostActionExecutor {
    let rcon = RconClient::start(RconSettings {
        server: "127.0.0.1:9"
            .parse()
            .expect("the loopback address parses as a socket address"),
        password: SecretText::new("range-master"),
        timings: RconTimings::default(),
    })
    .await
    .expect("the RCON client binds a local UDP socket");
    let process_control = ProcessControl::new(ProcessControlSettings {
        systemctl_program: systemctl.program(),
        unit: SystemdUnitName::parse("tbd-reforger.service").expect("the unit name is valid"),
        start_dwell: Duration::ZERO,
        verb_timeout: Duration::from_secs(5),
        state_read_timeout: Duration::from_secs(5),
    });
    HostActionExecutor::new(
        process_control,
        rcon,
        DedicatedServerConfig::new(config.to_path_buf()),
    )
}

/// `server.json` holding [`SERVER_CONFIG`] in a directory of its own.
fn server_config_file() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("the temporary directory is created");
    let path = directory.path().join("server.json");
    fs::write(&path, SERVER_CONFIG).expect("the server config is written");
    (directory, path)
}

const RUNNING_UNIT: UnitScenario = UnitScenario {
    load_state: "loaded",
    active_state: "active",
    active_state_after_verb: "active",
    verb_exit: "0",
    show_exit: 0,
};

#[tokio::test]
async fn host_agent_ledger_mission_restart_changes_nothing_without_an_executing_acknowledgement() {
    let api = FakeLedgerApi::start().await;
    let systemctl = FakeSystemctl::install(&RUNNING_UNIT).await;
    let (_directory, config) = server_config_file();
    api.fail_executing_reports(&[Failure::StaleFencingToken]);
    let command = api.enqueue("restart_with_mission", mission_arguments());
    let agent = RunningAgent::start(&api, mission_host(&systemctl, &config).await);
    events_once(&api, "three claims after the stale report", |events| {
        events
            .iter()
            .skip_while(|event| !matches!(event, LedgerEvent::Executing { .. }))
            .filter(|event| matches!(event, LedgerEvent::Claim { .. }))
            .count()
            >= 3
    })
    .await;
    agent.stop().await;
    assert_eq!(steps(&api.events()), vec![executing(command, 409)]);
    assert_eq!(fs::read_to_string(&config).unwrap(), SERVER_CONFIG);
    assert!(systemctl.invocations().is_empty(), "nothing was restarted");
}
