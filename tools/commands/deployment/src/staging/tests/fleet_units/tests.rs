//! The three template units and their install.
use super::*;
use crate::staging::config::tests::base;

/// The unit's `ExecStart=` line.
fn exec_start(unit: &str) -> &str {
    unit.lines()
        .find_map(|line| line.strip_prefix("ExecStart="))
        .expect("an ExecStart line")
}

/// The game server runs the experimental install with the instance's own config and profile and
/// the shared addon folder; `-addons`, which `-config` refuses, never appears.
#[test]
fn the_game_server_unit_starts_instance_i_from_its_own_folder() {
    let unit = game_server_unit(&base());
    assert_eq!(
        exec_start(&unit),
        "/home/deploy/steam/arma-reforger-server/ArmaReforgerServer -addonsDir /home/deploy/tbd/addons \
         -config %h/tbd/fleet/instance-%i/server.config.json -profile %h/tbd/fleet/instance-%i/profile \
         -maxFPS 60 -logStats 30000 -nothrow"
    );
    assert!(unit.contains("\nWorkingDirectory=/home/deploy/steam/arma-reforger-server\n"));
    assert!(!exec_start(&unit).contains(" -addons "), "{unit}");
    assert!(!unit.contains("PLACEHOLDER"), "{unit}");
    assert!(unit.contains("\nRestart=on-failure\nRestartSec=10\n"));
    // The committed template keeps absolute placeholders, so it verifies as it stands.
    assert!(
        GAME_SERVER_TEMPLATE.contains("ExecStart=/TBD_SERVER_DIR_PLACEHOLDER/ArmaReforgerServer")
    );
    assert!(GAME_SERVER_TEMPLATE.contains(" -addonsDir /TBD_ADDONS_STAGING_PLACEHOLDER "));
}

#[test]
fn the_host_agent_unit_reads_the_instance_configuration() {
    assert_eq!(
        exec_start(HOST_AGENT_TEMPLATE),
        "%h/.local/bin/fleet_host_agent %h/.config/fleet_host_agent/instance-%i/agent.toml"
    );
    assert!(HOST_AGENT_TEMPLATE.contains("\nRestartPreventExitStatus=78\n"));
    assert!(HOST_AGENT_TEMPLATE.contains("\nTimeoutStopSec=200\n"));
}

#[test]
fn the_relay_unit_listens_where_its_environment_file_says_behind_a_private_socket() {
    assert_eq!(
        exec_start(RELAY_TEMPLATE),
        "%h/.local/bin/acknowledgement-dropping-relay serve --listen ${RELAY_LISTEN} \
         --upstream ${RELAY_UPSTREAM} --control-socket %t/acknowledgement-dropping-relay-%i/control.sock"
    );
    assert!(RELAY_TEMPLATE.contains("\nEnvironmentFile=%h/tbd/fleet/instance-%i/relay.env\n"));
    assert!(RELAY_TEMPLATE.contains("\nRuntimeDirectory=acknowledgement-dropping-relay-%i\n"));
    assert!(RELAY_TEMPLATE.contains("\nRuntimeDirectoryMode=0700\n"));
}

#[test]
fn the_install_writes_all_three_units_verbatim_and_enables_every_game_server() {
    let env = base();
    let instances = env.fleet.instances();
    let p = units_install_payload(&env, &instances);
    assert!(p.contains(&format!(
        "cat > \"$UNITS/tbd-reforger@.service\" <<'UNITEOF'\n{}UNITEOF\n",
        game_server_unit(&env)
    )));
    assert!(p.contains(&format!(
        "cat > \"$UNITS/fleet_host_agent@.service\" <<'UNITEOF'\n{HOST_AGENT_TEMPLATE}UNITEOF\n"
    )));
    assert!(p.contains(&format!(
        "cat > \"$UNITS/acknowledgement-dropping-relay@.service\" <<'UNITEOF'\n{RELAY_TEMPLATE}UNITEOF\n"
    )));
    assert!(p.contains(
        "systemctl --user enable tbd-reforger@1.service tbd-reforger@2.service \
         tbd-reforger@3.service tbd-reforger@4.service tbd-reforger@5.service\n"
    ));
    // Five of five: no surplus instance; every relay instance but 5 is retired.
    assert!(p.contains("for n in ; do\n"), "{p}");
    assert!(p.contains("for n in 1 2 3 4; do\n  systemctl --user disable --now \"acknowledgement-dropping-relay@$n.service\""));
}

#[test]
fn a_smaller_fleet_retires_the_instances_above_it() {
    let mut env = base();
    env.fleet.instance_count = 3;
    env.fleet.relay = None;
    let p = units_install_payload(&env, &env.fleet.instances());
    assert!(p.contains(
        "for n in 4 5; do\n  systemctl --user disable --now \"tbd-reforger@$n.service\" \"fleet_host_agent@$n.service\""
    ));
    assert!(p.contains("for n in 1 2 3 4 5; do\n  systemctl --user disable --now \"acknowledgement-dropping-relay@$n.service\""));
    assert_eq!(
        game_servers_restart_payload(&env.fleet.instances()),
        "set -euo pipefail\nsystemctl --user restart tbd-reforger@1.service tbd-reforger@2.service tbd-reforger@3.service\n"
    );
}
