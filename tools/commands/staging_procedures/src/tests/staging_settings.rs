//! The harness reads its settings from `deploy.env`: the host and its folders, the staging
//! deploy's own fleet, the defaults, and a missing key named when a procedure needs it.
use std::path::Path;

use super::*;
use crate::procedure_runner::runner_support::test_settings;

fn settings_from(text: &str) -> Result<StagingSettings> {
    let environment =
        DeployEnvironment::from_text(Path::new("/deploy.env"), Some(text), Vec::new()).unwrap();
    StagingSettings::from_environment(&environment)
}

#[test]
fn staging_settings_read_the_host_the_fleet_and_the_procedure_keys() {
    let settings = test_settings();
    assert_eq!(settings.host.ssh_destination(), "deploy@192.0.2.10");
    assert_eq!(settings.checkout, "/home/deploy/tbd/repo");
    assert_eq!(
        settings.server_install,
        "/home/deploy/steam/arma-reforger-server"
    );
    assert_eq!(settings.fleet_root(), "/home/deploy/tbd/fleet");
    assert_eq!(
        settings.api_env_file(),
        "/home/deploy/tbd/repo/apps/api/.env"
    );
    assert_eq!(settings.api_origin, "http://127.0.0.1:8080");
    assert_eq!(settings.api_unit, "tbd-website-api.service");
    assert_eq!(settings.database_container, "tbd_staging_db");
    assert_eq!(settings.operator().unwrap(), "123456789012345678");
    assert_eq!(
        settings.load_source_addresses,
        ["192.0.2.117", "192.0.2.240"]
    );
    assert_eq!(settings.server_address().unwrap(), "192.0.2.10");
    let mut published = settings.clone();
    published.public_address = Some("203.0.113.7".into());
    assert_eq!(published.server_address().unwrap(), "203.0.113.7");
    assert_eq!(settings.game_server_units()[4], "tbd-reforger@5.service");
    assert_eq!(settings.host_agent_units()[0], "fleet_host_agent@1.service");
    assert_eq!(
        settings.relay_unit(),
        Some((5, "acknowledgement-dropping-relay@5.service".to_string()))
    );
}

/// The harness reads the API origin through the staging deploy's reader, so the `backendUrl` W12
/// writes into a profile never differs from the deploy's by a trailing `/`; the fleet root is the
/// deploy's too.
#[test]
fn staging_settings_read_the_api_origin_and_fleet_root_as_the_deploy_does() {
    let file = "TBD_SSH_HOST=sam@dooley.local\nTBD_BACKEND_URL=https://api.example.org/\n";
    let settings = settings_from(file).unwrap();
    let environment =
        DeployEnvironment::from_text(Path::new("/deploy.env"), Some(file), Vec::new()).unwrap();
    assert_eq!(settings.api_origin, backend_url(&environment));
    assert_eq!(settings.api_origin, "https://api.example.org");
    assert_eq!(settings.fleet.agent_api_url, "https://api.example.org");
    assert_eq!(
        settings.fleet_root(),
        format!("/home/sam/{FLEET_ROOT_UNDER_HOME}")
    );
}

#[test]
fn staging_settings_default_what_the_file_leaves_out_and_name_what_a_procedure_needs() {
    let settings = settings_from("TBD_SSH_HOST=sam@dooley.local\n").unwrap();
    assert_eq!(settings.fleet.instance_count, 5);
    assert_eq!(settings.relay_unit(), None);
    assert_eq!(settings.ssh, SshBase::Plain);
    assert!(settings.load_source_addresses.is_empty());
    assert!(
        format!("{:#}", settings.operator().unwrap_err())
            .contains("TBD_STAGING_OPERATOR_DISCORD_ID")
    );
    let no_user = settings_from("TBD_SSH_HOST=dooley.local\n").unwrap_err();
    assert!(
        format!("{no_user:#}").contains("names no user"),
        "{no_user:#}"
    );
    assert!(settings_from("TBD_SSH_HOST=sam@dooley.local\nTBD_FLEET_INSTANCES=9\n").is_err());
}
