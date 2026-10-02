//! One HTTP exchange with the website API through `curl`: a method, a URL, an optional bearer
//! and an optional JSON body go in; the status, the response headers and the body bytes come
//! out. No answer at all (the API is down, a timeout) is an error; every HTTP status is an
//! answer, so callers decide what each status means. Redirects are not followed.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// A response of the website API.
#[derive(Debug, Clone, PartialEq)]
pub struct ApiAnswer {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl ApiAnswer {
    /// The body as JSON.
    pub fn json(&self) -> Result<Value> {
        serde_json::from_slice(&self.body).context("the answer body is not JSON")
    }

    /// A response header, by case-insensitive name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// `details.code` of a refusal body, when it names one.
    pub fn refusal_code(&self) -> Option<String> {
        self.json()
            .ok()?
            .pointer("/details/code")?
            .as_str()
            .map(str::to_string)
    }

    /// The start of the body, for an error message.
    pub fn excerpt(&self) -> String {
        String::from_utf8_lossy(&self.body)
            .chars()
            .take(600)
            .collect()
    }
}

/// Carries one request to the website API.
pub trait ApiTransport {
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
pub struct CurlTransport {
    scratch: PathBuf,
    timeout_seconds: u32,
}

impl CurlTransport {
    pub fn new(scratch: PathBuf, timeout_seconds: u32) -> Self {
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
        let mut command = Command::new("curl");
        command
            .args(["-sS", "-m", &self.timeout_seconds.to_string(), "-X", method])
            .arg("-D")
            .arg(&headers_path)
            .arg("-o")
            .arg(&body_path)
            .args(["-w", "%{http_code}"]);
        if let Some(bearer) = bearer {
            command.args(["-H", &format!("Authorization: Bearer {bearer}")]);
        }
        if body.is_some() {
            command.args([
                "-H",
                "Content-Type: application/json",
                "--data-binary",
                "@-",
            ]);
        }
        command
            .arg(url)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().context("curl could not start")?;
        let mut input = child.stdin.take().context("curl stdin")?;
        if let Some(body) = body {
            input.write_all(serde_json::to_string(body)?.as_bytes())?;
        }
        drop(input);
        let output = child.wait_with_output().context("curl did not finish")?;
        if !output.status.success() {
            bail!(
                "{method} {url}: no answer (curl exit {}: {})",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let status: u16 = String::from_utf8_lossy(&output.stdout)
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
pub fn encode_component(text: &str) -> String {
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
