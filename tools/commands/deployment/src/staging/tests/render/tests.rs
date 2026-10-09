use super::*;

#[test]
fn every_fail_closed_branch_fires() {
    // ANTI-VACUITY: each of these is a real /tmp/t853 baseline, and each must be RED.
    for bad in [
        "not json at all {",
        "[1,2,3]",
        r#"{"name":"x"}"#,
        r#"{"name":"x","mods":null}"#,
        r#"{"name":"x","mods":{}}"#,
        r#"{"name":"x","mods":[]}"#,
        r#"{"name":"x","mods":[1]}"#,
        r#"{"name":"x","mods":[{"name":"","workshop_id":"a"}]}"#,
        r#"{"name":"x","mods":[{"name":"A","workshop_id":""}]}"#,
        r#"{"name":"x","mods":[{"name":"A","workshop_id":"Z"},{"name":"B","workshop_id":"Z"}]}"#,
    ] {
        assert!(modpack_mods_json(bad, "t").is_err(), "should fail: {bad}");
    }
    // …and the good one must be GREEN, or the ten above prove nothing.
    assert!(modpack_mods_json(r#"{"mods":[{"name":"A","workshop_id":"W"}]}"#, "t").is_ok());
}

/// The check re-reads the file: a document that is not JSON, a truncated scenario and a shared
/// A2S and game port are each refused, and the honest document beside them passes.
#[test]
fn the_config_check_refuses_what_the_engine_refuses() {
    let d = std::env::temp_dir().join(format!("tbd-config-check-{}", std::process::id()));
    let _ = fs::create_dir_all(&d);
    let write = |name: &str, text: &str| {
        let path = d.join(name);
        fs::write(&path, text).unwrap();
        path
    };
    let honest = r#"{"bindAddress": "0.0.0.0", "bindPort": 2001, "publicAddress": "192.0.2.10",
        "publicPort": 2001, "a2s": {"address": "0.0.0.0", "port": 17777},
        "game": {"name": "TBD Staging 1", "passwordAdmin": "a", "admins": [],
                 "scenarioId": "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf", "maxPlayers": 64,
                 "mods": [{"modId": "5EAF00DBEEF01234", "name": "TBD_Framework"}]},
        "operating": {}}"#;
    assert!(validate_server_config(&write("ok.json", honest)).is_ok());
    let truncated = honest.replace("}Missions/TBD_Dev_POC.conf", "");
    assert!(validate_server_config(&write("trunc.json", &truncated)).is_err());
    let clash = honest.replace("\"port\": 17777", "\"port\": 2001");
    assert!(validate_server_config(&write("clash.json", &clash)).is_err());
    assert!(validate_server_config(&write("raw.json", "{\"bindPort\": not-a-port}")).is_err());
    let _ = fs::remove_dir_all(&d);
}
