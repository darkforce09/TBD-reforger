//! One HTTP exchange of a virtual client, through its source address's guard.
//!
//! - **Role:** holds a client's address-bound HTTP client, waits until the address guard lets a
//!   request leave, sends it with its bearer, entity tag and body, reads the answer to the end of
//!   its body, and classifies the outcome.
//! - **Position:** both lanes of a virtual client send through one [`ClientConnection`]; the guard
//!   is the run's [`crate::source_address_pool::SourceAddressPool`] in the shared
//!   [`RunContext`].
//! - **Signals & state:** the HTTP client's connection pool, bound to the source address and
//!   keeping at most one idle connection; nothing else outlives an exchange.
//! - **Invariants:**
//!   - A request never leaves before its scheduled instant, and every instant an [`Exchange`]
//!     reports is an offset from the run start.
//!   - A request outside the auth routes takes its place in the address's order; an auth request
//!     waits outside that order while the auth ceiling holds it back, so it never stalls the
//!     requests queued behind it.
//!   - A token reaches only the `Authorization` header or the refresh body it was built into.

use std::net::IpAddr;
use std::time::Duration;

use reqwest::header::{CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use staging_load_plan::latency_recording::RequestOutcome;
use staging_load_plan::request_catalog::ResolvedRequest;
use staging_load_plan::workload_plan::HttpMethod;
use tokio::time::Instant;

use crate::account_rotation::SecretToken;
use crate::error::{Error, Result};
use crate::virtual_client::RunContext;

/// The `User-Agent` of every virtual client.
const USER_AGENT: &str = "tbd-staging-load-generation";

/// Whether an exchange hands the answer's body back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BodyUse {
    Discard,
    Keep,
}

/// One finished exchange, with instants as offsets from the run start.
pub(crate) struct Exchange {
    pub(crate) outcome: RequestOutcome,
    pub(crate) sent: Duration,
    pub(crate) finished: Duration,
    pub(crate) body: Option<Vec<u8>>,
    pub(crate) entity_tag: Option<String>,
    /// A per-address ceiling held the send back past its due instant.
    pub(crate) guard_delayed: bool,
}

/// What one request of a client carries besides the resolved request itself.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RequestHeaders<'a> {
    pub(crate) bearer: Option<&'a SecretToken>,
    pub(crate) if_none_match: Option<&'a str>,
}

/// A client's way out: its index, its source address and the HTTP client bound to it.
#[derive(Debug, Clone)]
pub(crate) struct ClientConnection {
    pub(crate) client: u32,
    pub(crate) address: usize,
    http: reqwest::Client,
}

impl ClientConnection {
    /// The connection of `client` from address `address` (`source`), with the request timeout.
    pub(crate) fn new(
        client: u32,
        address: usize,
        source: IpAddr,
        timeout: Duration,
    ) -> Result<Self> {
        Ok(Self {
            client,
            address,
            http: http_client(source, timeout)?,
        })
    }

    /// Pass the address's ceilings, send, read the body to its end, and classify the outcome.
    pub(crate) async fn exchange(
        &self,
        context: &RunContext,
        scheduled: Duration,
        request: ResolvedRequest,
        headers: RequestHeaders<'_>,
        body_use: BodyUse,
    ) -> Exchange {
        let guard_delayed = self
            .wait_for_the_guard(context, scheduled, request.auth)
            .await;
        let sent = Instant::now();
        let url = format!("{}{}", context.origin, request.path);
        let mut builder = self.http.request(method(request.method), url);
        if let Some(token) = headers.bearer {
            builder = builder.bearer_auth(token.expose());
        }
        if let Some(tag) = headers.if_none_match {
            builder = builder.header(IF_NONE_MATCH, tag);
        }
        if let Some(body) = request.body {
            builder = builder.header(CONTENT_TYPE, "application/json").body(body);
        }
        let answer = async {
            let response = builder.send().await?;
            let status = response.status().as_u16();
            let entity_tag = response
                .headers()
                .get(ETAG)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let body = response.bytes().await?;
            Ok::<_, reqwest::Error>((status, entity_tag, body))
        }
        .await;
        let finished = Instant::now();
        let (outcome, body, entity_tag) = match answer {
            Ok((status, entity_tag, body)) => (
                RequestOutcome::classify(status, &request.expected_statuses),
                (body_use == BodyUse::Keep).then(|| body.to_vec()),
                entity_tag,
            ),
            Err(error) if error.is_timeout() => (RequestOutcome::Timeout, None, None),
            Err(_) => (RequestOutcome::TransportError, None, None),
        };
        Exchange {
            outcome,
            sent: sent.saturating_duration_since(context.start),
            finished: finished.saturating_duration_since(context.start),
            body,
            entity_tag,
            guard_delayed,
        }
    }

    /// Sleep until the address's guard lets the request leave; `true` when a ceiling held it
    /// back past its due instant.
    async fn wait_for_the_guard(
        &self,
        context: &RunContext,
        scheduled: Duration,
        auth: bool,
    ) -> bool {
        let due = Instant::now().max(context.start + scheduled);
        if !auth {
            let send_at = context.pool.reserve_send(self.address, due);
            tokio::time::sleep_until(send_at).await;
            return send_at > due;
        }
        let mut requested = due;
        loop {
            match context.pool.reserve_auth_send(self.address, requested) {
                Ok(send_at) => {
                    tokio::time::sleep_until(send_at).await;
                    return send_at > due;
                }
                Err(allowed) => {
                    tokio::time::sleep_until(allowed).await;
                    requested = Instant::now().max(allowed);
                }
            }
        }
    }
}

/// One client's HTTP client: bound to its source address, one idle connection, no proxy, no
/// redirects, and the total request timeout.
fn http_client(source: IpAddr, timeout: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .local_address(source)
        .pool_max_idle_per_host(1)
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .tcp_nodelay(true)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| Error::HttpClientNotBuilt {
            address: source,
            error,
        })
}

fn method(method: HttpMethod) -> reqwest::Method {
    match method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Delete => reqwest::Method::DELETE,
    }
}
