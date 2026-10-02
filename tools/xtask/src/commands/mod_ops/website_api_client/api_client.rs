//! An authenticated caller of the website API: the base URL, a bearer token and the transport.
//! A call names the status it expects; any other status is an error that carries the refusal
//! code and the start of the body, so the tool's operator sees what the platform said.

use anyhow::{Result, bail};
use serde_json::Value;

use super::http_exchange::{ApiAnswer, ApiTransport};

/// Calls the website API with one bearer token.
pub struct ApiClient<'transport> {
    transport: &'transport dyn ApiTransport,
    base: String,
    bearer: String,
}

impl<'transport> ApiClient<'transport> {
    /// `base` is the API origin, such as `http://127.0.0.1:8080`; paths start at `/api/v1`.
    pub fn new(transport: &'transport dyn ApiTransport, base: &str, bearer: &str) -> Self {
        Self {
            transport,
            base: base.trim_end_matches('/').to_string(),
            bearer: bearer.to_string(),
        }
    }

    /// One request, whatever its status.
    pub fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Result<ApiAnswer> {
        self.transport.exchange(
            method,
            &format!("{}{path}", self.base),
            Some(&self.bearer),
            body,
        )
    }

    /// One request that must answer `expected`; its body is returned.
    pub fn expect(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        expected: u16,
    ) -> Result<ApiAnswer> {
        let answer = self.call(method, path, body)?;
        if answer.status != expected {
            bail!("{}", refusal_message(method, path, expected, &answer));
        }
        Ok(answer)
    }

    /// One request that must answer `expected` with a JSON body.
    pub fn expect_json(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        expected: u16,
    ) -> Result<Value> {
        self.expect(method, path, body, expected)?.json()
    }
}

/// `METHOD path answered 409 DEPLOYMENT_IN_PROGRESS (expected 202): <body>`.
pub(super) fn refusal_message(
    method: &str,
    path: &str,
    expected: u16,
    answer: &ApiAnswer,
) -> String {
    let code = answer
        .refusal_code()
        .map(|code| format!(" {code}"))
        .unwrap_or_default();
    format!(
        "{method} {path} answered {}{code} (expected {expected}): {}",
        answer.status,
        answer.excerpt()
    )
}

/// A string field of a JSON value, or an error naming where it was expected.
pub(super) fn text_at(value: &Value, pointer: &str, what: &str) -> Result<String> {
    match value.pointer(pointer).and_then(Value::as_str) {
        Some(text) if !text.is_empty() => Ok(text.to_string()),
        _ => bail!("{what}: no `{pointer}` in {value}"),
    }
}
