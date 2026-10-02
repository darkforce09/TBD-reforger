//! The host agents (`apps/fleet_host_agent`) of the fleet: one per instance, each a user service of
//! the account that runs the game servers, because it restarts its instance's unit and rewrites its
//! server config's `scenarioId` for mission restarts; it polls the platform with its instance's
//! `host_agent` machine credential and reads the game over RCON.
//!
//! **Role:** renders each instance's `agent.toml` ([`agent_configuration`]) and the payload that
//! builds and installs the agent binary, writes every configuration, restarts every
//! `fleet_host_agent@N` and reads each unit's state back ([`host_agents_install_payload`]).
//!
//! **Position:** called by the deploy pipeline in `super::remote` after every boot verdict holds and
//! after the relay is up; the unit template itself is written by `super::fleet_units`.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** an `agent.toml` names files, never secrets: the instance's
//! `secrets/host-agent-credential` and `secrets/rcon-password` under `~/tbd/fleet/instance-N/`;
//! it is written mode 600 under a mode-700 folder; the relay instance's agent polls the relay on
//! loopback, every other agent polls `TBD_HOST_AGENT_API_URL`; a unit that is not active after
//! the restart fails the deploy rather than being reported as installed.

use super::fleet_instances::{FleetInstance, HOST_AGENT_CREDENTIAL_FILE, RCON_PASSWORD_FILE};
use super::fleet_units::unit_list;

/// Instance `instance`'s `agent.toml`, for an unquoted heredoc: the remote shell expands `$HOME`,
/// which the agent needs because it accepts absolute paths only.
pub fn agent_configuration(instance: &FleetInstance) -> String {
    let folder = format!("$HOME/{}", instance.home_relative_folder());
    format!(
        "api_base_url = \"{api}\"\n\
         credential_file = \"{folder}/secrets/{HOST_AGENT_CREDENTIAL_FILE}\"\n\
         poll_interval_seconds = 5\n\
         \n\
         [game_server]\n\
         systemd_user_unit = \"{unit}\"\n\
         server_config_path = \"{folder}/server.config.json\"\n\
         \n\
         [rcon]\n\
         address = \"127.0.0.1\"\n\
         port = {port}\n\
         password_file = \"{folder}/secrets/{RCON_PASSWORD_FILE}\"\n",
        api = instance.agent_api_url,
        unit = instance.game_server_unit(),
        port = instance.rcon_port,
    )
}

/// The install: build the agent from the synced checkout, install the binary, write every
/// instance's configuration, restart every agent, then read each unit's state back, because a unit
/// that did not come up must fail the deploy rather than be reported as installed.
pub fn host_agents_install_payload(remote_dir: &str, instances: &[FleetInstance]) -> String {
    let mut payload = format!(
        "set -euo pipefail\n\
         {toolchain}\n\
         umask 077\n\
         mkdir -p \"$HOME/.local/bin\" \"$HOME/.config/fleet_host_agent\"\n\
         chmod 700 \"$HOME/.config/fleet_host_agent\"\n\
         (cd '{remote_dir}' && cargo build --release -q -p fleet_host_agent)\n\
         install -m 755 '{remote_dir}/target/release/fleet_host_agent' \"$HOME/.local/bin/fleet_host_agent\"\n",
        toolchain = crate::commands::deploy::remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH,
    );
    for instance in instances {
        payload.push_str(&format!(
            "AGENT_DIR=\"$HOME/.config/fleet_host_agent/instance-{n}\"\n\
             mkdir -p \"$AGENT_DIR\"\n\
             chmod 700 \"$AGENT_DIR\"\n\
             cat > \"$AGENT_DIR/agent.toml\" <<AGENTTOML\n\
             {configuration}AGENTTOML\n\
             chmod 600 \"$AGENT_DIR/agent.toml\"\n",
            n = instance.number,
            configuration = agent_configuration(instance),
        ));
    }
    let units = unit_list(instances, FleetInstance::host_agent_unit);
    payload.push_str(&format!(
        "systemctl --user enable {units}\n\
         systemctl --user restart {units}\n\
         sleep 3\n\
         failed=0\n\
         for unit in {units}; do\n\
         \x20 state=\"$(systemctl --user show -p ActiveState --value \"$unit\" 2>/dev/null || true)\"\n\
         \x20 if [ \"$state\" = \"active\" ]; then\n\
         \x20   echo \"  $unit active\"\n\
         \x20 else\n\
         \x20   echo \"FAIL: $unit is '$state', not active.\" >&2\n\
         \x20   journalctl --user -u \"$unit\" -n 20 --no-pager >&2 || true\n\
         \x20   failed=1\n\
         \x20 fi\n\
         done\n\
         exit \"$failed\"\n"
    ));
    payload
}

#[cfg(test)]
#[path = "tests/host_agent/tests.rs"]
mod tests;
