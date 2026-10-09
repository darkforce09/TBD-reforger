//! The arguments of `restart_with_mission`, a cross-terrain mission deployment, validated by
//! the rules the API builds them with: `deployment_id` and `artifact_id` are UUIDs in their
//! hyphenated form, `artifact_sha256` is 64 lowercase hex digits, `scenario_id` is a scenario
//! header resource, and no other key is accepted.

use serde_json::{Map, Value};
use uuid::Uuid;

use super::command_refusal::{CommandRefusal, arguments_with_only};
use crate::identifiers::ScenarioId;

pub(super) const RESTART_WITH_MISSION: &str = "restart_with_mission";

const ARGUMENT_KEYS: [&str; 4] = [
    "deployment_id",
    "artifact_id",
    "artifact_sha256",
    "scenario_id",
];
const HYPHENATED_UUID_LENGTH: usize = 36;
const SHA256_HEX_DIGITS: usize = 64;

const UUID_EXPECTATION: &str = "a UUID in its hyphenated form";
const SHA256_EXPECTATION: &str = "64 lowercase hex digits";
const SCENARIO_EXPECTATION: &str = "a scenario header resource: {16 uppercase hex digits} then a path of letters, digits and _./- ending in .conf";

/// The deployment a `restart_with_mission` command carries. The agent writes only the scenario
/// into the server config; the ids and the digest identify the deployment in the log, and the
/// game runtime loads and verifies the artifact itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionDeployment {
    /// The platform's deployment record.
    pub deployment_id: Uuid,
    /// The mission artifact the game runtime loads.
    pub artifact_id: Uuid,
    /// The artifact's SHA-256 digest, 64 lowercase hex digits.
    pub artifact_sha256: String,
    /// The mission header the server config is switched to.
    pub scenario_id: ScenarioId,
}

impl MissionDeployment {
    pub(super) fn from_arguments(arguments: &Value) -> Result<Self, CommandRefusal> {
        let arguments = arguments_with_only(RESTART_WITH_MISSION, arguments, &ARGUMENT_KEYS)?;
        let artifact_sha256 = text(arguments, "artifact_sha256")
            .filter(|digest| {
                digest.len() == SHA256_HEX_DIGITS
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            .ok_or_else(|| invalid("artifact_sha256", SHA256_EXPECTATION))?;
        let scenario_id = text(arguments, "scenario_id")
            .and_then(ScenarioId::parse)
            .ok_or_else(|| invalid("scenario_id", SCENARIO_EXPECTATION))?;
        Ok(Self {
            deployment_id: uuid(arguments, "deployment_id")?,
            artifact_id: uuid(arguments, "artifact_id")?,
            artifact_sha256: artifact_sha256.to_owned(),
            scenario_id,
        })
    }
}

fn text<'a>(arguments: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    arguments.get(key).and_then(Value::as_str)
}

/// A UUID in the 36-character hyphenated form, in either letter case.
fn uuid(arguments: &Map<String, Value>, key: &'static str) -> Result<Uuid, CommandRefusal> {
    text(arguments, key)
        .filter(|raw| raw.len() == HYPHENATED_UUID_LENGTH)
        .and_then(|raw| Uuid::try_parse(raw).ok())
        .ok_or_else(|| invalid(key, UUID_EXPECTATION))
}

fn invalid(key: &'static str, expected: &'static str) -> CommandRefusal {
    CommandRefusal::InvalidArgument {
        action: RESTART_WITH_MISSION,
        key,
        expected,
    }
}

#[cfg(test)]
#[path = "tests/mission_deployment.rs"]
mod tests;
