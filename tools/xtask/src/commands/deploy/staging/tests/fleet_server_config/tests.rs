//! Each instance's server config: a render for the five instances, the rcon block, the password
//! placeholders, visibility, and the checks every rendered file passes.
use super::*;
use crate::commands::deploy::staging::config::tests::base;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tbd-fleet-config-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn read(path: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// `--render-only <directory>` writes one checked config per instance, and each carries its own
/// ports, name and visibility, the admin RCON block on loopback, and no secret.
#[test]
fn render_only_writes_every_instance_of_the_five() {
    let dir = scratch("render-only");
    let env = base();
    assert_eq!(render_only(&env, dir.to_str().unwrap()), 0);
    for instance in env.fleet.instances() {
        let n = instance.number;
        let path = dir.join(format!("instance-{n}/server.config.json"));
        let config = read(&path);
        assert_eq!(config["bindPort"], 2000 + n, "instance {n}");
        assert_eq!(config["publicPort"], 2000 + n);
        assert_eq!(config["publicAddress"], "192.0.2.10");
        assert_eq!(config["a2s"]["port"], 17776 + n);
        assert_eq!(
            config["rcon"],
            serde_json::json!({ "address": "127.0.0.1", "port": 19998 + n,
                                "password": RCON_PASSWORD_PLACEHOLDER,
                                "permission": "admin", "maxClients": 2 })
        );
        assert_eq!(config["game"]["name"], format!("TBD Staging {n}"));
        assert_eq!(config["game"]["password"], JOIN_PASSWORD_PLACEHOLDER);
        assert_eq!(
            config["game"]["visible"],
            n == 1,
            "only instance 1 is listed"
        );
        assert_eq!(
            config["game"]["scenarioId"],
            "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"
        );
        assert_eq!(config["game"]["mods"][0]["modId"], "5EAF00DBEEF01234");
    }
    assert!(!dir.join("instance-6").exists());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_smaller_fleet_renders_only_its_instances() {
    let dir = scratch("three");
    let mut env = base();
    env.fleet.instance_count = 3;
    env.fleet.relay = None;
    assert_eq!(render_only(&env, dir.to_str().unwrap()), 0);
    assert!(dir.join("instance-3/server.config.json").is_file());
    assert!(!dir.join("instance-4").exists());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_rendered_scenario_is_the_one_passed_in() {
    let dir = scratch("scenario");
    let env = base();
    let second = &env.fleet.instances()[1];
    let mods = fleet_mods_json(&env).unwrap();
    let out = dir.join("second.json");
    render_instance_server_config(
        &env,
        &mods,
        second,
        "{0123456789ABCDEF}Missions/Deployed.conf",
        &out,
    )
    .unwrap();
    assert_eq!(
        read(&out)["game"]["scenarioId"],
        "{0123456789ABCDEF}Missions/Deployed.conf"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_checks_catch_the_truncated_scenario_and_a_raw_quote() {
    // The two cases a format-only validator is BLIND to, and the honest config beside them.
    let dir = scratch("checks");
    let env = base();
    let first = &env.fleet.instances()[0];
    let mods = fleet_mods_json(&env).unwrap();
    assert!(
        render_instance_server_config(&env, &mods, first, "{69A85365FC09E2CA", &dir.join("t.json"))
            .is_err(),
        "truncated scenario must fail"
    );
    let mut quoted = base();
    quoted.admin_password = "a\" , \"evil\": 1, \"x\": \"b".into();
    let res =
        render_instance_server_config(&quoted, &mods, first, &env.scenario, &dir.join("q.json"));
    let text = fs::read_to_string(dir.join("q.json")).unwrap_or_default();
    assert!(
        res.is_err() || text.contains("evil"),
        "raw substitution must be observable"
    );
    assert!(
        render_instance_server_config(&env, &mods, first, &env.scenario, &dir.join("ok.json"))
            .is_ok()
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// The rendered text names both placeholders and neither kind of password, so a render leaves the
/// development machine with nothing secret in it.
#[test]
fn a_render_carries_placeholders_and_no_password() {
    let block = rcon_block(&base().fleet.instances()[4]);
    assert_eq!(
        block,
        "\"rcon\": { \"address\": \"127.0.0.1\", \"port\": 20003, \"password\": \
         \"TBD_RCON_PASSWORD_FROM_HOST_FILE\", \"permission\": \"admin\", \"maxClients\": 2 },"
    );
    assert!(RCON_PASSWORD_PLACEHOLDER.ends_with("_FROM_HOST_FILE"));
    assert!(JOIN_PASSWORD_PLACEHOLDER.ends_with("_FROM_HOST_FILE"));
}
