//! A stand-in for the staging API that the load run tests drive: single-use refresh rotation,
//! optionally slow, per-account write checks, conditional, quick, slow, streamed, failing, hanging
//! and aborted reads, and a ledger of every request with its source address and arrival instant.
//!
//! Tokens are `refresh-<account>-<generation>` and `access-<account>-<generation>`; the account
//! file's tokens are generation 0, and each refresh answers the next generation. Account `k`
//! writes to `em-<k mod events>`, claims `slot-<k mod events>-<k div events>`, bookmarks
//! `mission-<k mod events>` and saves fire missions for `event-<k mod events>`.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use axum::body::{Body, Bytes};
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, Method, StatusCode, Uri, header};
use axum::response::Response;
use futures::stream;
use serde_json::{Value, json};

/// How long `/api/v1/slow` holds its answer.
pub(super) const SLOW_ANSWER: Duration = Duration::from_millis(200);
/// How long `/api/v1/quick` holds its answer.
pub(super) const QUICK_ANSWER: Duration = Duration::from_millis(5);
/// The route that answers after [`QUICK_ANSWER`].
pub(super) const QUICK_ROUTE: &str = "/api/v1/quick";
/// The pause before each of the three chunks of `/api/v1/streamed`.
pub(super) const STREAMED_CHUNK_PAUSE: Duration = Duration::from_millis(100);
/// The entity tag `/api/v1/conditional` answers with.
const ENTITY_TAG: &str = "\"stub-v1\"";
/// The refresh route.
pub(super) const REFRESH_ROUTE: &str = "/api/v1/auth/refresh";

/// One request as the stub saw it.
#[derive(Debug, Clone)]
pub(super) struct StubHit {
    pub(super) peer: IpAddr,
    pub(super) path: String,
    pub(super) at: Instant,
    /// The account of the presented bearer or refresh token.
    pub(super) account: Option<usize>,
    /// The bearer token is older than the account's newest generation.
    pub(super) stale_token: bool,
    pub(super) if_none_match: bool,
}

impl StubHit {
    pub(super) fn is_refresh(&self) -> bool {
        self.path == REFRESH_ROUTE
    }
}

#[derive(Default)]
struct Ledger {
    generations: HashMap<usize, u32>,
    refresh_replays: usize,
    registered: HashMap<usize, bool>,
    bookmarked: HashMap<usize, bool>,
    invalid_writes: Vec<String>,
    hits: Vec<StubHit>,
}

struct StubState {
    ledger: Mutex<Ledger>,
    options: StubOptions,
}

/// How the stub behaves.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct StubOptions {
    /// Fixture events the per-account write checks assume.
    pub(super) events: usize,
    /// Every non-refresh request from this address answers 503.
    pub(super) failing_peer: Option<IpAddr>,
    /// How long every refresh waits before it is processed.
    pub(super) refresh_delay: Duration,
}

impl StubState {
    fn ledger(&self) -> MutexGuard<'_, Ledger> {
        self.ledger.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The running stub and its own runtime.
pub(super) struct StubApi {
    pub(super) origin: String,
    state: Arc<StubState>,
    runtime: Option<tokio::runtime::Runtime>,
}

impl StubApi {
    /// Serve on 127.0.0.1 with `events` fixture events; every non-refresh request from
    /// `failing_peer` answers 503.
    pub(super) fn start(events: usize, failing_peer: Option<IpAddr>) -> Self {
        Self::start_with(StubOptions {
            events,
            failing_peer,
            refresh_delay: Duration::ZERO,
        })
    }

    /// Serve on 127.0.0.1 as `options` describe.
    pub(super) fn start_with(options: StubOptions) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("stub runtime");
        let state = Arc::new(StubState {
            ledger: Mutex::default(),
            options,
        });
        let listener = runtime
            .block_on(tokio::net::TcpListener::bind(("127.0.0.1", 0)))
            .expect("stub listener");
        let origin = format!("http://{}", listener.local_addr().expect("stub address"));
        let app = axum::Router::new()
            .fallback(answer)
            .with_state(Arc::clone(&state));
        runtime.spawn(async move {
            let service = app.into_make_service_with_connect_info::<SocketAddr>();
            axum::serve(listener, service).await.expect("stub server");
        });
        Self {
            origin,
            state,
            runtime: Some(runtime),
        }
    }

    pub(super) fn hits(&self) -> Vec<StubHit> {
        self.state.ledger().hits.clone()
    }

    pub(super) fn refresh_replays(&self) -> usize {
        self.state.ledger().refresh_replays
    }

    pub(super) fn invalid_writes(&self) -> Vec<String> {
        self.state.ledger().invalid_writes.clone()
    }

    /// The newest token generation of `account`.
    pub(super) fn generation(&self, account: usize) -> u32 {
        self.state
            .ledger()
            .generations
            .get(&account)
            .copied()
            .unwrap_or(0)
    }
}

impl Drop for StubApi {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

async fn answer(
    State(state): State<Arc<StubState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let at = Instant::now();
    let path = uri.path().to_owned();
    if method == Method::POST && path == REFRESH_ROUTE {
        tokio::time::sleep(state.options.refresh_delay).await;
        return refresh(&state, peer.ip(), at, &body);
    }
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|token| token_parts(token, "access-"));
    let stale_token = {
        let mut ledger = state.ledger();
        let stale = bearer.is_some_and(|(account, generation)| {
            ledger.generations.get(&account).copied().unwrap_or(0) != generation
        });
        ledger.hits.push(StubHit {
            peer: peer.ip(),
            path: path.clone(),
            at,
            account: bearer.map(|(account, _)| account),
            stale_token: stale,
            if_none_match: headers.contains_key(header::IF_NONE_MATCH),
        });
        stale
    };
    let Some((account, _)) = bearer.filter(|_| !stale_token) else {
        return status(StatusCode::UNAUTHORIZED);
    };
    if state.options.failing_peer == Some(peer.ip()) {
        return status(StatusCode::SERVICE_UNAVAILABLE);
    }
    route(&state, &method, &path, account, &headers, &body).await
}

fn refresh(state: &StubState, peer: IpAddr, at: Instant, body: &[u8]) -> Response {
    let presented = serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| value.get("refresh_token")?.as_str().map(str::to_owned))
        .and_then(|token| token_parts(&token, "refresh-"));
    let mut ledger = state.ledger();
    ledger.hits.push(StubHit {
        peer,
        path: REFRESH_ROUTE.to_owned(),
        at,
        account: presented.map(|(account, _)| account),
        stale_token: false,
        if_none_match: false,
    });
    let Some((account, generation)) = presented else {
        return status(StatusCode::BAD_REQUEST);
    };
    let newest = ledger.generations.get(&account).copied().unwrap_or(0);
    if generation != newest {
        ledger.refresh_replays += 1;
        return status(StatusCode::UNAUTHORIZED);
    }
    ledger.generations.insert(account, newest + 1);
    json_answer(
        StatusCode::OK,
        &json!({
            "access_token": format!("access-{account}-{}", newest + 1),
            "expires_at": "2099-01-01T00:00:00Z",
            "refresh_token": format!("refresh-{account}-{}", newest + 1),
            "token_type": "Bearer",
        }),
    )
}

async fn route(
    state: &StubState,
    method: &Method,
    path: &str,
    account: usize,
    headers: &HeaderMap,
    body: &[u8],
) -> Response {
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    match (method.as_str(), segments.as_slice()) {
        ("GET", ["api", "v1", "events"]) => json_answer(StatusCode::OK, &json!([])),
        ("GET", ["api", "v1", "conditional"]) => conditional(headers),
        ("GET", ["api", "v1", "not-modified"]) => status(StatusCode::NOT_MODIFIED),
        ("GET", ["api", "v1", "quick"]) => {
            tokio::time::sleep(QUICK_ANSWER).await;
            json_answer(StatusCode::OK, &json!({ "quick": true }))
        }
        ("GET", ["api", "v1", "slow"]) => {
            tokio::time::sleep(SLOW_ANSWER).await;
            json_answer(StatusCode::OK, &json!({ "slow": true }))
        }
        ("GET", ["api", "v1", "streamed"]) => streamed(),
        ("GET", ["api", "v1", "fail"]) => status(StatusCode::INTERNAL_SERVER_ERROR),
        ("GET", ["api", "v1", "hang"]) => {
            tokio::time::sleep(Duration::from_secs(30)).await;
            status(StatusCode::OK)
        }
        ("GET", ["api", "v1", "abort"]) => aborted(),
        ("POST", ["api", "v1", "event-missions", attachment, "register"]) => {
            state.register(account, attachment, Some(body))
        }
        ("DELETE", ["api", "v1", "event-missions", attachment, "register"]) => {
            state.register(account, attachment, None)
        }
        ("POST", ["api", "v1", "missions", mission, "bookmark"]) => {
            state.bookmark(account, mission, true)
        }
        ("DELETE", ["api", "v1", "missions", mission, "bookmark"]) => {
            state.bookmark(account, mission, false)
        }
        ("POST", ["api", "v1", "fire-missions"]) => state.fire_mission(account, body),
        _ => status(StatusCode::NOT_FOUND),
    }
}

impl StubState {
    /// A claim carries its body; a withdrawal carries none.
    fn register(&self, account: usize, attachment: &str, claim: Option<&[u8]>) -> Response {
        let event = account % self.options.events;
        let mut problems = Vec::new();
        if attachment != format!("em-{event}") {
            problems.push(format!("account {account} registered on {attachment}"));
        }
        if let Some(body) = claim {
            let slot = body_field(body, "slot_id");
            let own = format!("slot-{event}-{}", account / self.options.events);
            if slot.as_deref() != Some(own.as_str()) {
                problems.push(format!("account {account} claimed {slot:?}, not {own}"));
            }
        }
        let mut ledger = self.ledger();
        if ledger.registered.get(&account).copied().unwrap_or(false) == claim.is_some() {
            problems.push(format!(
                "account {account} repeated its last registration change"
            ));
        }
        if problems.is_empty() {
            ledger.registered.insert(account, claim.is_some());
            json_answer(StatusCode::OK, &json!({ "registered": claim.is_some() }))
        } else {
            ledger.invalid_writes.extend(problems);
            status(StatusCode::CONFLICT)
        }
    }

    fn bookmark(&self, account: usize, mission: &str, add: bool) -> Response {
        let own = format!("mission-{}", account % self.options.events);
        let mut ledger = self.ledger();
        let marked = ledger.bookmarked.get(&account).copied().unwrap_or(false);
        let problem = if mission != own {
            Some(format!("account {account} bookmarked {mission}, not {own}"))
        } else if marked == add {
            Some(format!(
                "account {account} repeated its last bookmark change"
            ))
        } else {
            None
        };
        match problem {
            None => {
                ledger.bookmarked.insert(account, add);
                status(StatusCode::NO_CONTENT)
            }
            Some(problem) => {
                ledger.invalid_writes.push(problem);
                status(StatusCode::CONFLICT)
            }
        }
    }

    fn fire_mission(&self, account: usize, body: &[u8]) -> Response {
        let own = format!("event-{}", account % self.options.events);
        let event = body_field(body, "event_id");
        if event.as_deref() == Some(own.as_str()) {
            json_answer(StatusCode::CREATED, &json!({ "saved": true }))
        } else {
            let problem = format!("account {account} saved a fire mission for {event:?}");
            self.ledger().invalid_writes.push(problem);
            status(StatusCode::UNPROCESSABLE_ENTITY)
        }
    }
}

fn conditional(headers: &HeaderMap) -> Response {
    let presented = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok());
    if presented == Some(ENTITY_TAG) {
        return status(StatusCode::NOT_MODIFIED);
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::ETAG, ENTITY_TAG)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"version":1}"#))
        .expect("conditional answer")
}

/// Headers at once, then three chunks, each after a pause.
fn streamed() -> Response {
    let chunks = stream::unfold(0u32, |sent| async move {
        if sent == 3 {
            return None;
        }
        tokio::time::sleep(STREAMED_CHUNK_PAUSE).await;
        Some((Ok::<_, std::io::Error>(Bytes::from_static(b"[]")), sent + 1))
    });
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from_stream(chunks))
        .expect("streamed answer")
}

/// A 200 whose body breaks off, which drops the connection mid-answer.
fn aborted() -> Response {
    let chunks = stream::iter([
        Ok(Bytes::from_static(b"{\"partial\":")),
        Err(std::io::Error::other("the stub breaks the body off")),
    ]);
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from_stream(chunks))
        .expect("aborted answer")
}

fn token_parts(token: &str, prefix: &str) -> Option<(usize, u32)> {
    let (account, generation) = token.strip_prefix(prefix)?.split_once('-')?;
    Some((account.parse().ok()?, generation.parse().ok()?))
}

fn body_field(body: &[u8], field: &str) -> Option<String> {
    serde_json::from_slice::<Value>(body)
        .ok()?
        .get(field)?
        .as_str()
        .map(str::to_owned)
}

fn status(code: StatusCode) -> Response {
    Response::builder()
        .status(code)
        .body(Body::empty())
        .expect("status answer")
}

fn json_answer(code: StatusCode, value: &Value) -> Response {
    Response::builder()
        .status(code)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .expect("json answer")
}
