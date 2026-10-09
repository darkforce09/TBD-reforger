//! Why a claimed command is refused without acting, and the argument check every action shares:
//! the arguments are a JSON object holding no key the action does not accept.

use serde_json::{Map, Value};
use thiserror::Error;

/// Characters of a refused action name or argument key quoted back in a refusal.
const QUOTED_NAME_MAX_CHARS: usize = 64;

/// Why the agent refuses a claimed command without acting; the message becomes the failure reason
/// of the command's result report.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CommandRefusal {
    /// The action (`broadcast`, `kick`, `load_mission`) is the game runtime's; carries its name.
    #[error("{0} runs in the game runtime, not on the host agent")]
    GameRuntimeAction(String),
    /// The action is unknown to the host agent; carries its name, cut short for quoting.
    #[error("the host agent does not perform the action {0:?}")]
    UnsupportedAction(String),
    /// The arguments are not a JSON object.
    #[error("the arguments of {action} are not a JSON object")]
    ArgumentsNotAnObject {
        /// The action as the ledger names it.
        action: &'static str,
    },
    /// The arguments hold a key the action does not accept.
    #[error("{action} does not accept the argument {key:?}")]
    UnexpectedArgument {
        /// The action as the ledger names it.
        action: &'static str,
        /// The unexpected key, cut short for quoting.
        key: String,
    },
    /// An accepted argument is missing or does not have the expected form.
    #[error("{action} needs {key} to be {expected}")]
    InvalidArgument {
        /// The action as the ledger names it.
        action: &'static str,
        /// The argument at fault.
        key: &'static str,
        /// The form the argument must have.
        expected: &'static str,
    },
}

/// The arguments as an object holding no key outside `allowed`.
pub(super) fn arguments_with_only<'a>(
    action: &'static str,
    arguments: &'a Value,
    allowed: &[&str],
) -> Result<&'a Map<String, Value>, CommandRefusal> {
    let arguments = arguments
        .as_object()
        .ok_or(CommandRefusal::ArgumentsNotAnObject { action })?;
    match arguments
        .keys()
        .find(|key| !allowed.contains(&key.as_str()))
    {
        Some(key) => Err(CommandRefusal::UnexpectedArgument {
            action,
            key: quoted(key),
        }),
        None => Ok(arguments),
    }
}

/// A name from the API, cut short enough to quote in a failure reason.
pub(super) fn quoted(name: &str) -> String {
    name.chars().take(QUOTED_NAME_MAX_CHARS).collect()
}
