use super::*;

/// A well-formed `mod_runtime` machine credential; no server accepts it.
pub(crate) const RUNTIME_CREDENTIAL: &str = "tbdm_0123456789abcdef0123456789abcdef_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Shared fixture: a fully-resolved `Env` with the documented defaults. `pub(super)` so
/// [`super::super::render`]'s tests use the SAME baseline — two drifting fixtures would let a
/// render test pass against inputs the loader can no longer produce.
pub(crate) fn base() -> Env {
    Env {
        deploy_host: DeployHost::parse("deploy@192.0.2.10").expect("parses"),
        remote_dir: "/home/deploy/tbd/repo".into(),
        profile_dir: "/home/deploy/tbd/profile".into(),
        addons_staging: "/home/deploy/tbd/addons".into(),
        mod_runtime_credential: RUNTIME_CREDENTIAL.into(),
        backend_url: "http://127.0.0.1:8080".into(),
        addon_guid: "B2C3D4E5F6A78901".into(),
        scenario: "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf".into(),
        server_dir: "/home/deploy/steam/arma-reforger-server".into(),
        server_mode: "config".into(),
        workshop_mod_id: "5EAF00DBEEF01234".into(),
        public_address: Ipv4Addr::new(192, 0, 2, 10),
        game_port: "2001".into(),
        a2s_port: "17777".into(),
        server_name: "TBD Staging POC".into(),
        admin_password: "tbd-admin".into(),
        max_players: "64".into(),
        admin_identity_ids: String::new(),
        server_config_remote: "/home/deploy/tbd/server.config.json".into(),
        boot_verify_timeout: "180".into(),
        modpack_json: String::new(),
        modpack_url: String::new(),
        modpack_token: String::new(),
        workshop_mod_name: "TBD_Framework".into(),
        host_agent: None,
        ssh_pass: None,
        ssh_identity_file: None,
    }
}

const SETTINGS_FILE: &str = "/home/deploy/checkout/tools_v2/xtask/deploy/deploy.env";

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
fn mode_gate_matches_the_bash_case() {
    let mut e = base();
    e.server_mode = "bogus".into();
    assert!(e.validate(Path::new("/nonexistent")).is_err());
    // addons mode skips every config-mode requirement, including the port rule.
    e.server_mode = "addons".into();
    e.a2s_port = e.game_port.clone();
    e.workshop_mod_id = String::new();
    assert!(e.validate(Path::new("/nonexistent")).is_ok());
    // config mode with no mod source at all.
    e.server_mode = "config".into();
    assert!(e.validate(Path::new("/nonexistent")).is_err());
    // …satisfied by a modpack file instead of the single-mod id.
    e.modpack_json = "/tmp/pack.json".into();
    e.a2s_port = "17777".into();
    assert!(e.validate(Path::new("/nonexistent")).is_ok());
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
                TBD_PROFILE_DIR=/p/q\nTBD_A2S_PORT=9999\nTBD_SSH_PASS=\n";
    let process = [
        ("TBD_SSH_HOST", "deploy@198.51.100.7"),
        ("TBD_A2S_PORT", "1"),
        ("TBD_SSH_PASS", "exported"),
        ("TBD_MODPACK_JSON", "/tmp/pack.json"),
    ];
    // The runtime credential is required; without it the settings do not load.
    assert!(env_from(head, &process).is_err());
    let full = format!("{head}TBD_MOD_RUNTIME_CREDENTIAL={RUNTIME_CREDENTIAL}\n");
    let e = env_from(&full, &process).expect("loads");
    assert_eq!(e.deploy_host.ssh_destination(), "deploy@192.0.2.10");
    assert_eq!(e.a2s_port, "9999");
    assert_eq!(
        e.ssh_pass, None,
        "an empty assignment is not filled from the environment"
    );
    assert_eq!(
        e.modpack_json, "/tmp/pack.json",
        "a key the file never assigns"
    );
    assert_eq!(e.mod_runtime_credential, RUNTIME_CREDENTIAL);
    assert!(e.host_agent.is_none(), "the host agent is opt-in");
    // Defaults: the folders under the deploy user's home, and the `:=` default of a port.
    assert_eq!(e.remote_dir, "/home/deploy/tbd/repo");
    assert_eq!(e.addons_staging, "/home/deploy/tbd/addons-staging");
    assert_eq!(e.server_dir, "/home/deploy/steam/arma-reforger-server");
    assert_eq!(e.game_port, "2001");
    // dirname of TBD_PROFILE_DIR.
    assert_eq!(e.server_config_remote, "/p/server.config.json");
    // The scenario default is NOT truncated — the measured brace-expansion defect.
    assert_eq!(e.scenario, "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf");
    // A missing file is the documented rc-1 message, not a panic.
    assert!(Env::load(Path::new("/nonexistent/deploy.env")).is_err());
}

#[test]
fn host_agent_settings_are_required_once_the_install_is_asked_for() {
    let head = format!(
        "TBD_SSH_HOST=deploy@192.0.2.10\nTBD_MOD_RUNTIME_CREDENTIAL={RUNTIME_CREDENTIAL}\n\
         TBD_INSTALL_HOST_AGENT=1\n"
    );
    assert!(
        env_from(&head, &[]).is_err(),
        "the agent needs its own credential and the RCON password"
    );
    let e = env_from(
        &format!("{head}TBD_HOST_AGENT_CREDENTIAL=tbdm_x\nTBD_RCON_PASSWORD=secret\n"),
        &[],
    )
    .expect("loads");
    let agent = e.host_agent.as_ref().expect("the install is asked for");
    assert_eq!(agent.rcon_port, "19999");
    assert_eq!(
        agent.api_base_url, "http://127.0.0.1:8080",
        "defaults to TBD_BACKEND_URL"
    );
    // A malformed agent credential is refused by the validation, before any deploy step.
    assert!(e.validate(Path::new("/nonexistent")).is_err());
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
            "TBD_SSH_HOST=deploy@192.0.2.10\nTBD_MOD_RUNTIME_CREDENTIAL={RUNTIME_CREDENTIAL}\n\
             touch {}\n",
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
    let file =
        format!("TBD_SSH_HOST=192.0.2.10\nTBD_MOD_RUNTIME_CREDENTIAL={RUNTIME_CREDENTIAL}\n");
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
