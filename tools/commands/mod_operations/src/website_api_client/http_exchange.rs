//! One HTTP exchange with the website API through `curl`.
//!
//! **Role:** a method, a URL, an optional bearer and an optional JSON body go in; the status, the
//! response headers and the body bytes come out ([`ApiAnswer`]); [`ApiTransport`] is the seam the
//! tests replace and [`CurlTransport`] the real one; `encode_component` percent-encodes a path or
//! query component.
//! **Position:** under [`crate::website_api_client`]; [`crate::website_api_client::ApiClient`] and
//! `mod test-game-runtime-api` exchange through it; `curl` runs through `process_runner`.
//! **Signals & state:** [`CurlTransport`] writes each answer's headers and body into its scratch
//! folder.
//! **Invariants:** no answer at all (the API is down, a timeout) is an error; every HTTP status is
//! an answer, so callers decide what each status means; redirects are not followed.

use std::path::PathBuf;

use crate::error::{Result, ResultExt, bail};
use process_runner::Run;
use serde_json::Value;
use verification_core::NotRun;

/// A response of the website API.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ApiAnswer {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl ApiAnswer {
    /// The body as JSON.
    pub(crate) fn json(&self) -> Result<Value> {
        serde_json::from_slice(&self.body).context("the answer body is not JSON")
    }

    /// A response header, by case-insensitive name.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// `details.code` of a refusal body, when it names one.
    pub(crate) fn refusal_code(&self) -> Option<String> {
        self.json()
            .ok()?
            .pointer("/details/code")?
            .as_str()
            .map(str::to_string)
    }

    /// The start of the body, for an error message.
    pub(crate) fn excerpt(&self) -> String {
        String::from_utf8_lossy(&self.body)
            .chars()
            .take(600)
            .collect()
    }
}

/// Carries one request to the website API.
pub(crate) trait ApiTransport {
    fn exchange(
        &self,
        method: &str,
        url: &str,
        bearer: Option<&str>,
        body: Option<&Value>,
    ) -> Result<ApiAnswer>;
}

/// The `curl` transport. It keeps the last response's headers and body in its scratch
/// directory, so one transport serves one caller at a time.
pub(crate) struct CurlTransport {
    scratch: PathBuf,
    timeout_seconds: u32,
}

impl CurlTransport {
    pub(crate) fn new(scratch: PathBuf, timeout_seconds: u32) -> Self {
        Self {
            scratch,
            timeout_seconds,
        }
    }
}

impl ApiTransport for CurlTransport {
    fn exchange(
        &self,
        method: &str,
        url: &str,
        bearer: Option<&str>,
        body: Option<&Value>,
    ) -> Result<ApiAnswer> {
        std::fs::create_dir_all(&self.scratch)
            .with_context(|| format!("create {}", self.scratch.display()))?;
        let headers_path = self.scratch.join("answer.headers");
        let body_path = self.scratch.join("answer.body");
        let _ = std::fs::remove_file(&headers_path);
        let _ = std::fs::remove_file(&body_path);
        let mut command = Run::new("curl")
            .args(["-sS", "-m", &self.timeout_seconds.to_string(), "-X", method])
            .arg("-D")
            .arg(&headers_path)
            .arg("-o")
            .arg(&body_path)
            .args(["-w", "%{http_code}"]);
        if let Some(bearer) = bearer {
            command = command.args(["-H", &format!("Authorization: Bearer {bearer}")]);
        }
        if let Some(body) = body {
            command = command
                .args([
                    "-H",
                    "Content-Type: application/json",
                    "--data-binary",
                    "@-",
                ])
                .stdin(serde_json::to_string(body)?);
        }
        let output = match command.arg(url).output() {
            Ok(output) => output,
            Err(why @ (NotRun::Signalled { .. } | NotRun::Timeout { .. })) => {
                return Err(why).context("curl did not finish");
            }
            Err(why) => return Err(why).context("curl could not start"),
        };
        if output.code != 0 {
            bail!(
                "{method} {url}: no answer (curl exit {}: {})",
                output.code,
                output.stderr.trim()
            );
        }
        let status: u16 = output
            .stdout
            .trim()
            .parse()
            .with_context(|| format!("{method} {url}: curl reported no HTTP status"))?;
        let headers = parse_headers(&std::fs::read_to_string(&headers_path).unwrap_or_default());
        let body = std::fs::read(&body_path).unwrap_or_default();
        Ok(ApiAnswer {
            status,
            headers,
            body,
        })
    }
}

/// The header lines of a `curl -D` dump. The status line has no colon and is skipped; a value
/// keeps any colons after the first.
pub(super) fn parse_headers(dump: &str) -> Vec<(String, String)> {
    dump.lines()
        .filter_map(|line| line.trim_end_matches('\r').split_once(':'))
        .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
        .collect()
}

/// Percent-encode one path segment or query value: every byte but the RFC 3986 unreserved
/// characters becomes `%XX`.
pub(crate) fn encode_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(char::from(byte))
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}
