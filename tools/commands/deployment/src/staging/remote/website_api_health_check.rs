//! The check that the website API answers on the staging host before the game server deploy
//! changes anything there.
//!
//! **Role:** asks the host for `<TBD_BACKEND_URL>/healthz` and stops the deploy, naming
//! `cargo xtask deploy website`, when the API does not answer.
//!
//! **Position:** the first remote step of [`super::deploy`], before the rsync; the probe's text
//! is [`super::super::payloads::website_api_health_payload`], and it runs through
//! [`super::Runner`], so a dry run prints it instead.
//!
//! **Signals & state:** none; one ssh round trip.
//!
//! **Invariants:** the game server deploy starts nothing of the website stack: the API, its
//! Postgres and the Caddy web server come from `cargo xtask deploy website`, so a host without
//! them stops here with exit 1 rather than later in the game-runtime smoke or at the mod's first
//! call; ssh's own failure (255) keeps its code, because ssh has already said why.

use super::*;
use crate::staging::payloads::website_api_health_payload;

/// The exit status ssh reserves for its own failure: the remote command never ran.
const SSH_FAILURE: i32 = 255;

/// The API's health route under `backend_url`, the origin the mod calls: `TBD_BACKEND_URL`'s one
/// reading ([`crate::staging::fleet_instances::backend_url`]), which already
/// comes without a trailing `/`.
pub(super) fn website_api_health_url(backend_url: &str) -> String {
    format!("{backend_url}/healthz")
}

/// What the deploy prints when the probe on the host exits `code`.
pub(super) fn website_api_refusal(health_url: &str, code: i32) -> String {
    format!(
        "ERROR: {health_url} does not answer on the staging host (probe exit {code}).\n\
         \x20      The website API, its Postgres and Caddy come from `cargo xtask deploy website`:\n\
         \x20      run it, then deploy staging again."
    )
}

/// Runs the probe on the host. `Err(1)` when the API does not answer; ssh's own failure and a
/// transport that never ran keep their codes.
pub(super) fn require_website_api(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    env: &Env,
) -> Result<(), u8> {
    let health_url = website_api_health_url(&env.backend_url);
    let probe = website_api_health_payload(&health_url);
    match runner.ssh(
        base,
        host,
        &["bash".to_string(), "-s".to_string()],
        Some(probe),
    )? {
        0 => Ok(()),
        SSH_FAILURE => Err(SSH_FAILURE as u8),
        code => {
            eprintln!("{}", website_api_refusal(&health_url, code));
            Err(1)
        }
    }
}
