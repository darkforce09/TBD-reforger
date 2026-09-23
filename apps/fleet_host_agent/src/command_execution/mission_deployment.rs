//! The arguments of `restart_with_mission`, a cross-terrain mission deployment, validated by
//! the rules the API builds them with: `deployment_id` and `artifact_id` are UUIDs in their
//! hyphenated form, `artifact_sha256` is 64 lowercase hex digits, `scenario_id` is a scenario
//! header resource, and no other key is accepted.

use serde_json::{Map, Value};
use uuid::Uuid;

use super::command_refusal::{CommandRefusal, arguments_with_only};

pub(super) const RESTART_WITH_MISSION: &str = "restart_with_mission";

const ARGUMENT_KEYS: [&str; 4] = [
    "deployment_id",
    "artifact_id",
    "artifact_sha256",
    "scenario_id",
];
const HYPHENATED_UUID_LENGTH: usize = 36;
const SHA256_HEX_DIGITS: usize = 64;
const SCENARIO_GUID_HEX_DIGITS: usize = 16;
const SCENARIO_HEADER_SUFFIX: &str = ".conf";

const UUID_EXPECTATION: &str = "a UUID in its hyphenated form";
const SHA256_EXPECTATION: &str = "64 lowercase hex digits";
const SCENARIO_EXPECTATION: &str = "a scenario header resource: {16 uppercase hex digits} then a path of letters, digits and _./- ending in .conf";

/// The deployment a `restart_with_mission` command carries. The agent writes only the scenario
/// into the server config; the ids and the digest identify the deployment in the log, and the
/// game runtime loads and verifies the artifact itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionDeployment {
    pub deployment_id: Uuid,
    pub artifact_id: Uuid,
    pub artifact_sha256: String,
    pub scenario_id: ScenarioId,
}

/// A scenario header resource matching `^\{[0-9A-F]{16}\}[A-Za-z0-9_./-]+\.conf$`, for example
/// `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioId(String);

impl ScenarioId {
    pub fn parse(raw: &str) -> Option<Self> {
        let (guid, path) = raw.strip_prefix('{')?.split_once('}')?;
        let valid = guid.len() == SCENARIO_GUID_HEX_DIGITS
            && guid
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
            && path.len() > SCENARIO_HEADER_SUFFIX.len()
            && path.ends_with(SCENARIO_HEADER_SUFFIX)
            && path.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'/' | b'-')
            });
        valid.then(|| Self(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
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
