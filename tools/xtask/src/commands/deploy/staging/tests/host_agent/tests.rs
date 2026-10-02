//! Each instance's agent configuration and the agents' install.
use super::*;
use crate::commands::deploy::staging::config::tests::base;
use crate::commands::deploy::staging::remote::scenario_of_config;

#[test]
fn each_agent_configuration_names_its_instance_files_and_unit() {
    let instances = base().fleet.instances();
    assert_eq!(
        agent_configuration(&instances[1]),
        "api_base_url = \"http://127.0.0.1:8080\"\n\
         credential_file = \"$HOME/tbd/fleet/instance-2/secrets/host-agent-credential\"\n\
         poll_interval_seconds = 5\n\
         \n\
         [game_server]\n\
         systemd_user_unit = \"tbd-reforger@2.service\"\n\
         server_config_path = \"$HOME/tbd/fleet/instance-2/server.config.json\"\n\
         \n\
         [rcon]\n\
         address = \"127.0.0.1\"\n\
         port = 20000\n\
         password_file = \"$HOME/tbd/fleet/instance-2/secrets/rcon-password\"\n"
    );
    // The relay instance's agent polls the relay.
    assert!(
        agent_configuration(&instances[4])
            .starts_with("api_base_url = \"http://127.0.0.1:18085\"\n")
    );
}

#[test]
fn the_install_writes_every_configuration_and_reads_every_unit_back() {
    let env = base();
    let p = host_agents_install_payload(&env.remote_dir, &env.fleet.instances());
    assert!(
        p.starts_with("set -euo pipefail\nexport PATH=\"$HOME/.cargo/bin:$PATH\"\numask 077\n")
    );
    assert!(p.contains(
        "(cd '/home/deploy/tbd/repo' && cargo build --release -q -p fleet-host-agent)\n"
    ));
    for n in 1..=5 {
        assert!(p.contains(&format!(
            "AGENT_DIR=\"$HOME/.config/fleet-host-agent/instance-{n}\"\nmkdir -p \"$AGENT_DIR\"\nchmod 700 \"$AGENT_DIR\"\n"
        )));
    }
    assert_eq!(p.matches("<<AGENTTOML\n").count(), 5);
    assert_eq!(
        p.matches("chmod 600 \"$AGENT_DIR/agent.toml\"\n").count(),
        5
    );
    let units = "fleet-host-agent@1.service fleet-host-agent@2.service fleet-host-agent@3.service \
                 fleet-host-agent@4.service fleet-host-agent@5.service";
    assert!(p.contains(&format!("systemctl --user restart {units}\n")));
    assert!(p.contains(&format!("for unit in {units}; do\n")));
    assert!(p.contains("show -p ActiveState --value \"$unit\""));
    assert!(p.ends_with("exit \"$failed\"\n"));
    // It names secret files and never writes a secret.
    assert!(!p.contains("printf '%s'"), "{p}");
}

/// ssh runs the payload in a non-login shell, which never reads `~/.profile`: the agent build
/// finds `cargo` only because the payload puts the toolchain on `PATH` before it.
#[test]
fn the_install_puts_the_rust_toolchain_on_path_before_the_agent_build() {
    let env = base();
    let p = host_agents_install_payload(&env.remote_dir, &env.fleet.instances());
    let toolchain = p
        .find(crate::commands::deploy::remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH)
        .expect("the payload puts the toolchain on PATH");
    assert!(toolchain < p.find("cargo build").expect("the payload builds the agent"));
}

#[test]
fn the_live_scenario_is_read_only_from_a_valid_value() {
    assert_eq!(
        scenario_of_config("{0123456789ABCDEF}Missions/Deployed.conf\n").as_deref(),
        Some("{0123456789ABCDEF}Missions/Deployed.conf")
    );
    assert_eq!(scenario_of_config(""), None, "no config yet");
    assert_eq!(scenario_of_config("{69A85365FC09E2CA"), None);
}
