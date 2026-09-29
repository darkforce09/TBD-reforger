//! The scenario each fleet instance runs now. Deployments own `game.scenarioId` once the host
//! agent has switched it for a mission restart, so a redeploy keeps the live value and
//! `TBD_SCENARIO` seeds only an instance whose config does not exist yet. The value is extracted on
//! the host, because the config also holds the instance's RCON and join passwords, which never
//! leave it.

use super::*;

/// The remote script that prints the `game.scenarioId` of `instance`'s config, or nothing when the
/// config is absent. Only the scenario line crosses the wire.
pub(crate) fn scenario_read_payload(instance: &FleetInstance) -> String {
    format!(
        "CONFIG=\"$HOME/{}/server.config.json\"\n\
         [ -f \"$CONFIG\" ] || exit 0\n\
         sed -n 's/.*\"scenarioId\": *\"\\([^\"]*\\)\".*/\\1/p' \"$CONFIG\" | head -1\n",
        instance.home_relative_folder()
    )
}

/// The live `game.scenarioId` of `instance`, when its config names one the engine accepts.
pub(super) fn deployed_scenario(
    runner: &Runner,
    base: &SshBase,
    host: &str,
    instance: &FleetInstance,
) -> Result<Option<String>, u8> {
    let (_, text) = runner.ssh_capture(
        base,
        host,
        &bash_stdin(),
        Some(scenario_read_payload(instance)),
    )?;
    Ok(scenario_of_config(&text))
}

/// The scenario a host read printed, when it matches the engine's resource schema.
pub(crate) fn scenario_of_config(text: &str) -> Option<String> {
    let scenario = text.trim();
    regex::Regex::new(r"^\{[0-9A-F]{16}\}[a-zA-Z0-9_./ -]+$")
        .ok()?
        .is_match(scenario)
        .then(|| scenario.to_string())
}
