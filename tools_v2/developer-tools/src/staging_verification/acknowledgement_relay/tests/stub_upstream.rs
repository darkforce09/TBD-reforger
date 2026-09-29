//! A stand-in for the API behind the relay in the relay tests: it answers every request with the
//! next planned answer and keeps a ledger of every request exactly as it arrived. Also the
//! temporary folder the tests put control sockets in.

use std::collections::VecDeque;
use std::fs;
use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::response::Response;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// One request as the stub received it.
#[derive(Debug, Clone)]
pub(super) struct ReceivedRequest {
    pub(super) method: Method,
    pub(super) uri: Uri,
    pub(super) headers: HeaderMap,
    pub(super) body: Bytes,
}

/// One answer the stub gives, in the order planned.
#[derive(Debug, Clone)]
pub(super) struct PlannedAnswer {
    pub(super) status: StatusCode,
    pub(super) headers: Vec<(&'static str, &'static str)>,
    pub(super) body: String,
}

/// A `200` claim answer for `command_id` with `fencing_token`, shaped as the API answers.
pub(super) fn claim_answer(command_id: &str, fencing_token: i64) -> PlannedAnswer {
    let body = json!({
        "command_id": command_id,
        "server_id": "5d1b7c1e-0000-4000-8000-000000000005",
        "action": "restart",
        "arguments": {},
        "fencing_token": fencing_token,
        "lease_expires_at": "2026-09-29T12:00:00Z",
    });
    json_answer(StatusCode::OK, &body.to_string())
}

/// The `204` answer to a claim when nothing is claimable.
pub(super) fn nothing_claimable() -> PlannedAnswer {
    PlannedAnswer {
        status: StatusCode::NO_CONTENT,
        headers: Vec::new(),
        body: String::new(),
    }
}

/// A `200` receipt answer to a result report for `command_id`.
pub(super) fn result_receipt(command_id: &str) -> PlannedAnswer {
    let body = json!({ "id": command_id, "state": "succeeded", "attempts": 1 });
    json_answer(StatusCode::OK, &body.to_string())
}

/// A `409` answer to a result report whose command has already finished.
pub(super) fn result_conflict() -> PlannedAnswer {
    let body = json!({
        "error": "the command is not executing",
        "details": { "code": "COMMAND_NOT_EXECUTING", "state": "succeeded" },
    });
    json_answer(StatusCode::CONFLICT, &body.to_string())
}

fn json_answer(status: StatusCode, body: &str) -> PlannedAnswer {
    PlannedAnswer {
        status,
        headers: vec![("content-type", "application/json")],
        body: body.to_string(),
    }
}

#[derive(Debug, Default)]
struct StubLedger {
    received: Vec<ReceivedRequest>,
    planned: VecDeque<PlannedAnswer>,
}

/// The running stub; dropping it stops the server.
#[derive(Debug)]
pub(super) struct StubUpstream {
    address: SocketAddr,
    ledger: Arc<Mutex<StubLedger>>,
    task: JoinHandle<()>,
}

impl StubUpstream {
    /// Serve on a free loopback port of the current runtime.
    pub(super) async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("stub listener");
        let address = listener.local_addr().expect("stub address");
        let ledger = Arc::new(Mutex::new(StubLedger::default()));
        let application = Router::new()
            .fallback(answer_planned)
            .with_state(ledger.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, application)
                .await
                .expect("stub server");
        });
        Self {
            address,
            ledger,
            task,
        }
    }

    /// `http://127.0.0.1:<port>`.
    pub(super) fn origin(&self) -> String {
        format!("http://{}", self.address)
    }

    /// Queue `answer` for the next request.
    pub(super) fn plan(&self, answer: PlannedAnswer) {
        lock(&self.ledger).planned.push_back(answer);
    }

    /// Every request received so far, in arrival order.
    pub(super) fn received(&self) -> Vec<ReceivedRequest> {
        lock(&self.ledger).received.clone()
    }
}

impl Drop for StubUpstream {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn lock(ledger: &Mutex<StubLedger>) -> MutexGuard<'_, StubLedger> {
    ledger.lock().unwrap_or_else(PoisonError::into_inner)
}

async fn answer_planned(
    State(ledger): State<Arc<Mutex<StubLedger>>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let planned = {
        let mut ledger = lock(&ledger);
        ledger.received.push(ReceivedRequest {
            method,
            uri,
            headers,
            body,
        });
        ledger.planned.pop_front()
    };
    let Some(planned) = planned else {
        let mut response = Response::new(Body::from("no answer planned"));
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        return response;
    };
    let mut response = Response::new(Body::from(planned.body));
    *response.status_mut() = planned.status;
    for (name, value) in planned.headers {
        response
            .headers_mut()
            .append(name, value.parse().expect("planned header value"));
    }
    response
}

/// A mode-700 folder under the system temporary folder, removed with everything in it on drop;
/// its path stays short enough for a Unix socket inside it.
#[derive(Debug)]
pub(super) struct TemporaryFolder(PathBuf);

impl TemporaryFolder {
    /// A new, empty folder whose name carries `label`.
    pub(super) fn new(label: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "tbd-relay-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&path).expect("temporary folder");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("folder mode");
        let folder = Self(path);
        assert!(
            folder.socket_path().as_os_str().len() < 100,
            "{} is too long for a Unix socket path",
            folder.socket_path().display()
        );
        folder
    }

    /// Where the tests put the control socket.
    pub(super) fn socket_path(&self) -> PathBuf {
        self.0.join("control.sock")
    }
}

impl Drop for TemporaryFolder {
    fn drop(&mut self) {
        // A test that failed may have left the folder half-populated; removal is best effort.
        let _ = fs::remove_dir_all(&self.0);
    }
}
