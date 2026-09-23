//! The scenario the staging server runs now. Deployments own `game.scenarioId` once the host
//! agent has switched it for a mission restart, so a redeploy keeps the live value and
//! `TBD_SCENARIO` seeds only a server whose config does not exist yet.

use super::*;

/// The live config's `game.scenarioId`, when the server has a config naming one the engine
/// accepts.
pub(super) fn deployed_scenario(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    env: &Env,
) -> Result<Option<String>, u8> {
    let (_, text) = runner.ssh_capture(
        base,
        host,
        &[format!(
            "cat '{}' 2>/dev/null || true",
            env.server_config_remote
        )],
    )?;
    Ok(scenario_of_config(&text))
}

/// `game.scenarioId` of a server config, when it matches the engine's resource schema.
pub(crate) fn scenario_of_config(text: &str) -> Option<String> {
    let config: serde_json::Value = serde_json::from_str(text).ok()?;
    let scenario = config.pointer("/game/scenarioId")?.as_str()?;
    regex::Regex::new(r"^\{[0-9A-F]{16}\}[a-zA-Z0-9_./ -]+$")
        .ok()?
        .is_match(scenario)
        .then(|| scenario.to_string())
}
