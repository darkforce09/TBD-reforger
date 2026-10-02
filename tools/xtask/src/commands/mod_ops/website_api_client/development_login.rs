//! The development login: `GET /api/v1/auth/dev-login?role=<role>` exists only while the API
//! runs with `APP_ENV=development`, and answers 302 to the frontend callback with the access
//! token in the URL fragment.

use anyhow::{Context, Result, bail};

use super::http_exchange::{ApiTransport, encode_component};

/// An access token for a development account with `role` (`admin`, `mission_maker`, …).
pub fn development_login(
    transport: &dyn ApiTransport,
    api_base: &str,
    role: &str,
) -> Result<String> {
    let url = format!(
        "{api_base}/api/v1/auth/dev-login?role={}",
        encode_component(role)
    );
    let answer = transport.exchange("GET", &url, None, None)?;
    if answer.status != 302 {
        bail!(
            "GET /api/v1/auth/dev-login answered {} (expected 302) — does the API run with \
             APP_ENV=development? {}",
            answer.status,
            answer.excerpt()
        );
    }
    let location = answer
        .header("location")
        .context("dev-login answered 302 without a Location header")?;
    access_token_from_callback(location).context("dev-login's callback URL carries no access_token")
}

/// The `access_token` of a callback URL's fragment.
pub(super) fn access_token_from_callback(location: &str) -> Option<String> {
    let (_, fragment) = location.split_once('#')?;
    fragment
        .split('&')
        .find_map(|pair| pair.strip_prefix("access_token="))
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}
