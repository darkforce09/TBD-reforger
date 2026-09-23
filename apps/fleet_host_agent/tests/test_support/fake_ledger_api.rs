//! A stand-in for the platform API's executor routes, imitating the fleet command ledger's
//! contract: `POST /api/v1/fleet-executor/commands/claim` answers queued commands with 200 and
//! otherwise 204; `.../{commandId}/executing` and `.../{commandId}/result` answer 200, or the
//! failures a test queues (503, or 409 STALE_FENCING_TOKEN). Every request is recorded in
//! order, and an executor under test records its effects in the same log. An observer a test
//! installs runs as each `executing` report arrives, and what it sees is logged just before that
//! report.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use uuid::Uuid;

/// The fencing token of every command the stand-in hands out.
pub const FENCING_TOKEN: i64 = 7;

#[derive(Debug, Clone, PartialEq)]
pub enum LedgerEvent {
    Claim {
        authorization: Option<String>,
    },
    Executing {
        command_id: Uuid,
        body: Value,
        answered: u16,
    },
    Result {
        command_id: Uuid,
        body: Value,
        answered: u16,
    },
    /// Recorded by the executor under test when it performs a command.
    Effect {
        action: String,
    },
    /// What the executing observer saw as an `executing` report arrived.
    Observed(String),
}

/// A failure answered instead of a success.
#[derive(Debug, Clone, Copy)]
pub enum Failure {
    ServiceUnavailable,
    StaleFencingToken,
}

impl Failure {
    fn response(self) -> (StatusCode, Json<Value>) {
        match self {
            Self::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"error": "the API is restarting"})),
            ),
            Self::StaleFencingToken => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "the claim is no longer current",
                    "details": {"code": "STALE_FENCING_TOKEN", "state": "queued"},
                })),
            ),
        }
    }
}

type ExecutingObserver = Box<dyn Fn() -> String + Send>;

#[derive(Default)]
struct LedgerState {
    queue: VecDeque<Value>,
    claim_failures: VecDeque<Failure>,
    executing_failures: VecDeque<Failure>,
    result_failures: VecDeque<Failure>,
    executing_observer: Option<ExecutingObserver>,
    events: Vec<LedgerEvent>,
}

/// Shared handle to the stand-in's event log.
#[derive(Clone)]
pub struct EventLog(Arc<Mutex<LedgerState>>);

impl EventLog {
    pub fn record_effect(&self, action: &str) {
        self.lock().events.push(LedgerEvent::Effect {
            action: action.to_owned(),
        });
    }

    fn lock(&self) -> MutexGuard<'_, LedgerState> {
        self.0.lock().expect("an unpoisoned ledger")
    }
}

pub struct FakeLedgerApi {
    address: SocketAddr,
    log: EventLog,
    task: JoinHandle<()>,
}

impl FakeLedgerApi {
    pub async fn start() -> Self {
        let log = EventLog(Arc::new(Mutex::new(LedgerState::default())));
        let router = Router::new()
            .route("/api/v1/fleet-executor/commands/claim", post(claim))
            .route(
                "/api/v1/fleet-executor/commands/{command_id}/executing",
                post(executing),
            )
            .route(
                "/api/v1/fleet-executor/commands/{command_id}/result",
                post(result),
            )
            .with_state(log.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        let address = listener.local_addr().expect("a bound listener");
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("the stand-in API serves");
        });
        Self { address, log, task }
    }

    pub fn base_url(&self) -> reqwest::Url {
        reqwest::Url::parse(&format!("http://{}/", self.address)).expect("a URL")
    }

    pub fn event_log(&self) -> EventLog {
        self.log.clone()
    }

    /// Queues a command for the next claim and returns its id.
    pub fn enqueue(&self, action: &str, arguments: Value) -> Uuid {
        let command_id = Uuid::new_v4();
        self.log.lock().queue.push_back(json!({
            "command_id": command_id,
            "server_id": Uuid::new_v4(),
            "action": action,
            "arguments": arguments,
            "fencing_token": FENCING_TOKEN,
            "lease_expires_at": "2026-09-23T12:00:30.5Z",
        }));
        command_id
    }

    pub fn fail_claims(&self, failures: &[Failure]) {
        self.log.lock().claim_failures.extend(failures);
    }

    pub fn fail_executing_reports(&self, failures: &[Failure]) {
        self.log.lock().executing_failures.extend(failures);
    }

    pub fn fail_result_reports(&self, failures: &[Failure]) {
        self.log.lock().result_failures.extend(failures);
    }

    /// Runs `observer` as each `executing` report arrives and logs what it returns.
    pub fn observe_on_executing(&self, observer: impl Fn() -> String + Send + 'static) {
        self.log.lock().executing_observer = Some(Box::new(observer));
    }

    pub fn events(&self) -> Vec<LedgerEvent> {
        self.log.lock().events.clone()
    }
}

impl Drop for FakeLedgerApi {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn claim(State(log): State<EventLog>, headers: HeaderMap) -> Response {
    let mut ledger = log.lock();
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    ledger.events.push(LedgerEvent::Claim { authorization });
    if let Some(failure) = ledger.claim_failures.pop_front() {
        return failure.response().into_response();
    }
    match ledger.queue.pop_front() {
        Some(command) => (StatusCode::OK, Json(command)).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    }
}

async fn executing(
    State(log): State<EventLog>,
    Path(command_id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Response {
    let mut ledger = log.lock();
    let observed = ledger.executing_observer.as_ref().map(|observe| observe());
    if let Some(observed) = observed {
        ledger.events.push(LedgerEvent::Observed(observed));
    }
    let failure = ledger.executing_failures.pop_front();
    let (status, answer) = failure.map_or_else(
        || {
            (
                StatusCode::OK,
                Json(json!({"id": command_id, "state": "executing"})),
            )
        },
        Failure::response,
    );
    ledger.events.push(LedgerEvent::Executing {
        command_id,
        body,
        answered: status.as_u16(),
    });
    (status, answer).into_response()
}

async fn result(
    State(log): State<EventLog>,
    Path(command_id): Path<Uuid>,
    Json(body): Json<Value>,
) -> Response {
    let mut ledger = log.lock();
    let failure = ledger.result_failures.pop_front();
    let state = if body["succeeded"] == true {
        "succeeded"
    } else {
        "failed"
    };
    let (status, answer) = failure.map_or_else(
        || {
            (
                StatusCode::OK,
                Json(json!({"id": command_id, "state": state})),
            )
        },
        Failure::response,
    );
    ledger.events.push(LedgerEvent::Result {
        command_id,
        body,
        answered: status.as_u16(),
    });
    (status, answer).into_response()
}
