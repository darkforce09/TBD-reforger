//! The relay itself: the loopback listener, the forwarded exchange, and the withheld answer.
//!
//! - **Role:** [`start`] binds the control socket and the listener and serves both on the caller's
//!   runtime; [`serve`] runs them on a runtime of its own until SIGTERM or SIGINT. Each exchange is
//!   read whole, forwarded to the upstream origin, read back whole and handed to the drop policy,
//!   which passes the answer back unchanged or withholds it.
//! - **Position:** under [`super`]; the command line calls [`serve`], and the tests call [`start`]
//!   against a stub API.
//! - **Signals & state:** one [`DropPolicy`] shared by every connection and the control socket; one
//!   upstream HTTP client and its connection pool; the [`RelayLog`] sink.
//! - **Invariants:**
//!   - The upstream receives the method, path, query, end-to-end headers and body the agent sent,
//!     and the agent receives the upstream's status, end-to-end headers and body. Hop-by-hop
//!     headers stay on their own connection, and the relay adds no header of its own.
//!   - A withheld answer is held for the settings' hold time, then its connection is aborted: no
//!     byte of the answer is ever written.
//!   - Headers are never logged or stored; a log line names at most the method, the path, the
//!     command and its fencing token.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{self, HeaderMap, HeaderName};
use axum::http::{StatusCode, request};
use axum::response::{IntoResponse, Response};
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use super::connection_abort::{AbortableListener, ConnectionAbort};
use super::control_socket::ControlSocket;
use super::drop_policy::{Decision, DropPolicy, ExecutorExchange, RelayIdentity};
use super::relay_settings::{RelaySettings, UpstreamOrigin};

/// Largest request or answer body the relay carries; the executor documents are a few kilobytes.
const MAXIMUM_BODY_BYTES: usize = 1024 * 1024;
const UPSTREAM_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Longer than the agent's own timeout, so the agent, not the relay, gives up on a slow answer.
const UPSTREAM_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
/// Headers that belong to one connection rather than to the message (RFC 9110 §7.6.1).
const HOP_BY_HOP_HEADERS: [&str; 8] = [
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// Where the relay writes its event lines.
#[derive(Debug, Clone)]
pub enum RelayLog {
    /// Standard error, which the unit's journal keeps.
    StandardError,
    /// Lines kept in memory, for the tests.
    Captured(Arc<Mutex<Vec<String>>>),
}

impl RelayLog {
    /// Write one event line.
    pub(super) fn line(&self, text: impl Into<String>) {
        let text = text.into();
        match self {
            Self::StandardError => eprintln!("acknowledgement-dropping-relay: {text}"),
            Self::Captured(lines) => lines
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(text),
        }
    }
}

/// A relay serving on the runtime that started it.
#[derive(Debug)]
pub struct RunningRelay {
    listen: SocketAddr,
    stop: oneshot::Sender<()>,
    task: JoinHandle<Result<()>>,
}

impl RunningRelay {
    /// The address the relay listens on.
    pub fn listen_address(&self) -> SocketAddr {
        self.listen
    }

    /// Serve until `stop` completes, then shut down.
    ///
    /// # Errors
    ///
    /// The listener or the control socket stopped serving before `stop` completed.
    pub async fn run_until(self, stop: impl Future<Output = ()>) -> Result<()> {
        let Self {
            stop: stop_sender,
            mut task,
            ..
        } = self;
        tokio::select! {
            () = stop => {}
            ended = &mut task => return ended.context("the relay task failed")?,
        }
        // The task ends as soon as it sees the signal, or has ended already.
        let _ = stop_sender.send(());
        task.await.context("the relay task failed")?
    }

    /// Stop accepting connections and remove the control socket; exchanges in flight end on
    /// their own.
    ///
    /// # Errors
    ///
    /// The listener or the control socket had stopped serving on its own.
    pub async fn shut_down(self) -> Result<()> {
        self.run_until(std::future::ready(())).await
    }
}

/// Bind the control socket and the listener, and serve both on the current runtime.
///
/// # Errors
///
/// The listen address is not a loopback address, the control socket cannot be created (a relay
/// answers on it, or its path holds something else), the listen address cannot be bound, or the
/// upstream client cannot be built.
pub async fn start(settings: RelaySettings, log: RelayLog) -> Result<RunningRelay> {
    if !settings.listen.ip().is_loopback() {
        bail!(
            "{} is not a loopback address: the relay listens only on 127.0.0.0/8 or ::1",
            settings.listen
        );
    }
    let control = ControlSocket::bind(&settings.control_socket)?;
    let listener = TcpListener::bind(settings.listen)
        .await
        .with_context(|| format!("cannot listen on {}", settings.listen))?;
    let listen = listener.local_addr()?;
    let policy = Arc::new(DropPolicy::new(RelayIdentity {
        listen: listen.to_string(),
        upstream: settings.upstream.as_str().to_string(),
        withhold: settings.withhold,
    }));
    let state = Arc::new(RelayState {
        upstream: Upstream::new(settings.upstream.clone())?,
        policy: policy.clone(),
        log: log.clone(),
        withhold: settings.withhold,
    });
    let application = Router::new()
        .fallback(relay_exchange)
        .with_state(state)
        .into_make_service_with_connect_info::<ConnectionAbort>();
    let (stop, stopped) = oneshot::channel::<()>();
    let control_log = log.clone();
    let task = tokio::spawn(async move {
        tokio::select! {
            served = axum::serve(AbortableListener::new(listener), application).into_future() => {
                served.context("the relay listener stopped")?;
                bail!("the relay listener stopped")
            }
            () = control.serve(policy, control_log) => Err(anyhow!("the control socket stopped")),
            _ = stopped => Ok(()),
        }
    });
    log.line(format!(
        "listening on {listen}, forwarding to {}, controlled on {}",
        settings.upstream.as_str(),
        settings.control_socket.display()
    ));
    Ok(RunningRelay { listen, stop, task })
}

/// Run the relay on a runtime of its own until SIGTERM or SIGINT.
///
/// # Errors
///
/// The runtime, the control socket or the listener cannot be set up, or either stops serving.
pub fn serve(settings: RelaySettings, log: RelayLog) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .context("cannot build the relay's runtime")?;
    runtime.block_on(async move {
        let mut terminate = signal(SignalKind::terminate()).context("cannot watch for SIGTERM")?;
        let mut interrupt = signal(SignalKind::interrupt()).context("cannot watch for SIGINT")?;
        let relay = start(settings, log.clone()).await?;
        let stop = async move {
            tokio::select! {
                _ = terminate.recv() => {}
                _ = interrupt.recv() => {}
            }
        };
        let outcome = relay.run_until(stop).await;
        log.line("stopped");
        outcome
    })
}

struct RelayState {
    upstream: Upstream,
    policy: Arc<DropPolicy>,
    log: RelayLog,
    withhold: Duration,
}

/// Forward one exchange, then pass its answer back or withhold it.
async fn relay_exchange(
    State(state): State<Arc<RelayState>>,
    ConnectInfo(connection): ConnectInfo<ConnectionAbort>,
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let Ok(body) = axum::body::to_bytes(body, MAXIMUM_BODY_BYTES).await else {
        return relay_failure(
            StatusCode::BAD_REQUEST,
            "the request body could not be read whole",
        );
    };
    let exchange = ExecutorExchange::classify(&parts.method, parts.uri.path());
    let answer = match state.upstream.forward(&parts, body.clone()).await {
        Ok(answer) => answer,
        Err(error) => {
            state.log.line(format!(
                "{} {}: the upstream did not answer: {error:#}",
                parts.method,
                parts.uri.path()
            ));
            return relay_failure(StatusCode::BAD_GATEWAY, "the upstream API did not answer");
        }
    };
    match state
        .policy
        .decide(exchange.as_ref(), answer.status, &body, &answer.body)
    {
        Decision::Forward => answer.into_response(),
        Decision::Withhold(record) => {
            state.log.line(record.announcement(state.withhold));
            tokio::time::sleep(state.withhold).await;
            connection.abort();
            // Every write on the aborted connection fails, so hyper closes it without sending
            // this placeholder or any byte of the upstream's answer.
            StatusCode::SERVICE_UNAVAILABLE.into_response()
        }
    }
}

/// An answer the relay writes itself, when it cannot carry the exchange.
fn relay_failure(status: StatusCode, reason: &str) -> Response {
    (
        status,
        format!("acknowledgement-dropping-relay: {reason}\n"),
    )
        .into_response()
}

/// The upstream's answer, read whole.
struct UpstreamAnswer {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl IntoResponse for UpstreamAnswer {
    fn into_response(self) -> Response {
        let mut response = Response::new(Body::from(self.body));
        *response.status_mut() = self.status;
        *response.headers_mut() = self.headers;
        response
    }
}

/// The upstream origin and the client that reaches it.
struct Upstream {
    client: reqwest::Client,
    origin: UpstreamOrigin,
}

impl Upstream {
    fn new(origin: UpstreamOrigin) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(UPSTREAM_CONNECT_TIMEOUT)
            .timeout(UPSTREAM_REQUEST_TIMEOUT);
        if origin.host() == "localhost" {
            // The name is pinned to the loopback address the settings checked, never resolved.
            builder = builder.resolve(origin.host(), origin.address());
        }
        let client = builder
            .build()
            .context("cannot build the upstream client")?;
        Ok(Self { client, origin })
    }

    /// Send the exchange `parts` and `body` describe to the upstream and read its answer whole.
    async fn forward(&self, parts: &request::Parts, body: Bytes) -> Result<UpstreamAnswer> {
        let target = parts
            .uri
            .path_and_query()
            .map_or("/", |path_and_query| path_and_query.as_str());
        let url = self.origin.target(target)?;
        let skipped = [header::HOST, header::CONTENT_LENGTH];
        let mut request = self
            .client
            .request(parts.method.clone(), url)
            .headers(end_to_end_headers(&parts.headers, &skipped));
        if !body.is_empty() || parts.headers.contains_key(header::CONTENT_LENGTH) {
            request = request.body(body);
        }
        let mut response = request.send().await?;
        let status = response.status();
        let headers = end_to_end_headers(response.headers(), &[]);
        let mut collected = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if collected.len() + chunk.len() > MAXIMUM_BODY_BYTES {
                bail!("the upstream answer is longer than {MAXIMUM_BODY_BYTES} bytes");
            }
            collected.extend_from_slice(&chunk);
        }
        Ok(UpstreamAnswer {
            status,
            headers,
            body: Bytes::from(collected),
        })
    }
}

/// `headers` without the hop-by-hop headers, those the `Connection` header names, and `skipped`;
/// the `Authorization` value is marked sensitive so no debug output shows it.
fn end_to_end_headers(headers: &HeaderMap, skipped: &[HeaderName]) -> HeaderMap {
    let named_by_connection: Vec<String> = headers
        .get_all(header::CONNECTION)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(|token| token.trim().to_ascii_lowercase())
        .collect();
    let mut kept = HeaderMap::with_capacity(headers.len());
    for (name, value) in headers {
        let spelled = name.as_str();
        if HOP_BY_HOP_HEADERS.contains(&spelled)
            || skipped.contains(name)
            || named_by_connection.iter().any(|named| named == spelled)
        {
            continue;
        }
        let mut value = value.clone();
        if name == header::AUTHORIZATION {
            value.set_sensitive(true);
        }
        kept.append(name.clone(), value);
    }
    kept
}

#[cfg(test)]
#[path = "tests/relay_tests.rs"]
mod tests;
