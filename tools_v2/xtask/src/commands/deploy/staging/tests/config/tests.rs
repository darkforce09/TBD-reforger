use super::*;

/// Shared fixture: a fully-resolved `Env` with the documented defaults. `pub(super)` so
/// [`super::super::render`]'s tests use the SAME baseline — two drifting fixtures would let a
/// render test pass against inputs the loader can no longer produce.
pub(crate) fn base() -> Env {
    Env {
        ssh_host: "sam@192.168.0.140".into(),
        remote_dir: "/home/sam/tbd/repo".into(),
        profile_dir: "/home/sam/tbd/profile".into(),
        addons_staging: "/home/sam/tbd/addons".into(),
        game_server_token: "tok".into(),
        mission_id: "msn_8f3a2c".into(),
        event_id: "b0000000-0000-4000-8000-000000000001".into(),
        backend_url: "http://127.0.0.1:8080".into(),
        addon_guid: "B2C3D4E5F6A78901".into(),
        scenario: "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf".into(),
        bind_ip: "192.168.0.140".into(),
        server_dir: "/home/sam/steam/arma-reforger-server".into(),
        server_mode: "config".into(),
        workshop_mod_id: "5EAF00DBEEF01234".into(),
        public_address: "192.168.0.140".into(),
        game_port: "2001".into(),
        a2s_port: "17777".into(),
        server_name: "TBD Staging POC".into(),
        admin_password: "tbd-admin".into(),
        max_players: "64".into(),
        admin_identity_ids: String::new(),
        server_config_remote: "/home/sam/tbd/server.config.json".into(),
        boot_verify_timeout: "180".into(),
        modpack_json: String::new(),
        modpack_url: String::new(),
        modpack_token: String::new(),
        workshop_mod_name: "TBD_Framework".into(),
        run_t092_smoke: false,
        ssh_pass: None,
        ssh_identity_file: None,
    }
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
    e.remote_dir = "/home/sam/prairielearn/x".into();
    assert!(e.validate(Path::new("/nonexistent")).is_err());
    // ODDITY PRESERVED: the bash used a `*prairielearn*` glob, which is CASE SENSITIVE, so
    // `/home/sam/PrairieLearn/x` was allowed through. `gate_deploy_website.rs` case-folds for
    // its own script; this one does not, because the two scripts made different checks and
    // silently widening a refusal is still a behaviour change.
    e.remote_dir = "/home/sam/PrairieLearn/x".into();
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
    let d = std::env::temp_dir().join(format!("tbd-t853-env-{}", std::process::id()));
    let _ = fs::create_dir_all(&d);
    let f = d.join("deploy.env");
    fs::write(
        &f,
        "# comment\nexport TBD_SSH_HOST=\"h\"\nTBD_REMOTE_DIR=/home/sam/tbd/r\n\
         TBD_PROFILE_DIR=/p/q\nTBD_ADDONS_STAGING=/a\nTBD_GAME_SERVER_TOKEN=t\n\
         TBD_A2S_PORT=9999\n",
    )
    .unwrap();
    let e = Env::load(&f).expect("loads");
    assert_eq!(e.a2s_port, "9999");
    // The `:=` default for a var the file omits.
    assert_eq!(e.game_port, "2001");
    // dirname of TBD_PROFILE_DIR.
    assert_eq!(e.server_config_remote, "/p/server.config.json");
    // The scenario default is NOT truncated — the measured brace-expansion defect.
    assert_eq!(e.scenario, "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf");
    // A missing file is the documented rc-1 message, not a panic.
    assert!(Env::load(&d.join("absent.env")).is_err());
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn source_no_longer_executes_the_env_file() {
    // FAIL-OPEN CLOSED: a stray command in deploy.env used to RUN. Now it is inert text.
    let d = std::env::temp_dir().join(format!("tbd-t853-src-{}", std::process::id()));
    let _ = fs::create_dir_all(&d);
    let canary = d.join("canary");
    let f = d.join("deploy.env");
    fs::write(
        &f,
        format!(
            "TBD_SSH_HOST=h\nTBD_REMOTE_DIR=/r\nTBD_PROFILE_DIR=/p\nTBD_ADDONS_STAGING=/a\n\
             TBD_GAME_SERVER_TOKEN=t\ntouch {}\n",
            canary.display()
        ),
    )
    .unwrap();
    let _ = Env::load(&f).expect("loads");
    assert!(!canary.exists(), "deploy.env must never be executed");
    let _ = fs::remove_dir_all(&d);
}
