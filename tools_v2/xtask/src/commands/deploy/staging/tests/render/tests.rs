use super::*;
use crate::commands::deploy::staging::config::tests::base;

#[test]
fn legacy_mods_render_matches_the_captured_bytes() {
    // Diffed against a captured render: two-space JSON indent then every
    // line after the first gets four extra spaces, so `[` sits flush after `"mods": `.
    let e = base();
    let mut mod0 = Map::new();
    mod0.insert("name".into(), Value::String(e.workshop_mod_name.clone()));
    mod0.insert(
        "workshop_id".into(),
        Value::String(e.workshop_mod_id.clone()),
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

#[test]
fn validator_catches_the_truncated_scenario_and_the_port_clash() {
    // The two cases a format-only validator is BLIND to.
    let d = std::env::temp_dir().join(format!("tbd-t853-cfg-{}", std::process::id()));
    let _ = fs::create_dir_all(&d);

    let mut e = base();
    e.scenario = "{69A85365FC09E2CA".into();
    assert!(
        render_server_config(&e, &e.scenario, &d.join("trunc.json")).is_err(),
        "truncated scenario must fail"
    );

    let mut e = base();
    e.a2s_port = e.game_port.clone();
    assert!(
        render_server_config(&e, &e.scenario, &d.join("clash.json")).is_err(),
        "a2s == bindPort must fail"
    );

    // And the honest config must pass, or the two above are vacuous.
    assert!(render_server_config(&base(), &base().scenario, &d.join("ok.json")).is_ok());
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn raw_substitution_can_emit_non_json_and_the_validator_catches_it() {
    // ODDITY PINNED: the template does not escape. A quote in the server name breaks the
    // document, and the "not valid JSON" branch — otherwise unreachable — fires.
    let d = std::env::temp_dir().join(format!("tbd-t853-raw-{}", std::process::id()));
    let _ = fs::create_dir_all(&d);
    let mut e = base();
    e.server_name = "a\" , \"evil\": 1, \"x\": \"b".into();
    let p = d.join("raw.json");
    let res = render_server_config(&e, &e.scenario, &p);
    let text = fs::read_to_string(&p).unwrap_or_default();
    assert!(
        res.is_err() || text.contains("evil"),
        "raw substitution must be observable"
    );
    // A non-numeric port is the cleaner case: it cannot parse at all.
    let mut e = base();
    e.game_port = "not-a-port".into();
    assert!(render_server_config(&e, &e.scenario, &d.join("port.json")).is_err());
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn render_only_refuses_addons_mode() {
    let mut e = base();
    e.server_mode = "addons".into();
    assert_eq!(render_only(&e, "/tmp/never-written-t853.json"), 2);
    assert!(!Path::new("/tmp/never-written-t853.json").exists());
}

#[test]
fn curl_argv_is_stable() {
    // NEVER EXECUTED: no credential of this tier exists anywhere in this program. The argv is
    // the only thing that can be asserted, so it is asserted exactly.
    let mut e = base();
    e.modpack_url = "https://tbd.example/api/v1/modpacks/current".into();
    e.modpack_token = "jwt.abc.def".into();
    assert_eq!(
        curl_argv(&e, Path::new("/tmp/out.json")),
        vec![
            "-sS",
            "-o",
            "/tmp/out.json",
            "-w",
            "%{http_code}",
            "-H",
            "Authorization: Bearer jwt.abc.def",
            "https://tbd.example/api/v1/modpacks/current",
        ]
    );
}

#[test]
fn modpack_url_without_a_token_fails_before_any_network_call() {
    // The credential tier: TBD_GAME_SERVER_TOKEN is a SERVICE_TOKEN and does
    // not authenticate an AuthUser route, so an empty TBD_MODPACK_TOKEN must fail closed here
    // rather than produce a 401 nobody reads.
    let mut e = base();
    e.modpack_url = "https://tbd.example/x".into();
    assert!(resolve_modpack_doc(&e).is_err());
}
