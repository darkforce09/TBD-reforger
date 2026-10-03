use super::*;
use crate::staging::config::tests::base;

#[test]
fn legacy_mods_render_matches_the_captured_bytes() {
    // Diffed against a captured render: two-space JSON indent then every
    // line after the first gets four extra spaces, so `[` sits flush after `"mods": `.
    let e = base();
    let mut mod0 = Map::new();
    mod0.insert("name".into(), Value::String(e.workshop_mod_name.clone()));
    mod0.insert(
        "workshop_id".into(),
        Value::String(e.workshop_mod_id.to_string()),
    );
    mod0.insert("version".into(), Value::String(String::new()));
    let mut d = Map::new();
    d.insert("mods".into(), Value::Array(vec![Value::Object(mod0)]));
    let got = modpack_mods_json(&Value::Object(d).to_string(), "L").expect("renders");
    assert_eq!(
        got,
        "[\n      {\n        \"modId\": \"5EAF00DBEEF01234\",\n        \"name\": \"TBD_Framework\"\n      }\n    ]"
    );
}

#[test]
fn version_is_emitted_only_when_non_empty_and_key_order_is_pinned() {
    let doc = r#"{"mods":[{"name":"A","workshop_id":"W","version":"1.0.2"}]}"#;
    let got = modpack_mods_json(doc, "t").expect("renders");
    // modId, name, version — the python dict's insertion order, not alphabetical.
    let idx_id = got.find("modId").unwrap();
    let idx_name = got.find("\"name\"").unwrap();
    let idx_ver = got.find("version").unwrap();
    assert!(idx_id < idx_name && idx_name < idx_ver, "{got}");
    // An empty version is omitted entirely rather than rendered as "".
    let doc = r#"{"mods":[{"name":"A","workshop_id":"W","version":""}]}"#;
    assert!(!modpack_mods_json(doc, "t").unwrap().contains("version"));
}

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

#[test]
fn curl_argv_is_stable_and_the_token_travels_on_stdin() {
    // NEVER EXECUTED: no credential of this tier exists anywhere in this program. The argv is
    // the only thing that can be asserted, so it is asserted exactly.
    let mut e = base();
    e.modpack_url = "https://tbd.example/api/v1/modpacks/current".into();
    e.modpack_token = "jwt.abc.def".into();
    let argv = curl_argv(&e, Path::new("/tmp/out.json"));
    assert_eq!(
        argv,
        vec![
            "-sS",
            "-o",
            "/tmp/out.json",
            "-w",
            "%{http_code}",
            "-H",
            "@-",
            "https://tbd.example/api/v1/modpacks/current",
        ]
    );
    assert!(argv.iter().all(|a| !a.contains("jwt.abc.def")));
    assert_eq!(curl_header_stdin(&e), "Authorization: Bearer jwt.abc.def\n");
}

#[test]
fn modpack_url_without_a_token_fails_before_any_network_call() {
    // The credential tier: the deploy's machine credentials do not authenticate an AuthUser
    // route, so an empty TBD_MODPACK_TOKEN must fail closed here rather than produce a 401
    // nobody reads.
    let mut e = base();
    e.modpack_url = "https://tbd.example/x".into();
    assert!(resolve_modpack_doc(&e).is_err());
}
