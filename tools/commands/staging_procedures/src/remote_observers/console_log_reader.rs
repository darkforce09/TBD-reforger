//! A game server instance's newest `console.log`.
//!
//! **Role:** builds the read of an instance's newest log folder's `console.log` (its path first,
//! then at most the last [`CONSOLE_LOG_TAIL_BYTES`]), and splits that answer back apart.
//!
//! **Position:** used by the fleet effects (mission loaded, session started, kick line, link
//! command) and by the build identity (the Workshop version); the instance's `-profile` folder is
//! the staging deploy's [`InstanceFolder::profile`].
//!
//! **Signals & state:** none; pure builder and parser.
//!
//! **Invariants:** only reads; an instance without a log folder exits 3 with nothing on stdout, so
//! "no log" never reads as an empty log.

use super::remote_command::{RemoteCommand, shell_quote};
use deployment::staging::fleet_instances::InstanceFolder;

/// The tail of `console.log` one read returns.
pub(crate) const CONSOLE_LOG_TAIL_BYTES: u64 = 4 * 1024 * 1024;

/// The first line of every answer: `log: <path>`.
const PATH_PREFIX: &str = "log: ";

/// A pulled `console.log`: where it was and its (tail) text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConsoleLog {
    pub path: String,
    pub text: String,
}

/// The read of instance `instance`'s newest `console.log` under `fleet_root`: the engine writes one
/// `logs_<time>/` folder per boot into the `logs/` of the instance's `-profile` folder.
pub(crate) fn newest(fleet_root: &str, instance: u16) -> RemoteCommand {
    let profile = InstanceFolder::under(fleet_root, instance).profile();
    let logs = shell_quote(&format!("{profile}/logs"));
    RemoteCommand::read_script(
        "console log",
        format!(
            "set -uo pipefail\n\
             newest=\"$(ls -1d {logs}/logs_* 2>/dev/null | tail -n 1)\"\n\
             if [ -z \"$newest\" ] || [ ! -r \"$newest/console.log\" ]; then\n\
             \x20 echo \"instance {instance} has no console.log\" >&2\n\
             \x20 exit 3\n\
             fi\n\
             echo \"{PATH_PREFIX}$newest/console.log\"\n\
             tail -c {CONSOLE_LOG_TAIL_BYTES} \"$newest/console.log\"\n"
        ),
    )
}

/// The path line and the text after it, or `None` when the answer has no path line.
pub(crate) fn parse(output: &str) -> Option<ConsoleLog> {
    let (first, rest) = output.split_once('\n').unwrap_or((output, ""));
    let path = first.strip_prefix(PATH_PREFIX)?;
    Some(ConsoleLog {
        path: path.to_string(),
        text: rest.to_string(),
    })
}
