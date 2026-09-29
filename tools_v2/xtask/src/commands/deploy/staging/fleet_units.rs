//! The three systemd template units of the fleet and the payloads that install and restart them.
//!
//! **Role:** embeds `tbd-reforger@.service`, `fleet-host-agent@.service` and
//! `acknowledgement-dropping-relay@.service` from `tools_v2/xtask/deploy/systemd/`, renders the
//! game server template's two folder placeholders ([`game_server_unit`]), and builds the payloads
//! that write the three units, disable the units the fleet does not run, and restart the game
//! servers.
//!
//! **Position:** called by the deploy pipeline in `super::remote`; the host agent and relay
//! payloads (`super::host_agent`, `super::acknowledgement_relay`) enable their own instances.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** the units go over verbatim inside quoted heredocs, so the remote shell expands
//! nothing in them; the game server template is written with both placeholders replaced; every
//! instance above the fleet size, and every relay instance other than the relay instance, is
//! disabled and stopped, so a smaller fleet leaves nothing running.

use super::config::Env;
use super::fleet_instances::{FleetInstance, MAXIMUM_FLEET_INSTANCES};

/// The game server template, with `/TBD_SERVER_DIR_PLACEHOLDER` and
/// `/TBD_ADDONS_STAGING_PLACEHOLDER`.
pub const GAME_SERVER_TEMPLATE: &str =
    include_str!("../../../../deploy/systemd/tbd-reforger@.service");
/// The host agent template, installed byte for byte.
pub const HOST_AGENT_TEMPLATE: &str =
    include_str!("../../../../deploy/systemd/fleet-host-agent@.service");
/// The relay template, installed byte for byte.
pub const RELAY_TEMPLATE: &str =
    include_str!("../../../../deploy/systemd/acknowledgement-dropping-relay@.service");

/// The game server template for this host: each placeholder keeps the template's leading slash and
/// takes the folder without its own.
pub fn game_server_unit(env: &Env) -> String {
    GAME_SERVER_TEMPLATE
        .replace(
            "TBD_SERVER_DIR_PLACEHOLDER",
            env.server_dir.trim_start_matches('/'),
        )
        .replace(
            "TBD_ADDONS_STAGING_PLACEHOLDER",
            env.addons_staging.trim_start_matches('/'),
        )
}

const UNITS_INSTALL: &str = r#"set -euo pipefail
UNITS="$HOME/.config/systemd/user"
mkdir -p "$UNITS"
cat > "$UNITS/tbd-reforger@.service" <<'UNITEOF'
@GAME_SERVER@UNITEOF
cat > "$UNITS/fleet-host-agent@.service" <<'UNITEOF'
@HOST_AGENT@UNITEOF
cat > "$UNITS/acknowledgement-dropping-relay@.service" <<'UNITEOF'
@RELAY@UNITEOF
loginctl enable-linger "$(id -un)" 2>/dev/null || true
systemctl --user daemon-reload
for n in @SURPLUS@; do
  systemctl --user disable --now "tbd-reforger@$n.service" "fleet-host-agent@$n.service" 2>/dev/null || true
done
for n in @NOT_RELAYED@; do
  systemctl --user disable --now "acknowledgement-dropping-relay@$n.service" 2>/dev/null || true
done
systemctl --user enable @GAME_SERVER_UNITS@
echo "  installed tbd-reforger@, fleet-host-agent@ and acknowledgement-dropping-relay@; enabled @GAME_SERVER_UNITS@"
"#;

/// Numbers from `range` as a shell word list.
fn words(range: impl Iterator<Item = u16>) -> String {
    range.map(|n| n.to_string()).collect::<Vec<_>>().join(" ")
}

/// The unit names of `instances`, `unit` picking one per instance, as a shell word list.
pub fn unit_list(instances: &[FleetInstance], unit: fn(&FleetInstance) -> String) -> String {
    instances.iter().map(unit).collect::<Vec<_>>().join(" ")
}

/// Write the three template units, reload the user manager, disable and stop what the fleet does
/// not run, and enable each instance's game server.
pub fn units_install_payload(env: &Env, instances: &[FleetInstance]) -> String {
    let count = instances.len() as u16;
    let relay_instance = instances
        .iter()
        .find(|instance| instance.relay_port.is_some())
        .map(|instance| instance.number);
    UNITS_INSTALL
        .replace("@SURPLUS@", &words((count + 1)..=MAXIMUM_FLEET_INSTANCES))
        .replace(
            "@NOT_RELAYED@",
            &words((1..=MAXIMUM_FLEET_INSTANCES).filter(|n| Some(*n) != relay_instance)),
        )
        .replace(
            "@GAME_SERVER_UNITS@",
            &unit_list(instances, FleetInstance::game_server_unit),
        )
        .replace("@HOST_AGENT@", HOST_AGENT_TEMPLATE)
        .replace("@RELAY@", RELAY_TEMPLATE)
        .replace("@GAME_SERVER@", &game_server_unit(env))
}

/// Restart every instance's game server; `restart` starts a unit that is not running.
pub fn game_servers_restart_payload(instances: &[FleetInstance]) -> String {
    format!(
        "set -euo pipefail\nsystemctl --user restart {}\n",
        unit_list(instances, FleetInstance::game_server_unit)
    )
}

#[cfg(test)]
#[path = "tests/fleet_units/tests.rs"]
mod tests;
