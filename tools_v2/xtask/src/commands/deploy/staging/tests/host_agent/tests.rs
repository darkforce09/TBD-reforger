//! The host agent's settings, the rcon block it adds to the server config, and its install.
use super::*;
use crate::commands::deploy::staging::config::tests::{RUNTIME_CREDENTIAL, base};
use crate::commands::deploy::staging::remote::scenario_of_config;
use crate::commands::deploy::staging::render::render_server_config;

const AGENT_CREDENTIAL: &str = "tbdm_fedcba9876543210fedcba9876543210_fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

fn settings() -> HostAgentSettings {
    HostAgentSettings {
        credential: AGENT_CREDENTIAL.into(),
        rcon_password: "rcon-secret".into(),
        rcon_port: "19999".into(),
        api_base_url: "http://127.0.0.1:8080".into(),
    }
}

#[test]
fn machine_credentials_must_have_the_issued_shape() {
    assert_eq!(validate_machine_credential("K", RUNTIME_CREDENTIAL), Ok(()));
    for bad in [
        "",
        "replace-with-the-mod_runtime-machine-credential-of-this-server",
        "tbdm_0123456789ABCDEF0123456789abcdef_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "tbdm_0123456789abcdef0123456789abcdef_0123",
        "tbdx_0123456789abcdef0123456789abcdef_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    ] {
        assert_eq!(validate_machine_credential("K", bad), Err(1), "{bad}");
    }
}

#[test]
fn settings_the_agent_or_the_engine_would_refuse_are_refused_first() {
    assert_eq!(settings().validate("config"), Ok(()));
    assert_eq!(
        settings().validate("addons"),
        Err(1),
        "mission restarts need a config file"
    );
    for password in ["ab", "has space", "quo\"te", "sin'gle", "back\\slash"] {
        let mut s = settings();
        s.rcon_password = password.into();
        assert_eq!(s.validate("config"), Err(1), "{password}");
    }
    for port in ["0", "65536", "port"] {
        let mut s = settings();
        s.rcon_port = port.into();
        assert_eq!(s.validate("config"), Err(1), "{port}");
    }
    let mut remote_http = settings();
    remote_http.api_base_url = "http://tbd.example.org".into();
    assert_eq!(remote_http.validate("config"), Err(1));
    let mut https = settings();
    https.api_base_url = "https://tbd.example.org".into();
    assert_eq!(https.validate("config"), Ok(()));
}

#[test]
fn install_payload_writes_secret_files_and_reads_the_unit_state_back() {
    let p = install_payload(
        &settings(),
        "/home/sam/tbd/repo",
        "/home/sam/tbd/server.config.json",
    );
    assert!(p.starts_with("set -euo pipefail\numask 077\n"));
    assert!(
        p.contains("(cd '/home/sam/tbd/repo' && cargo build --release -q -p fleet-host-agent)")
    );
    assert!(p.contains(&format!(
        "printf '%s' '{AGENT_CREDENTIAL}' > \"$AGENT_DIR/machine-credential\""
    )));
    assert!(p.contains("printf '%s' 'rcon-secret' > \"$AGENT_DIR/rcon-password\""));
    assert!(p.contains("chmod 600 \"$AGENT_DIR/machine-credential\" \"$AGENT_DIR/rcon-password\""));
    // The configuration names the unit the agent controls and the config it rewrites.
    assert!(p.contains("systemd_user_unit = \"tbd-reforger.service\"\n"));
    assert!(p.contains("server_config_path = \"/home/sam/tbd/server.config.json\"\n"));
    assert!(p.contains("[rcon]\naddress = \"127.0.0.1\"\nport = 19999\n"));
    // The unit goes over verbatim from the committed template.
    assert!(p.contains(&format!("<<'UNITEOF'\n{UNIT_TEMPLATE}UNITEOF\n")));
    assert!(p.contains("RestartPreventExitStatus=78"));
    // Do not trust the enable: read the state back and fail the deploy when it is not active.
    assert!(p.contains("show -p ActiveState --value fleet-host-agent.service"));
    assert!(p.contains("if [ \"$state\" != \"active\" ]; then\n"));
}

#[test]
fn server_config_gains_a_loopback_monitor_rcon_block_for_the_agent() {
    let dir = std::env::temp_dir().join(format!("tbd-rcon-render-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut env = base();
    env.host_agent = Some(settings());
    let path = dir.join("with-agent.json");
    render_server_config(&env, "{0123456789ABCDEF}Missions/Deployed.conf", &path).unwrap();
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        config["rcon"],
        serde_json::json!({ "address": "127.0.0.1", "port": 19999, "password": "rcon-secret",
                            "permission": "monitor", "maxClients": 2 })
    );
    assert_eq!(
        config["game"]["scenarioId"],
        "{0123456789ABCDEF}Missions/Deployed.conf"
    );
    let without = dir.join("without-agent.json");
    render_server_config(&base(), &base().scenario, &without).unwrap();
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&without).unwrap()).unwrap();
    assert!(config.get("rcon").is_none());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_live_scenario_is_read_only_from_a_valid_config() {
    let config = r#"{"game": {"scenarioId": "{0123456789ABCDEF}Missions/Deployed.conf"}}"#;
    assert_eq!(
        scenario_of_config(config).as_deref(),
        Some("{0123456789ABCDEF}Missions/Deployed.conf")
    );
    assert_eq!(scenario_of_config(""), None, "no config yet");
    assert_eq!(scenario_of_config("{\"game\": {}}"), None);
    assert_eq!(
        scenario_of_config(r#"{"game": {"scenarioId": "{69A85365FC09E2CA"}}"#),
        None
    );
}
