//! The dedicated server's JSON configuration (the file its `-config` parameter names) and the
//! one change the agent makes to it: pointing `game.scenarioId` at another scenario header.
//!
//! The change is surgical. The byte range of the `game.scenarioId` string is located
//! (`scenario_id_location`) and only that range is replaced, so every other key and value, the
//! spelling of every number, the order of the keys and the whitespace stay byte for byte. The
//! result is parsed again and must equal the original document with `game.scenarioId` alone
//! changed. The new text then replaces the file atomically (`atomic_replacement`): the path names
//! either the complete old file or the complete new one, never a partial write.

mod atomic_replacement;
mod scenario_id_location;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;
use thiserror::Error;

use atomic_replacement::replace_atomically;
use scenario_id_location::scenario_id_span;

/// A Reforger server config is a few kilobytes; anything far larger is not one.
const SERVER_CONFIG_MAX_BYTES: u64 = 1 << 20;

/// Why the scenario could not be switched. The file is unchanged in every case.
#[derive(Debug, Error)]
pub enum ServerConfigError {
    #[error("cannot be read: {0}")]
    Unreadable(io::Error),
    #[error("is larger than {limit} bytes")]
    TooLarge { limit: u64 },
    #[error("is not valid JSON: {0}")]
    NotJson(String),
    #[error("has no single game.scenarioId string to replace: {0}")]
    NoScenarioId(&'static str),
    #[error("cannot be rewritten without changing more than game.scenarioId")]
    RewriteNotSurgical,
    #[error("cannot be replaced: {0}")]
    NotReplaced(io::Error),
}

/// The dedicated server's config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DedicatedServerConfig {
    path: PathBuf,
}

impl DedicatedServerConfig {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Points `game.scenarioId` at `scenario_id` and changes nothing else in the file.
    pub fn switch_scenario(&self, scenario_id: &str) -> Result<(), ServerConfigError> {
        let original = read_bounded(&self.path)?;
        let rewritten = with_scenario_id(&original, scenario_id)?;
        replace_atomically(&self.path, rewritten.as_bytes()).map_err(ServerConfigError::NotReplaced)
    }
}

fn read_bounded(path: &Path) -> Result<String, ServerConfigError> {
    let length = fs::metadata(path)
        .map_err(ServerConfigError::Unreadable)?
        .len();
    if length > SERVER_CONFIG_MAX_BYTES {
        return Err(ServerConfigError::TooLarge {
            limit: SERVER_CONFIG_MAX_BYTES,
        });
    }
    let bytes = fs::read(path).map_err(ServerConfigError::Unreadable)?;
    String::from_utf8(bytes)
        .map_err(|_| ServerConfigError::NotJson("it is not UTF-8 text".to_owned()))
}

/// `document` with the value of `game.scenarioId` replaced by `scenario_id`, every other byte
/// kept.
fn with_scenario_id(document: &str, scenario_id: &str) -> Result<String, ServerConfigError> {
    let original: Value = serde_json::from_str(document)
        .map_err(|error| ServerConfigError::NotJson(error.to_string()))?;
    let span = scenario_id_span(document)
        .map_err(|absence| ServerConfigError::NoScenarioId(absence.describe()))?;
    let replacement = Value::from(scenario_id).to_string();
    let rewritten = format!(
        "{}{replacement}{}",
        &document[..span.start],
        &document[span.end..]
    );
    let mut expected = original;
    let game = expected
        .get_mut("game")
        .and_then(Value::as_object_mut)
        .ok_or(ServerConfigError::RewriteNotSurgical)?;
    game.insert("scenarioId".to_owned(), Value::from(scenario_id));
    let reparsed: Value =
        serde_json::from_str(&rewritten).map_err(|_| ServerConfigError::RewriteNotSurgical)?;
    if reparsed == expected {
        Ok(rewritten)
    } else {
        Err(ServerConfigError::RewriteNotSurgical)
    }
}

#[cfg(test)]
#[path = "tests/dedicated_server_config.rs"]
mod tests;
