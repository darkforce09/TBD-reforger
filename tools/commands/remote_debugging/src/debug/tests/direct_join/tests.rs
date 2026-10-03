use super::*;
use std::process::Command;

fn fixture_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("direct-join-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::write(root.join(".ai/tickets/ROOT"), "{}").unwrap();
    fs::create_dir_all(root.join("apps/mod")).unwrap();
    fs::create_dir_all(root.join(repository_layout::DEPLOY_DIR)).unwrap();
    root
}

/// Deploy settings with no file and no process variables: no staging host.
fn no_settings(root: &Path) -> DeployEnvironment {
    DeployEnvironment::from_text(&root.join(repository_layout::DEPLOY_ENV), None, [])
        .expect("an absent file always loads")
}

fn empty_home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("direct-join-home-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(home.join(".local/bin")).unwrap();
    home
}

#[test]
fn arm_nocursor_exits_err() {
    let _g = tool_test_support::lock_env();
    let root = fixture_root("nocursor");
    // no .cursor/
    let home = empty_home("nocursor");
    let err = run_with(
        &root,
        &home,
        &no_settings(&root),
        &RunId::from("arm-nocursor"),
        None,
    )
    .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("open") && msg.contains("debug-8fc1e0.log"),
        "bash-red arm: missing .cursor must fail open: {msg}"
    );
}

#[test]
fn arm_logdir_exits_err() {
    let _g = tool_test_support::lock_env();
    let root = fixture_root("logdir");
    fs::create_dir_all(root.join(".cursor/debug-8fc1e0.log")).unwrap();
    let home = empty_home("logdir");
    let err = run_with(
        &root,
        &home,
        &no_settings(&root),
        &RunId::from("arm-logdir"),
        None,
    )
    .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("open") && msg.contains("debug-8fc1e0.log"),
        "bash-red arm: log-as-dir must fail open: {msg}"
    );
}

#[test]
fn clean_empty_home_writes_unknown_and_missing() {
    let _g = tool_test_support::lock_env();
    let root = fixture_root("clean");
    fs::create_dir_all(root.join(".cursor")).unwrap();
    let home = empty_home("clean");
    let code = run_with(
        &root,
        &home,
        &no_settings(&root),
        &RunId::from("clean-baseline"),
        None,
    )
    .unwrap();
    assert_eq!(code, 0);
    let log = fs::read_to_string(root.join(".cursor/debug-8fc1e0.log")).unwrap();
    assert!(log.contains("\"client_build\":\"unknown\""));
    assert!(log.contains("\"server_build\":\"unknown\""));
    assert!(log.contains("\"path\":\"missing\""));
    // No host: the probes that need one record `skipped` and reach no network.
    assert!(log.contains("\"raw\":\"skipped\""), "{log}");
    assert!(log.contains("\"ms\":\"skipped\""), "{log}");
    assert!(
        log.contains("\"skipped\":\"TBD_SSH_HOST is not set"),
        "{log}"
    );
}

#[test]
fn the_remote_probe_quotes_the_profile_folder() {
    let script = remote_probe_script(
        SINGLE_SERVER_UNIT,
        &single_quoted("/home/deploy/tbd/it's profile"),
        SINGLE_SERVER_PORTS,
        true,
    );
    assert!(
        script.contains("LOG=$(ls -td '/home/deploy/tbd/it'\\''s profile'/logs/logs_* "),
        "{script}"
    );
}

/// Settings naming a staging host, with the fleet settings given.
fn host_settings(root: &Path, fleet: &str) -> DeployEnvironment {
    DeployEnvironment::from_text(
        &root.join(repository_layout::DEPLOY_ENV),
        Some(&format!("TBD_SSH_HOST=deploy@staging.invalid\n{fleet}")),
        [],
    )
    .expect("the settings parse")
}

#[test]
fn direct_join_instance_probes_its_own_unit_ports_and_profile() {
    let root = fixture_root("instance-target");
    let environment = host_settings(&root, "");
    let third = select_fleet_instance(&environment, 3).expect("in range");
    let host = staging_host(&environment, Some(&third)).expect("a host");
    assert_eq!(host.ports, [2003, 17779]);
    let script = host.remote_script.expect("a script");
    assert!(
        script.contains("is-active tbd-reforger@3.service"),
        "{script}"
    );
    assert!(script.contains("grep -c ':2003 '") && script.contains("grep -c ':17779 '"));
    assert!(script.contains("echo \"service=$SVC udp2003=$PG udp17779=$PA\""));
    assert!(script.contains("ls -td \"$HOME\"/'tbd/fleet/instance-3/profile'/logs/logs_* "));
    assert!(!script.contains("fleet_host"), "{script}");

    let single = staging_host(&environment, None).expect("a host");
    assert_eq!(single.ports, [2001, 17777]);
    let script = single.remote_script.expect("a script");
    assert!(
        script.contains("is-active tbd-reforger.service"),
        "{script}"
    );
    assert!(script.contains("udp2001=$PG udp17777=$PA"), "{script}");
}

/// The data of the first row of `log`, which is H1's.
fn h1_data(log: &Path) -> serde_json::Value {
    let text = fs::read_to_string(log).unwrap();
    let row: serde_json::Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(row["hypothesisId"], "H1", "{text}");
    row["data"].clone()
}

/// H1 keys its listener counts by the probed server's own ports: instance 3's listeners read
/// true under `udp_2003` and `udp_17779`, the single server's keys stay `udp_2001` and
/// `udp_17777`, and rows whose settings named no server carry no `udp_` key.
#[test]
fn direct_join_listener_row_follows_the_probed_server() {
    let root = fixture_root("listener-row");
    let log = root.join("rows.log");
    let write = |listener_ports: Option<[u16; 2]>| {
        let _ = fs::remove_file(&log);
        let observed = DirectJoinObservations {
            run_id: &RunId::from("r"),
            remote: "service=active udp2003=2 udp17779=1\nlisten=none",
            listener_ports,
            client_build: "1",
            server_build: "1",
            symlink: "missing",
            ping_ms: "fail",
            ping_host: "",
            ping_address: "",
            a2s_json: "{}",
        };
        probes::cmd_direct_join_log(&log, &observed).unwrap();
        h1_data(&log)
    };
    let third = select_fleet_instance(&host_settings(&root, ""), 3).expect("in range");
    let instance = write(Some(probed_ports(Some(&third))));
    assert_eq!(instance["udp_2003"], true, "{instance}");
    assert_eq!(instance["udp_17779"], true, "{instance}");
    assert!(instance.get("udp_2001").is_none(), "{instance}");
    let single = write(Some(probed_ports(None)));
    assert_eq!(single["udp_2001"], false, "{single}");
    assert_eq!(single["udp_17777"], false, "{single}");
    let unnamed = write(None);
    assert_eq!(unnamed["service_active"], true, "{unnamed}");
    assert!(
        unnamed
            .as_object()
            .unwrap()
            .keys()
            .all(|key| !key.starts_with("udp_")),
        "{unnamed}"
    );
}

/// With no staging host, `--instance 3` still keys H1 by instance 3's ports, and fleet settings
/// the deploy refuses skip the host's probes and name no port.
#[test]
fn direct_join_instance_run_without_a_host_keys_h1_by_its_ports() {
    let _g = tool_test_support::lock_env();
    let root = fixture_root("instance-no-host");
    fs::create_dir_all(root.join(".cursor")).unwrap();
    let home = empty_home("instance-no-host");
    let log = root.join(".cursor/debug-8fc1e0.log");
    assert_eq!(
        run_with(
            &root,
            &home,
            &no_settings(&root),
            &RunId::from("r"),
            Some(3)
        )
        .unwrap(),
        0
    );
    let data = h1_data(&log);
    assert_eq!(data["raw"], "skipped", "{data}");
    assert_eq!(data["udp_2003"], false, "{data}");
    assert_eq!(data["udp_17779"], false, "{data}");
    assert!(data.get("udp_2001").is_none(), "{data}");

    fs::remove_file(&log).unwrap();
    let colliding = DeployEnvironment::from_text(
        &root.join(repository_layout::DEPLOY_ENV),
        Some("TBD_FLEET_GAME_PORT_BASE=17776\n"),
        [],
    )
    .expect("the settings parse");
    assert_eq!(
        run_with(&root, &home, &colliding, &RunId::from("r"), Some(1)).unwrap(),
        0
    );
    let data = h1_data(&log);
    assert_eq!(data["raw"], "skipped", "{data}");
    assert!(
        data.as_object()
            .unwrap()
            .keys()
            .all(|key| !key.starts_with("udp_")),
        "{data}"
    );
}

#[test]
fn direct_join_single_server_probe_names_instance_on_a_fleet_host() {
    let script = remote_probe_script("tbd-reforger.service", "'/nowhere'", [2001, 17777], true);
    let home = empty_home("fleet-host");
    let run = |home: &Path| {
        Command::new("bash")
            .arg("-c")
            .arg(&script)
            .env("HOME", home)
            .output()
            .expect("bash runs")
    };
    let single = run(&home);
    assert!(!String::from_utf8_lossy(&single.stdout).contains("fleet_host"));
    fs::create_dir_all(home.join("tbd/fleet")).unwrap();
    let fleet = run(&home);
    let text = String::from_utf8_lossy(&fleet.stdout);
    assert!(text.starts_with("service=fleet_host\nrefused="), "{text}");
    assert!(text.contains("pass --instance N (1 to 5)"), "{text}");
}

#[test]
fn direct_join_instance_outside_the_fleet_exits_1_before_probing() {
    let _g = tool_test_support::lock_env();
    let root = fixture_root("instance-refused");
    fs::create_dir_all(root.join(".cursor")).unwrap();
    let home = empty_home("instance-refused");
    let environment = host_settings(&root, "TBD_FLEET_INSTANCES=2\n");
    assert_eq!(
        run_with(&root, &home, &environment, &RunId::from("r"), Some(3)).unwrap(),
        1
    );
    assert_eq!(
        run_with(&root, &home, &environment, &RunId::from("r"), Some(0)).unwrap(),
        1
    );
    assert!(!root.join(".cursor/debug-8fc1e0.log").exists());
}

#[test]
fn steam_two_field_buildid_is_empty_not_unknown() {
    let home = empty_home("steam2");
    let apps = home.join(".local/share/Steam/steamapps");
    fs::create_dir_all(&apps).unwrap();
    // Typical Steam line — awk `$3` empty (preserved oddity).
    fs::write(
        apps.join("appmanifest_1874880.acf"),
        "\t\"buildid\"\t\t\"999\"\n",
    )
    .unwrap();
    assert_eq!(steam_build_id(&home, "1874880"), "");
    assert_eq!(steam_build_id(&home, "1874900"), "unknown");
}

#[test]
fn steam_three_field_buildid_uses_awk_dollar3() {
    let home = empty_home("steam3");
    let apps = home.join(".local/share/Steam/steamapps");
    fs::create_dir_all(&apps).unwrap();
    fs::write(
        apps.join("appmanifest_1874880.acf"),
        "\t\"buildid\"\t\t\"x\"\t\t\"111\"\n",
    )
    .unwrap();
    assert_eq!(steam_build_id(&home, "1874880"), "111");
}
