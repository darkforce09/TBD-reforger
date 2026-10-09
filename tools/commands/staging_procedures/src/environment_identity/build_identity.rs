//! What the staging host runs: the API and host agent binaries, the API build, the migration
//! head, the Postgres version, the Experimental server build, the Workshop version, and the guild
//! ids.
//!
//! **Role:** builds the reads of each build identity and extracts the value from each answer.
//!
//! **Position:** read once per recorded run by [`super::collect`]; the readers are the shared
//! observers (database, `/metrics`, `console.log`) and the server install's app manifest.
//!
//! **Signals & state:** none; pure builders and parsers.
//!
//! **Invariants:** only reads; the main guild id is read from the API env file by key, and no
//! other value of that file leaves the host.

use crate::remote_observers::remote_command::{RemoteCommand, shell_quote};
use crate::staging_settings::StagingSettings;

/// The API env key naming the main guild.
const MAIN_GUILD_KEY: &str = "DISCORD_GUILD_ID";

/// The read of the binaries' digests and the main guild id, as `key=value` lines.
pub(crate) fn host_files(settings: &StagingSettings) -> RemoteCommand {
    let api = shell_quote(&format!("{}/target/release/api-server", settings.checkout));
    let env_file = shell_quote(&settings.api_env_file());
    RemoteCommand::read_script(
        "build identity",
        format!(
            "set -uo pipefail\n\
             echo \"api_binary_sha256=$(sha256sum {api} 2>/dev/null | cut -d ' ' -f 1)\"\n\
             echo \"host_agent_binary_sha256=$(sha256sum \"$HOME/.local/bin/game_server_host_agent\" 2>/dev/null | cut -d ' ' -f 1)\"\n\
             echo \"main_guild_id=$(sed -n 's/^{MAIN_GUILD_KEY}=//p' {env_file} 2>/dev/null | tail -n 1 | tr -d '\"\\r')\"\n"
        ),
    )
}

/// The `key=value` lines of an answer with a non-empty value.
pub(crate) fn key_values(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(_, value)| !value.trim().is_empty())
        .map(|(key, value)| (key.to_string(), value.trim().to_string()))
        .collect()
}

/// The version of the first `console.log` line that names the `TBD_Framework` addon and carries
/// a dotted version number.
pub(crate) fn workshop_version(console_log: &str) -> Option<String> {
    console_log
        .lines()
        .filter(|line| line.to_ascii_lowercase().contains("tbd_framework"))
        .find_map(|line| {
            line.split(|c: char| !(c.is_ascii_digit() || c == '.'))
                .find(|token| {
                    let parts: Vec<&str> = token.split('.').collect();
                    parts.len() >= 3 && parts.iter().all(|part| !part.is_empty())
                })
                .map(str::to_string)
        })
}
