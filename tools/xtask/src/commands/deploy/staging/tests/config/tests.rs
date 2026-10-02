use super::*;
use crate::commands::deploy::staging::fleet_instances::RelaySettings;

/// Shared fixture: a fully-resolved `Env` with the documented defaults and the staging fleet of
/// five instances, the relay on instance 5. `pub(crate)` so every other test module of the deploy
/// uses the SAME baseline — two drifting fixtures would let a render test pass against inputs the
/// loader can no longer produce.
pub(crate) fn base() -> Env {
    Env {
        deploy_host: DeployHost::parse("deploy@192.0.2.10").expect("parses"),
        remote_dir: "/home/deploy/tbd/repo".into(),
        profile_dir: "/home/deploy/tbd/profile".into(),
        addons_staging: "/home/deploy/tbd/addons".into(),
        backend_url: "http://127.0.0.1:8080".into(),
        addon_guid: "B2C3D4E5F6A78901".into(),
        scenario: "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf".into(),
        server_dir: "/home/deploy/steam/arma-reforger-server".into(),
        workshop_mod_id: "5EAF00DBEEF01234".into(),
        public_address: Ipv4Addr::new(192, 0, 2, 10),
        admin_password: "tbd-admin".into(),
        max_players: "64".into(),
        admin_identity_ids: String::new(),
        boot_verify_timeout: "180".into(),
        modpack_json: String::new(),
        modpack_url: String::new(),
        modpack_token: String::new(),
        workshop_mod_name: "TBD_Framework".into(),
        fleet: FleetSettings {
            instance_count: 5,
            game_port_base: 2000,
            a2s_port_base: 17776,
            rcon_port_base: 19998,
            agent_api_url: "http://127.0.0.1:8080".into(),
            relay: Some(RelaySettings {
                instance: 5,
                port: 18085,
                upstream: "http://127.0.0.1:8080".into(),
            }),
        },
        ssh_pass: None,
        ssh_identity_file: None,
    }
}

const SETTINGS_FILE: &str = "/home/deploy/checkout/deploy/deploy.env";

/// The settings of a `deploy.env` holding `file`, beside the given process variables.
fn settings(file: &str, process: &[(&str, &str)]) -> Result<DeployEnvironment, String> {
    DeployEnvironment::from_text(
        Path::new(SETTINGS_FILE),
        Some(file),
        process
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string())),
    )
    .map_err(|error| error.to_string())
}

fn env_from(file: &str, process: &[(&str, &str)]) -> Result<Env, u8> {
    Env::from_environment(&settings(file, process).expect("parses"))
}

#[test]
fn dirname_matches_coreutils() {
    assert_eq!(dirname("/p/q"), "/p");
    assert_eq!(dirname("/p"), "/");
    assert_eq!(dirname("p"), ".");
    assert_eq!(dirname("/p/q/"), "/p");
    assert_eq!(dirname("/"), "/");
}

#[test]
fn xargs_like_trims_and_collapses() {
    assert_eq!(xargs_like("  a  "), "a");
    assert_eq!(xargs_like("a \t b\nc"), "a b c");
    assert_eq!(xargs_like("   "), "");
}

#[test]
fn admin_id_schema_is_the_engines() {
    let mut e = base();
    e.admin_identity_ids = "deadbeef-0000-4000-8000-000000000001".into();
    assert!(e.validate(Path::new("/nonexistent")).is_ok());
    e.admin_identity_ids = "11111111111111111".into();
    assert!(e.validate(Path::new("/nonexistent")).is_ok());
    // Uppercase hex is NOT an identityId by the engine's pattern — pinned because "helpfully"
    // case-folding here would let a config through that the engine kills 90 s into a boot.
    e.admin_identity_ids = "DEADBEEF-0000-4000-8000-000000000001".into();
    assert!(e.validate(Path::new("/nonexistent")).is_err());
    e.admin_identity_ids = "1234".into();
    assert!(e.validate(Path::new("/nonexistent")).is_err());
    // Whitespace around an id is stripped (`| xargs`) and a bare `,,` entry is skipped.
    e.admin_identity_ids = "  11111111111111111 , ,".into();
    assert!(e.validate(Path::new("/nonexistent")).is_ok());
    assert_eq!(e.admin_count(), 1);
}

#[test]
fn the_fleet_needs_a_mod_source_and_distinct_ports() {
    let mut e = base();
    e.workshop_mod_id = String::new();
    assert!(
        e.validate(Path::new("/nonexistent")).is_err(),
        "no mod source at all"
    );
    // …satisfied by a modpack file instead of the single-mod id.
    e.modpack_json = "/tmp/pack.json".into();
    assert!(e.validate(Path::new("/nonexistent")).is_ok());
    // An A2S base that puts instance 1's A2S port on instance 2's game port stops the deploy
    // before anything is sent.
    e.fleet.a2s_port_base = e.fleet.game_port_base + 1;
    assert!(e.validate(Path::new("/nonexistent")).is_err());
}

#[test]
fn prairielearn_is_refused_anywhere_in_the_path() {
    let mut e = base();
    e.remote_dir = "/home/deploy/prairielearn/x".into();
    assert!(e.validate(Path::new("/nonexistent")).is_err());
    // ODDITY PRESERVED: the bash used a `*prairielearn*` glob, which is CASE SENSITIVE, so
    // `/home/deploy/PrairieLearn/x` was allowed through. `gate_deploy_website.rs` case-folds for
    // its own script; this one does not, because the two scripts made different checks and
    // silently widening a refusal is still a behaviour change.
    e.remote_dir = "/home/deploy/PrairieLearn/x".into();
    assert!(e.validate(Path::new("/nonexistent")).is_ok());
}

#[test]
fn mod_source_label_names_the_actual_source() {
    // The bash printed only `modId=$TBD_WORKSHOP_MOD_ID`, which read as "the mod list is fine"
    // on a run whose mod list came from nowhere near the operator's modpack.
    let mut e = base();
    assert!(e.mod_source_label().starts_with("single-mod env fallback"));
    e.modpack_url = "https://x/y".into();
    assert_eq!(e.mod_source_label(), "modpack API https://x/y");
    e.modpack_json = "/p.json".into();
    assert_eq!(e.mod_source_label(), "modpack file /p.json");
}

#[test]
fn deploy_env_file_beats_the_process_environment() {
    let head = "# comment\nexport TBD_SSH_HOST=\"deploy@192.0.2.10\"\n\
                TBD_PROFILE_DIR=/p/q\nTBD_MAX_PLAYERS=12\nTBD_SSH_PASS=\n";
    let process = [
        ("TBD_SSH_HOST", "deploy@198.51.100.7"),
        ("TBD_MAX_PLAYERS", "1"),
        ("TBD_SSH_PASS", "exported"),
        ("TBD_MODPACK_JSON", "/tmp/pack.json"),
    ];
    let e = env_from(head, &process).expect("loads");
    assert_eq!(e.deploy_host.ssh_destination(), "deploy@192.0.2.10");
    assert_eq!(e.max_players, "12");
    assert_eq!(
        e.ssh_pass, None,
        "an empty assignment is not filled from the environment"
    );
    assert_eq!(
        e.modpack_json, "/tmp/pack.json",
        "a key the file never assigns"
    );
    // Defaults: the folders under the deploy user's home, and the fleet of five with its ports.
    assert_eq!(e.remote_dir, "/home/deploy/tbd/repo");
    assert_eq!(e.addons_staging, "/home/deploy/tbd/addons-staging");
    assert_eq!(e.server_dir, "/home/deploy/steam/arma-reforger-server");
    assert_eq!(e.fleet.instance_count, 5);
    assert_eq!(
        (
            e.fleet.game_port_base,
            e.fleet.a2s_port_base,
            e.fleet.rcon_port_base
        ),
        (2000, 17776, 19998)
    );
    assert_eq!(
        e.fleet.relay, None,
        "the relay runs only when an instance is named"
    );
    // The single-instance config the migration archives sits beside TBD_PROFILE_DIR.
    assert_eq!(e.single_instance_server_config(), "/p/server.config.json");
    // The scenario default is NOT truncated — the measured brace-expansion defect.
    assert_eq!(e.scenario, "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf");
    // A missing file is the documented rc-1 message, not a panic.
    assert!(Env::load(Path::new("/nonexistent/deploy.env")).is_err());
}

/// Every setting the fleet no longer reads is refused, all of them at once, and the refusal names
/// the line and the replacement but never the value.
#[test]
fn a_retired_setting_is_refused_with_its_replacement_and_without_its_value() {
    let mut file = String::from("TBD_SSH_HOST=deploy@192.0.2.10\n");
    for (index, (key, _)) in RETIRED_SETTINGS.iter().enumerate() {
        file.push_str(&format!("{key}=retired-value-{index}\n"));
    }
    assert!(env_from(&file, &[]).is_err());
    let environment = settings(&file, &[]).expect("parses");
    let refusals = retired_setting_refusals(&environment);
    assert_eq!(refusals.len(), RETIRED_SETTINGS.len());
    for (index, refusal) in refusals.iter().enumerate() {
        let text = refusal.to_string();
        let (key, replacement) = RETIRED_SETTINGS[index];
        assert!(
            text.starts_with(&format!(
                "{SETTINGS_FILE}:{}: {key}: is no longer read",
                index + 2
            )),
            "{text}"
        );
        assert!(text.contains(replacement), "{text}");
        assert!(
            !text.contains("retired-value"),
            "a refusal printed the value: {text}"
        );
    }
    for secret in [
        "TBD_MOD_RUNTIME_CREDENTIAL",
        "TBD_HOST_AGENT_CREDENTIAL",
        "TBD_RCON_PASSWORD",
    ] {
        assert!(
            RETIRED_SETTINGS.iter().any(|(key, _)| *key == secret),
            "{secret}"
        );
    }
    // An exported retired key is refused too, so no stale shell variable is silently ignored.
    let exported = settings(
        "TBD_SSH_HOST=deploy@192.0.2.10\n",
        &[("TBD_GAME_PORT", "2001")],
    )
    .expect("parses");
    assert_eq!(retired_setting_refusals(&exported).len(), 1);
}

#[test]
fn the_relay_is_read_only_with_its_port_and_a_loopback_upstream() {
    let head = "TBD_SSH_HOST=deploy@192.0.2.10\nTBD_FLEET_RELAY_INSTANCE=5\n";
    assert!(env_from(head, &[]).is_err(), "the relay port is required");
    let e = env_from(&format!("{head}TBD_FLEET_RELAY_PORT=18085\n"), &[]).expect("loads");
    assert_eq!(
        e.fleet.relay,
        Some(RelaySettings {
            instance: 5,
            port: 18085,
            upstream: "http://127.0.0.1:8080".into(),
        })
    );
    let public_upstream = format!(
        "{head}TBD_FLEET_RELAY_PORT=18085\nTBD_HOST_AGENT_API_URL=https://tbd.example.org\n"
    );
    assert!(
        env_from(&public_upstream, &[]).is_err(),
        "the relay forwards to loopback only"
    );
    let outside = "TBD_SSH_HOST=deploy@192.0.2.10\nTBD_FLEET_INSTANCES=3\n\
                   TBD_FLEET_RELAY_INSTANCE=4\nTBD_FLEET_RELAY_PORT=18084\n";
    assert!(
        env_from(outside, &[]).is_err(),
        "the relay instance must exist"
    );
}

#[test]
fn the_agents_api_origin_must_be_https_or_loopback_http() {
    let head = "TBD_SSH_HOST=deploy@192.0.2.10\n";
    let e = env_from(head, &[]).expect("loads");
    assert_eq!(
        e.fleet.agent_api_url, "http://127.0.0.1:8080",
        "defaults to TBD_BACKEND_URL"
    );
    for refused in [
        "TBD_HOST_AGENT_API_URL=http://tbd.example.org\n",
        "TBD_BACKEND_URL=http://tbd.example.org\n",
        "TBD_FLEET_INSTANCES=6\n",
        "TBD_FLEET_INSTANCES=0\n",
        "TBD_FLEET_GAME_PORT_BASE=two-thousand\n",
    ] {
        assert!(
            env_from(&format!("{head}{refused}"), &[]).is_err(),
            "{refused}"
        );
    }
    let https = env_from(
        &format!("{head}TBD_HOST_AGENT_API_URL=https://tbd.example.org\n"),
        &[],
    )
    .expect("loads");
    assert_eq!(https.fleet.agent_api_url, "https://tbd.example.org");
}

/// The deploy reads `TBD_BACKEND_URL` through the fleet's one reader, the reading `cargo xtask
/// staging` shares: the profiles' `backendUrl` and the agents' origin carry no trailing `/`.
#[test]
fn the_backend_url_is_the_one_reading_without_a_trailing_slash() {
    let file = "TBD_SSH_HOST=deploy@192.0.2.10\nTBD_BACKEND_URL=https://api.example.org/\n";
    let e = env_from(file, &[]).expect("loads");
    assert_eq!(e.backend_url, "https://api.example.org");
    assert_eq!(
        e.backend_url,
        fleet_instances::backend_url(&settings(file, &[]).expect("parses"))
    );
    assert_eq!(e.fleet.agent_api_url, "https://api.example.org");
    let profile = crate::commands::deploy::staging::payloads::instance_profile_commands(
        &e.remote_dir,
        &e.backend_url,
    );
    assert!(
        profile.contains("\"backendUrl\": \"https://api.example.org\"|"),
        "{profile}"
    );
}

#[test]
fn a_command_line_in_the_deploy_file_is_refused_and_never_run() {
    let d = std::env::temp_dir().join(format!("tbd-deploy-env-command-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    let canary = d.join("canary");
    let f = d.join("deploy.env");
    std::fs::write(
        &f,
        format!(
            "TBD_SSH_HOST=deploy@192.0.2.10\ntouch {}\n",
            canary.display()
        ),
    )
    .unwrap();
    assert!(
        Env::load(&f).is_err(),
        "a line that is not KEY=VALUE stops the deploy"
    );
    assert!(!canary.exists(), "deploy.env must never be executed");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_host_without_a_user_needs_its_folders_set() {
    let file = "TBD_SSH_HOST=192.0.2.10\n".to_string();
    assert!(env_from(&file, &[]).is_err());
    let folders = format!(
        "{file}TBD_REMOTE_DIR=/r\nTBD_PROFILE_DIR=/p/q\nTBD_ADDONS_STAGING=/a\nTBD_SERVER_DIR=/s\n"
    );
    let e = env_from(&folders, &[]).expect("loads");
    assert_eq!(e.deploy_host.ssh_destination(), "192.0.2.10");
    assert_eq!(e.server_dir, "/s");
}

fn address_of(host: &str, explicit: Option<&str>) -> Result<Ipv4Addr, SettingError> {
    let mut file = format!("TBD_SSH_HOST={host}\n");
    if let Some(value) = explicit {
        file.push_str(&format!("TBD_PUBLIC_ADDRESS={value}\n"));
    }
    let environment = settings(&file, &[]).expect("parses");
    let deploy_host = environment.deploy_host().expect("parses");
    public_address(&environment, &deploy_host)
}

#[test]
fn the_public_address_is_explicit_or_the_hosts_first_ipv4_address() {
    assert_eq!(
        address_of("deploy@192.0.2.10", Some("198.51.100.7")),
        Ok(Ipv4Addr::new(198, 51, 100, 7))
    );
    assert_eq!(
        address_of("deploy@192.0.2.10", None),
        Ok(Ipv4Addr::new(192, 0, 2, 10))
    );
    // `localhost` comes from /etc/hosts; resolvers that list ::1 first still yield IPv4.
    assert_eq!(
        address_of("deploy@localhost", None),
        Ok(Ipv4Addr::LOCALHOST)
    );
}

#[test]
fn a_bad_or_underivable_public_address_stops_the_deploy() {
    let bad = address_of("deploy@192.0.2.10", Some("staging.example")).unwrap_err();
    assert_eq!(
        bad.to_string(),
        format!("{SETTINGS_FILE}:2: TBD_PUBLIC_ADDRESS: `staging.example` is not an IPv4 address")
    );
    let underivable = address_of("deploy@::1", None).unwrap_err();
    assert_eq!(
        underivable.to_string(),
        format!(
            "TBD_PUBLIC_ADDRESS is unset and ::1 has no IPv4 address from here \
             (it resolves to IPv6 only: ::1): run avahi-daemon on the host or set \
             TBD_PUBLIC_ADDRESS in {SETTINGS_FILE}"
        )
    );
}
