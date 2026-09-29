//! The API's `/metrics` exposition, read on the staging host.
//!
//! **Role:** builds the read of `/metrics` with the observability bearer, and picks samples out of
//! the exposition text.
//!
//! **Position:** used by the load procedure's game-operation deltas, the Discord procedure's
//! outcome counter, and the build identity (`tbd_build_info`).
//!
//! **Signals & state:** none; pure builder and parser.
//!
//! **Invariants:** the token is read from the API env file on the host and piped to `curl` as a
//! header on stdin (`-H @-`); it never appears in an argument vector, a script or this process,
//! and the script prints nothing but the exposition.

use super::remote_command::{RemoteCommand, shell_quote};

/// The API env key holding the bearer `/metrics` requires.
pub(crate) const OBSERVABILITY_TOKEN_KEY: &str = "OBSERVABILITY_TOKEN";

/// The read of `<api_origin>/metrics`, authorised from `api_env_file` on the host.
pub(crate) fn exposition(api_env_file: &str, api_origin: &str) -> RemoteCommand {
    let env_file = shell_quote(api_env_file);
    let url = shell_quote(&format!("{api_origin}/metrics"));
    RemoteCommand::read_script(
        "metrics",
        format!(
            "set -euo pipefail\n\
             token=\"$(sed -n 's/^{OBSERVABILITY_TOKEN_KEY}=//p' {env_file} | tail -n 1 | tr -d '\"\\r')\"\n\
             if [ -z \"$token\" ]; then echo '{OBSERVABILITY_TOKEN_KEY} is not set on the host' >&2; exit 3; fi\n\
             printf 'Authorization: Bearer %s\\n' \"$token\" | curl -fsS --max-time 20 -H @- {url}\n"
        ),
    )
}

/// The value of the first sample of `name` whose labels contain every `label=value` of
/// `labels`, e.g. `sample(text, "tbd_build_info", &[])`.
pub(crate) fn sample(exposition: &str, name: &str, labels: &[(&str, &str)]) -> Option<f64> {
    exposition
        .lines()
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| {
            let (series, value) = line.rsplit_once(' ')?;
            let (metric, label_text) = match series.split_once('{') {
                Some((metric, rest)) => (metric, rest.trim_end_matches('}')),
                None => (series, ""),
            };
            let matches = metric == name
                && labels
                    .iter()
                    .all(|(key, value)| label_text.contains(&format!("{key}=\"{value}\"")));
            matches.then(|| value.parse().ok()).flatten()
        })
}

/// The `version` label of `tbd_build_info`, the running API's build.
pub(crate) fn build_version(exposition: &str) -> Option<String> {
    exposition.lines().find_map(|line| {
        let rest = line.strip_prefix("tbd_build_info{")?;
        let start = rest.find("version=\"")? + "version=\"".len();
        let end = rest[start..].find('"')?;
        Some(rest[start..start + end].to_string())
    })
}
