use super::*;
use crate::staging::fleet_instances::RelaySettings;

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
        workshop_mod_id: WorkshopModId::new("5EAF00DBEEF01234"),
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
