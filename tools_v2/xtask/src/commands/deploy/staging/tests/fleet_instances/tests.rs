//! The fleet's instances: the API origin, the port rules, names, visibility, folders and units.
use std::path::Path;

use super::*;
use crate::commands::deploy::staging::config::tests::base;

fn fleet() -> FleetSettings {
    base().fleet
}

fn environment(file: &str) -> DeployEnvironment {
    DeployEnvironment::from_text(Path::new("/deploy.env"), Some(file), Vec::new()).expect("parses")
}

/// `TBD_BACKEND_URL` has one reading for every command: its default, or the set origin without a
/// trailing `/`, which is also the origin the agents poll when `TBD_HOST_AGENT_API_URL` is unset.
#[test]
fn the_backend_url_has_one_reading_without_a_trailing_slash() {
    assert_eq!(backend_url(&environment("")), "http://127.0.0.1:8080");
    for file in [
        "TBD_BACKEND_URL=https://api.example.org\n",
        "TBD_BACKEND_URL=https://api.example.org/\n",
        "TBD_BACKEND_URL=https://api.example.org//\n",
    ] {
        assert_eq!(
            backend_url(&environment(file)),
            "https://api.example.org",
            "{file}"
        );
        let fleet = FleetSettings::from_environment(&environment(file)).expect("accepted");
        assert_eq!(fleet.agent_api_url, "https://api.example.org", "{file}");
    }
    let refused =
        FleetSettings::from_environment(&environment("TBD_BACKEND_URL=http://api.example.org\n"))
            .expect_err("plain http off the loopback");
    assert!(refused.to_string().contains("TBD_BACKEND_URL"), "{refused}");
}

/// Instance N's folder, `-profile` folder, secrets and server config under any fleet root, and
/// its game server unit, the one a text about any instance names as `tbd-reforger@N.service`.
#[test]
fn an_instance_folder_names_its_profile_secrets_and_server_config() {
    let folder = InstanceFolder::under("/home/deploy/tbd/fleet", 4);
    assert_eq!(folder.path(), "/home/deploy/tbd/fleet/instance-4");
    assert_eq!(
        folder.profile(),
        "/home/deploy/tbd/fleet/instance-4/profile"
    );
    assert_eq!(
        folder.secrets(),
        "/home/deploy/tbd/fleet/instance-4/secrets"
    );
    assert_eq!(
        folder.server_config(),
        "/home/deploy/tbd/fleet/instance-4/server.config.json"
    );
    let fourth = &fleet().instances()[3];
    assert_eq!(
        fourth.home_relative_folder(),
        InstanceFolder::under(FLEET_ROOT_UNDER_HOME, 4).path()
    );
    assert_eq!(fourth.game_server_unit(), game_server_unit_of(4));
    assert_eq!(game_server_unit_of("N"), "tbd-reforger@N.service");
}

/// Five instances on the documented bases: game 2000+N, A2S 17776+N, RCON 19998+N.
#[test]
fn five_instances_take_their_ports_from_the_three_bases() {
    let instances = fleet().instances();
    let ports: Vec<(u16, u16, u16, u16)> = instances
        .iter()
        .map(|i| (i.number, i.game_port, i.a2s_port, i.rcon_port))
        .collect();
    assert_eq!(
        ports,
        [
            (1, 2001, 17777, 19999),
            (2, 2002, 17778, 20000),
            (3, 2003, 17779, 20001),
            (4, 2004, 17780, 20002),
            (5, 2005, 17781, 20003),
        ]
    );
    assert_eq!(fleet().check_port_rules(), Ok(()));
}

#[test]
fn a_shared_or_overflowing_port_breaks_the_rules() {
    let mut clash = fleet();
    clash.rcon_port_base = clash.a2s_port_base + 2;
    let problem = clash.check_port_rules().unwrap_err();
    assert_eq!(
        problem,
        "port 17779 is both instance 1's RCON port and instance 3's A2S port"
    );
    let mut overflow = fleet();
    overflow.game_port_base = 65533;
    assert_eq!(
        overflow.check_port_rules().unwrap_err(),
        "instance 3's game port is TBD_FLEET_GAME_PORT_BASE 65533 + 3, above 65535"
    );
    let mut relay_on_a_game_port = fleet();
    relay_on_a_game_port.relay.as_mut().unwrap().port = 2004;
    assert_eq!(
        relay_on_a_game_port.check_port_rules().unwrap_err(),
        "TBD_FLEET_RELAY_PORT 2004 is also instance 4's game port"
    );
    let mut relay_on_the_api = fleet();
    relay_on_the_api.relay.as_mut().unwrap().port = 8080;
    assert!(relay_on_the_api.check_port_rules().is_err());
    let mut relay_zero = fleet();
    relay_zero.relay.as_mut().unwrap().port = 0;
    assert!(relay_zero.check_port_rules().is_err());
}

#[test]
fn only_instance_one_is_listed_and_every_instance_has_its_name_folder_and_units() {
    let instances = fleet().instances();
    let listed: Vec<bool> = instances
        .iter()
        .map(FleetInstance::listed_in_server_browser)
        .collect();
    assert_eq!(listed, [true, false, false, false, false]);
    let third = &instances[2];
    assert_eq!(third.server_name(), "TBD Staging 3");
    assert_eq!(third.home_relative_folder(), "tbd/fleet/instance-3");
    assert_eq!(third.game_server_unit(), "tbd-reforger@3.service");
    assert_eq!(third.host_agent_unit(), "fleet-host-agent@3.service");
    assert_eq!(third.relay_unit(), None);
}

/// Only the relay instance's agent polls the relay; every other agent polls the API.
#[test]
fn the_relay_instance_alone_polls_the_relay() {
    let instances = fleet().instances();
    for instance in &instances[..4] {
        assert_eq!(instance.agent_api_url, "http://127.0.0.1:8080");
        assert_eq!(instance.relay_port, None);
    }
    let fifth = &instances[4];
    assert_eq!(fifth.agent_api_url, "http://127.0.0.1:18085");
    assert_eq!(fifth.relay_port, Some(18085));
    assert_eq!(
        fifth.relay_unit().as_deref(),
        Some("acknowledgement-dropping-relay@5.service")
    );
    let mut without_relay = fleet();
    without_relay.relay = None;
    assert!(
        without_relay
            .instances()
            .iter()
            .all(|i| i.relay_port.is_none() && i.agent_api_url == "http://127.0.0.1:8080")
    );
}

#[test]
fn agent_origins_are_https_or_loopback_http() {
    for accepted in [
        "https://tbd.example.org",
        "http://127.0.0.1:8080",
        "http://localhost:8080/",
    ] {
        assert!(is_agent_origin(accepted), "{accepted}");
    }
    for refused in [
        "http://tbd.example.org",
        "http://127.0.0.2:8080",
        "ftp://127.0.0.1",
    ] {
        assert!(!is_agent_origin(refused), "{refused}");
    }
    assert_eq!(http_port("http://127.0.0.1:8080"), Some(8080));
    assert_eq!(http_port("http://127.0.0.1/"), Some(80));
}
