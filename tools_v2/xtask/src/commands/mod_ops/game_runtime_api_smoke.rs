//! `cargo xtask mod test-game-runtime-api`: the game-runtime routes a server's mod uses, called the
//! way the mod calls them — with the server's `mod_runtime` machine credential.
//!
//! * The deployment read answers the deployment the server runs (200) or `NO_DEPLOYMENT` (404).
//! * With a deployment, the artifact read answers the exact bytes, which hash to the deployment's
//!   `artifact_sha256` and to their entity tag; a deployment for an event also answers its roster
//!   in wire version 2.
//! * Without the credential, the deployment read answers 401.
//!
//! Settings: `TBD_API_BASE` (default `http://127.0.0.1:8080`) and `TBD_MACHINE_CREDENTIAL`
//! (required). Exit: 0 every check passed · 1 a check failed · 2 usage · 3 the API gave no answer.

use anyhow::Result;
use serde_json::Value;

use super::website_api_client::{
    ApiAnswer, ApiTransport, CurlTransport, encode_component, sha256_hex,
};

/// Entry for `xtask mod test-game-runtime-api`.
pub fn run() -> Result<u8> {
    let api = std::env::var("TBD_API_BASE").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let credential = std::env::var("TBD_MACHINE_CREDENTIAL").unwrap_or_default();
    if credential.is_empty() {
        eprintln!(
            "TBD_MACHINE_CREDENTIAL is required: a mod_runtime credential of the server under test."
        );
        return Ok(2);
    }
    let scratch =
        std::env::temp_dir().join(format!("tbd-game-runtime-smoke-{}", std::process::id()));
    let transport = CurlTransport::new(scratch.clone(), 30);
    let outcome = smoke(&transport, &api, &credential);
    let _ = std::fs::remove_dir_all(&scratch);
    match outcome {
        Ok(0) => {
            println!("game-runtime API: every check passed");
            Ok(0)
        }
        Ok(failures) => {
            println!("game-runtime API: {failures} check(s) failed");
            Ok(1)
        }
        Err(error) => {
            eprintln!("ENVIRONMENT: {error:#}");
            Ok(3)
        }
    }
}

/// Run every check; the number that failed. An error is an API that gave no answer at all.
pub(super) fn smoke(transport: &dyn ApiTransport, api: &str, credential: &str) -> Result<usize> {
    let base = format!("{}/api/v1/game-runtime", api.trim_end_matches('/'));
    let mut failures = 0usize;
    let mut check = |passed: bool, line: String| {
        println!("  {}  {line}", if passed { "ok  " } else { "FAIL" });
        if !passed {
            failures += 1;
        }
    };

    let deployment =
        transport.exchange("GET", &format!("{base}/deployment"), Some(credential), None)?;
    match deployment.status {
        404 => check(
            deployment.refusal_code().as_deref() == Some("NO_DEPLOYMENT"),
            "deployment: none (404 NO_DEPLOYMENT) — deploy an approved mission to test the artifact read".into(),
        ),
        200 => {
            let current = deployment.json().unwrap_or(Value::Null);
            check(true, format!("deployment: {}", current["deployment_id"].as_str().unwrap_or("?")));
            let artifact = current["artifact_id"].as_str().unwrap_or_default();
            let expected = current["artifact_sha256"].as_str().unwrap_or_default();
            let answer = transport.exchange(
                "GET",
                &format!("{base}/artifacts/{}", encode_component(artifact)),
                Some(credential),
                None,
            )?;
            check(artifact_is_intact(&answer, expected), format!(
                "artifact {artifact}: HTTP {}, {} bytes",
                answer.status,
                answer.body.len()
            ));
            if let Some(event) = current["event_id"].as_str().filter(|event| !event.is_empty()) {
                let roster = transport.exchange(
                    "GET",
                    &format!("{base}/events/{}/roster", encode_component(event)),
                    Some(credential),
                    None,
                )?;
                let version = roster.json().ok().and_then(|body| body["version"].as_i64());
                check(
                    roster.status == 200 && version == Some(2),
                    format!("roster of event {event}: HTTP {}, version {version:?}", roster.status),
                );
            }
        }
        status => check(false, format!("deployment: HTTP {status}: {}", deployment.excerpt())),
    }

    let anonymous = transport.exchange("GET", &format!("{base}/deployment"), None, None)?;
    check(
        anonymous.status == 401,
        format!(
            "deployment without a credential: HTTP {} (401 expected)",
            anonymous.status
        ),
    );
    Ok(failures)
}

/// The artifact answered 200 with bytes hashing to the deployment's SHA-256 and to their tag.
fn artifact_is_intact(answer: &ApiAnswer, expected: &str) -> bool {
    let actual = sha256_hex(&answer.body);
    let tag = answer.header("etag").map(|tag| tag.trim_matches('"'));
    answer.status == 200 && !expected.is_empty() && actual == expected && tag == Some(expected)
}

#[cfg(test)]
#[path = "tests/game_runtime_api_smoke/tests.rs"]
mod tests;
