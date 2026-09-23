//! Why a claimed command is refused without acting, and the argument check every action shares:
//! the arguments are a JSON object holding no key the action does not accept.

use serde_json::{Map, Value};
use thiserror::Error;

/// Characters of a refused action name or argument key quoted back in a refusal.
const QUOTED_NAME_MAX_CHARS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CommandRefusal {
    #[error("{0} runs in the game runtime, not on the host agent")]
    GameRuntimeAction(String),
    #[error("the host agent does not perform the action {0:?}")]
    UnsupportedAction(String),
    #[error("the arguments of {action} are not a JSON object")]
    ArgumentsNotAnObject { action: &'static str },
    #[error("{action} does not accept the argument {key:?}")]
    UnexpectedArgument { action: &'static str, key: String },
    #[error("{action} needs {key} to be {expected}")]
    InvalidArgument {
        action: &'static str,
        key: &'static str,
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
