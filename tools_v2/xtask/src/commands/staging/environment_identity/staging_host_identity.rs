//! The staging host's identity: name, kernel, CPUs and memory.
//!
//! **Role:** builds the read of the host's identity and turns its `key=value` answer into
//! environment entries.
//!
//! **Position:** read once per recorded run by [`super::collect`].
//!
//! **Signals & state:** none; pure builder and parser.
//!
//! **Invariants:** only reads; every entry key is `staging_<key>`.

use crate::commands::staging::remote_observers::remote_command::RemoteCommand;

/// The read: `host=`, `kernel=`, `cpus=` and `memory_kib=` lines.
pub(crate) fn command() -> RemoteCommand {
    RemoteCommand::read_script(
        "host identity",
        "set -uo pipefail\n\
         echo \"host=$(uname -n)\"\n\
         echo \"kernel=$(uname -r)\"\n\
         echo \"cpus=$(nproc)\"\n\
         echo \"memory_kib=$(awk '/^MemTotal:/ {print $2}' /proc/meminfo)\"\n"
            .to_string(),
    )
}

/// `(staging_<key>, value)` for every non-empty `key=value` line of `output`.
pub(crate) fn entries(output: &str) -> Vec<(String, String)> {
    output
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(_, value)| !value.trim().is_empty())
        .map(|(key, value)| (format!("staging_{key}"), value.trim().to_string()))
        .collect()
}
