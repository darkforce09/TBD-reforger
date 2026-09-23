//! HTTP calls to the executor routes of the command ledger.
//!
//! Every request carries the machine credential as a bearer token in a header marked
//! sensitive, and redirects are never followed, so the credential reaches only the configured
//! API origin. HTTPS is enforced whenever the configured origin is HTTPS.

use std::error::Error as _;
use std::sync::Once;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest::{Client, StatusCode, Url};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use super::ledger_messages::{ClaimedFleetCommand, ErrorEnvelope, ExecutionResult, ExecutionStart};
use crate::secret_text::SecretText;

const CLAIM_PATH: &str = "api/v1/fleet-executor/commands/claim";
const COMMANDS_PATH: &str = "api/v1/fleet-executor/commands/";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// `details.code` of a 409 whose claim is no longer the caller's.
const STALE_FENCING_TOKEN: &str = "STALE_FENCING_TOKEN";
/// Characters of the API's error message kept in an error.
const API_MESSAGE_MAX_CHARS: usize = 200;

/// The answer to a claim.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaimOutcome {
    Claimed(ClaimedFleetCommand),
    /// 204: nothing is claimable for this server now.
    NothingClaimable,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LedgerError {
    #[error("the API could not be reached: {0}")]
    Unreachable(String),
    #[error("the API answered {status}: {message}")]
    Unavailable { status: u16, message: String },
    #[error("the claim is no longer this agent's (409 STALE_FENCING_TOKEN)")]
    StaleFencingToken,
    #[error("the API answered 409 {code}: {message}")]
    Conflict { code: String, message: String },
    #[error("the API refused the request with {status}: {message}")]
    Refused { status: u16, message: String },
    #[error("the API's answer could not be read: {0}")]
    UnreadableAnswer(String),
}

impl LedgerError {
    /// The request may not have reached the API, or the API failed in a way a later attempt can
    /// pass: worth retrying. Every other error stays the same however often it is repeated.
    pub fn is_transient(&self) -> bool {
        matches!(self, Self::Unreachable(_) | Self::Unavailable { .. })
    }
}

#[derive(Debug, Error)]
#[error("the API client could not be set up: {0}")]
pub struct LedgerApiSetupError(String);

/// The executor routes of one API origin, authenticated as this host's agent.
#[derive(Debug, Clone)]
pub struct LedgerApi {
    http: Client,
    claim_url: Url,
    commands_url: Url,
}

impl LedgerApi {
    /// `api_base_url` must end in `/`; the agent configuration guarantees it.
    pub fn new(api_base_url: &Url, credential: &SecretText) -> Result<Self, LedgerApiSetupError> {
        install_tls_provider();
        let mut authorization = HeaderValue::from_str(&format!("Bearer {}", credential.expose()))
            .map_err(|_| {
            LedgerApiSetupError("the machine credential is not a valid header value".to_owned())
        })?;
        authorization.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization);
        let http = Client::builder()
            .default_headers(headers)
            .user_agent(concat!("fleet-host-agent/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::none())
            .https_only(api_base_url.scheme() == "https")
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|error| LedgerApiSetupError(error.to_string()))?;
        let route = |path: &str| {
            api_base_url
                .join(path)
                .map_err(|error| LedgerApiSetupError(format!("{path}: {error}")))
        };
        Ok(Self {
            http,
            claim_url: route(CLAIM_PATH)?,
            commands_url: route(COMMANDS_PATH)?,
        })
    }

    /// `POST /api/v1/fleet-executor/commands/claim` with the host agent's empty claim request.
    pub async fn claim(&self) -> Result<ClaimOutcome, LedgerError> {
        let response = self
            .http
            .post(self.claim_url.clone())
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(unreachable)?;
        let status = response.status();
        let body = response.bytes().await.map_err(unreachable)?;
        match status {
            StatusCode::NO_CONTENT => Ok(ClaimOutcome::NothingClaimable),
            status if status.is_success() => serde_json::from_slice(&body)
                .map(ClaimOutcome::Claimed)
                .map_err(|error| LedgerError::UnreadableAnswer(error.to_string())),
            status => Err(classify_refusal(status, &body)),
        }
    }

    /// `POST .../commands/{commandId}/executing`: the effect is about to start.
    pub async fn report_executing(
        &self,
        command_id: Uuid,
        fencing_token: i64,
    ) -> Result<(), LedgerError> {
        self.report(command_id, "executing", &ExecutionStart { fencing_token })
            .await
    }

    /// `POST .../commands/{commandId}/result`: what the effect did.
    pub async fn report_result(
        &self,
        command_id: Uuid,
        result: &ExecutionResult,
    ) -> Result<(), LedgerError> {
        self.report(command_id, "result", result).await
    }

    async fn report(
        &self,
        command_id: Uuid,
        step: &str,
        body: &impl Serialize,
    ) -> Result<(), LedgerError> {
        let url = self
            .commands_url
            .join(&format!("{command_id}/{step}"))
            .map_err(|error| LedgerError::UnreadableAnswer(error.to_string()))?;
        let response = self
            .http
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(unreachable)?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = response.bytes().await.unwrap_or_default();
        Err(classify_refusal(status, &body))
    }
}

/// Classifies an answer that is not a success. A 409 carrying `STALE_FENCING_TOKEN` means the
/// claim was taken away; request timeouts, rate limits and server errors are transient.
pub(super) fn classify_refusal(status: StatusCode, body: &[u8]) -> LedgerError {
    let envelope: ErrorEnvelope = serde_json::from_slice(body).unwrap_or_default();
    let message = envelope
        .error
        .as_deref()
        .or(status.canonical_reason())
        .unwrap_or("no message")
        .chars()
        .take(API_MESSAGE_MAX_CHARS)
        .collect::<String>();
    let code = envelope
        .details
        .as_ref()
        .and_then(|details| details.get("code"))
        .and_then(Value::as_str);
    match status {
        StatusCode::CONFLICT if code == Some(STALE_FENCING_TOKEN) => LedgerError::StaleFencingToken,
        StatusCode::CONFLICT => LedgerError::Conflict {
            code: code.unwrap_or("without a code").to_owned(),
            message,
        },
        StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS => LedgerError::Unavailable {
            status: status.as_u16(),
            message,
        },
        status if status.is_server_error() => LedgerError::Unavailable {
            status: status.as_u16(),
            message,
        },
        status => LedgerError::Refused {
            status: status.as_u16(),
            message,
        },
    }
}

/// A transport failure with its causes, such as "error sending request: connection refused".
fn unreachable(error: reqwest::Error) -> LedgerError {
    let mut description = error.to_string();
    let mut cause = error.source();
    while let Some(inner) = cause {
        description.push_str(": ");
        description.push_str(&inner.to_string());
        cause = inner.source();
    }
    LedgerError::Unreachable(description)
}

/// reqwest is built without a bundled crypto provider; ring serves every HTTPS connection.
fn install_tls_provider() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        // An error means a provider is already installed, which serves as well.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[cfg(test)]
#[path = "tests/ledger_api.rs"]
mod tests;
