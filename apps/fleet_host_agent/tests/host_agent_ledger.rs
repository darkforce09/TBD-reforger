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
use fleet_host_agent::action_verdict::ActionVerdict;
use fleet_host_agent::command_execution::{FleetActionExecutor, HostActionExecutor, HostCommand};
use fleet_host_agent::dedicated_server_config::DedicatedServerConfig;
use fleet_host_agent::ledger_client::{
    BackoffPolicy, ClaimOutcome, CommandLoop, LedgerApi, LedgerError, LedgerTimings,
};
use fleet_host_agent::process_control::{ProcessControl, ProcessControlSettings, SystemdUnitName};
use fleet_host_agent::rcon::{RconClient, RconSettings, RconTimings};
use fleet_host_agent::secret_text::SecretText;
use serde_json::{Map, Value, json};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep};
use uuid::Uuid;

const CREDENTIAL: &str = "tbdm_0123456789abcdef0123456789abcdef_\
     0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const CAMPAIGN: &str = "{ECC61978EDCC2B5A}Missions/23_Campaign.conf";
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
        let ledger = LedgerApi::new(&api.base_url(), &SecretText::new(CREDENTIAL)).unwrap();
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

fn claims(events: &[LedgerEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, LedgerEvent::Claim { .. }))
        .count()
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
async fn host_agent_ledger_polls_again_after_nothing_is_claimable() {
    let api = FakeLedgerApi::start().await;
    let agent = RunningAgent::start(&api, RecordingExecutor::succeeding(&api));
    let events = events_once(&api, "three claims", |events| claims(events) >= 3).await;
    agent.stop().await;
    assert!(steps(&events).is_empty(), "{events:#?}");
    for event in &events {
        assert_eq!(
            event,
            &LedgerEvent::Claim {
                authorization: Some(format!("Bearer {CREDENTIAL}")),
            }
        );
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
async fn host_agent_ledger_retries_the_executing_report_before_acting() {
    let api = FakeLedgerApi::start().await;
    api.fail_executing_reports(&[Failure::ServiceUnavailable, Failure::ServiceUnavailable]);
    let command = api.enqueue("restart", json!({}));
    let agent = RunningAgent::start(&api, RecordingExecutor::succeeding(&api));
    let events = events_once(&api, "the result", result_reported).await;
    agent.stop().await;
    assert_eq!(
        steps(&events),
        vec![
            executing(command, 503),
            executing(command, 503),
            executing(command, 200),
            effect("restart"),
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
async fn host_agent_ledger_stale_fencing_token_on_the_result_is_not_retried() {
    let api = FakeLedgerApi::start().await;
    api.fail_result_reports(&[Failure::StaleFencingToken]);
    let command = api.enqueue("list_players", json!({}));
    let agent = RunningAgent::start(&api, RecordingExecutor::succeeding(&api));
    events_once(&api, "the refused result", |events| {
        events.contains(&succeeded_result(command, 409))
    })
    .await;
    // Enough claim cycles for a retry to have shown up.
    let events = events_once(&api, "three claims after the result", |events| {
        let after_result = events
            .iter()
            .skip_while(|event| !matches!(event, LedgerEvent::Result { .. }))
            .filter(|event| matches!(event, LedgerEvent::Claim { .. }))
            .count();
        after_result >= 3
    })
    .await;
    agent.stop().await;
    assert_eq!(
        steps(&events),
        vec![
            executing(command, 200),
            effect("list_players"),
            succeeded_result(command, 409),
        ]
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

#[tokio::test]
async fn host_agent_ledger_refuses_arguments_that_fail_revalidation_without_acting() {
    let api = FakeLedgerApi::start().await;
    let mut bad_digest = mission_arguments();
    bad_digest["artifact_sha256"] = json!("9F86D081");
    let mut extra_key = mission_arguments();
    extra_key["unit"] = json!("other.service");
    let refused = [
        (
            api.enqueue("broadcast", json!({"message": "Restart in 5 minutes"})),
            "the host agent refused the command: broadcast runs in the game runtime, not on the \
             host agent",
        ),
        (
            api.enqueue("kick", json!({"arma_id": "a", "runtime_session_id": "b"})),
            "the host agent refused the command: kick runs in the game runtime, not on the host \
             agent",
        ),
        (
            api.enqueue("reboot", json!({})),
            "the host agent refused the command: the host agent does not perform the action \
             \"reboot\"",
        ),
        (
            api.enqueue("restart", json!({"force": true})),
            "the host agent refused the command: restart does not accept the argument \"force\"",
        ),
        (
            api.enqueue("restart_with_mission", bad_digest),
            "the host agent refused the command: restart_with_mission needs artifact_sha256 to be \
             64 lowercase hex digits",
        ),
        (
            api.enqueue("restart_with_mission", extra_key),
            "the host agent refused the command: restart_with_mission does not accept the \
             argument \"unit\"",
        ),
    ];
    let agent = RunningAgent::start(&api, RecordingExecutor::succeeding(&api));
    let events = events_once(&api, "every result", |events| {
        steps(events).len() >= refused.len()
    })
    .await;
    agent.stop().await;
    let expected: Vec<LedgerEvent> = refused
        .iter()
        .map(|(command_id, reason)| LedgerEvent::Result {
            command_id: *command_id,
            body: json!({
                "fencing_token": FENCING_TOKEN,
                "succeeded": false,
                "failure_reason": reason,
            }),
            answered: 200,
        })
        .collect();
    assert_eq!(
        steps(&events),
        expected,
        "no executing report and no effect"
    );
}

#[tokio::test]
async fn host_agent_ledger_backs_off_after_transport_errors_and_recovers() {
    let api = FakeLedgerApi::start().await;
    api.fail_claims(&[Failure::ServiceUnavailable, Failure::ServiceUnavailable]);
    let command = api.enqueue("start", json!({}));
    let agent = RunningAgent::start(&api, RecordingExecutor::succeeding(&api));
    let events = events_once(&api, "the result", result_reported).await;
    agent.stop().await;
    assert!(claims(&events) >= 3, "two failed claims, then the command");
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
async fn host_agent_ledger_unreachable_api_is_a_transient_error() {
    let unused = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = unused.local_addr().unwrap();
    drop(unused);
    let base = reqwest::Url::parse(&format!("http://{address}/")).unwrap();
    let ledger = LedgerApi::new(&base, &SecretText::new(CREDENTIAL)).unwrap();
    let error = ledger.claim().await.unwrap_err();
    assert!(matches!(error, LedgerError::Unreachable(_)), "{error}");
    assert!(error.is_transient());
}

#[tokio::test]
async fn host_agent_ledger_claims_are_read_from_the_contract_shape() {
    let api = FakeLedgerApi::start().await;
    let command = api.enqueue("restart_with_mission", mission_arguments());
    let ledger = LedgerApi::new(&api.base_url(), &SecretText::new(CREDENTIAL)).unwrap();
    let ClaimOutcome::Claimed(claimed) = ledger.claim().await.unwrap() else {
        panic!("a claimed command");
    };
    assert_eq!(claimed.command_id, command);
    assert_eq!(claimed.action, "restart_with_mission");
    assert_eq!(claimed.arguments, mission_arguments());
    assert_eq!(claimed.fencing_token, FENCING_TOKEN);
    assert_eq!(
        claimed.lease_expires_at.to_rfc3339(),
        "2026-09-23T12:00:30.500+00:00"
    );
    assert_eq!(
        ledger.claim().await.unwrap(),
        ClaimOutcome::NothingClaimable
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
        server: "127.0.0.1:9".parse().unwrap(),
        password: SecretText::new("range-master"),
        timings: RconTimings::default(),
    })
    .await
    .unwrap();
    let process_control = ProcessControl::new(ProcessControlSettings {
        systemctl_program: systemctl.program(),
        unit: SystemdUnitName::parse("tbd-reforger.service").unwrap(),
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
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    fs::write(&path, SERVER_CONFIG).unwrap();
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
async fn host_agent_ledger_mission_restart_reports_executing_before_the_config_changes() {
    let api = FakeLedgerApi::start().await;
    let systemctl = FakeSystemctl::install(&RUNNING_UNIT).await;
    let (_directory, config) = server_config_file();
    let observed = config.clone();
    api.observe_on_executing(move || fs::read_to_string(&observed).unwrap());
    api.fail_executing_reports(&[Failure::ServiceUnavailable]);
    let command = api.enqueue("restart_with_mission", mission_arguments());
    let agent = RunningAgent::start(&api, mission_host(&systemctl, &config).await);
    let events = events_once(&api, "the result", result_reported).await;
    agent.stop().await;
    assert_eq!(
        steps(&events),
        vec![
            LedgerEvent::Observed(SERVER_CONFIG.to_owned()),
            executing(command, 503),
            LedgerEvent::Observed(SERVER_CONFIG.to_owned()),
            executing(command, 200),
            LedgerEvent::Result {
                command_id: command,
                body: json!({
                    "fencing_token": FENCING_TOKEN,
                    "succeeded": true,
                    "outcome": {
                        "scenario_id": DEV_POC,
                        "unit_active_state": "active",
                        "config_path": config.display().to_string(),
                    },
                }),
                answered: 200,
            },
        ],
        "the config still named the old scenario at every executing report"
    );
    assert_eq!(
        fs::read_to_string(&config).unwrap(),
        SERVER_CONFIG.replace(CAMPAIGN, DEV_POC)
    );
    assert_eq!(
        systemctl.invocations().first().map(String::as_str),
        Some("--user restart tbd-reforger.service")
    );
}

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
