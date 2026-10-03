use super::*;
use deployment::staging::fleet_instances::MAXIMUM_FLEET_INSTANCES;
use std::path::Path;

fn settings(file: &str) -> DeployEnvironment {
    DeployEnvironment::from_text(Path::new("/nonexistent/deploy.env"), Some(file), [])
        .expect("the settings parse")
}

#[test]
fn staging_fleet_instance_ports_follow_the_instance_number() {
    let defaults = settings("");
    for number in 1..=MAXIMUM_FLEET_INSTANCES {
        let instance = select_fleet_instance(&defaults, number).expect("in range");
        assert_eq!(instance.number, number);
        assert_eq!(instance.game_port, 2000 + number);
        assert_eq!(instance.a2s_port, 17776 + number);
        assert_eq!(
            instance.game_server_unit(),
            format!("tbd-reforger@{number}.service")
        );
        assert_eq!(
            profile_under_home(&instance),
            format!("tbd/fleet/instance-{number}/profile")
        );
    }
    let moved = settings("TBD_FLEET_GAME_PORT_BASE=3000\nTBD_FLEET_A2S_PORT_BASE=4000\n");
    let third = select_fleet_instance(&moved, 3).expect("in range");
    assert_eq!((third.game_port, third.a2s_port), (3003, 4003));
}

/// The selected instance is the one `cargo xtask deploy staging` deploys from the same file,
/// relay and agent origin included.
#[test]
fn staging_fleet_instance_is_the_deploys_own_instance() {
    let file = "TBD_FLEET_INSTANCES=4\nTBD_FLEET_GAME_PORT_BASE=3100\nTBD_FLEET_RELAY_INSTANCE=4\n\
                TBD_FLEET_RELAY_PORT=18085\n";
    let environment = settings(file);
    let deployed = FleetSettings::from_environment(&environment)
        .expect("the deploy accepts the file")
        .instances();
    assert_eq!(deployed.len(), 4);
    for instance in &deployed {
        assert_eq!(
            &select_fleet_instance(&environment, instance.number).expect("in range"),
            instance
        );
    }
    let relayed = select_fleet_instance(&environment, 4).expect("in range");
    assert_eq!(relayed.game_port, 3104);
    assert_eq!(
        relayed.relay_unit().as_deref(),
        Some("acknowledgement-dropping-relay@4.service")
    );
    assert_eq!(relayed.agent_api_url, "http://127.0.0.1:18085");
}

#[test]
fn staging_fleet_instance_outside_the_fleet_is_refused() {
    for number in [0, 6] {
        assert!(matches!(
            select_fleet_instance(&settings(""), number),
            Err(InstanceSelectionError::OutOfRange { count: 5, .. })
        ));
    }
    let three = settings("TBD_FLEET_INSTANCES=3\n");
    let refusal = select_fleet_instance(&three, 4).expect_err("instance 4 of 3");
    assert_eq!(
        refusal.to_string(),
        "--instance 4 names no instance; the fleet runs instances 1 to 3"
    );
    assert!(select_fleet_instance(&three, 3).is_ok());
    // An instance outside the fleet is named as such even when a port rule is broken as well.
    assert!(matches!(
        select_fleet_instance(&settings("TBD_FLEET_GAME_PORT_BASE=65535\n"), 7),
        Err(InstanceSelectionError::OutOfRange {
            number: 7,
            count: 5
        })
    ));
}

#[test]
fn staging_fleet_instance_malformed_settings_are_setting_errors() {
    for file in [
        "TBD_FLEET_INSTANCES=six\n",
        "TBD_FLEET_INSTANCES=9\n",
        "TBD_FLEET_GAME_PORT_BASE=65535\n",
        "TBD_FLEET_A2S_PORT_BASE=-1\n",
        // The rules the deploy adds: distinct ports, an agent origin it accepts, a relay port.
        "TBD_FLEET_GAME_PORT_BASE=17776\n",
        "TBD_BACKEND_URL=http://staging.example.org\n",
        "TBD_FLEET_RELAY_INSTANCE=5\n",
    ] {
        assert!(
            matches!(
                select_fleet_instance(&settings(file), 1),
                Err(InstanceSelectionError::Setting(_))
            ),
            "{file}"
        );
    }
    let colliding = select_fleet_instance(&settings("TBD_FLEET_GAME_PORT_BASE=17776\n"), 2)
        .expect_err("a port shared by two uses");
    assert_eq!(
        colliding.to_string(),
        "/nonexistent/deploy.env: port 17777 is both instance 1's game port and instance 1's A2S port"
    );
}
