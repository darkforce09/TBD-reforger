use super::*;

#[test]
fn admins_json_uses_pythons_comma_space_separator() {
    // MEASURED: python3 -c 'import json; print(json.dumps(["a","b"]))' -> ["a", "b"]
    assert_eq!(admins_json(&["a".into(), "b".into()]), "[\"a\", \"b\"]");
    assert_eq!(admins_json(&[]), "[]");
    assert_eq!(admins_json(&["only".into()]), "[\"only\"]");
}

#[test]
fn empty_admins_are_dropped_by_the_line_pipeline() {
    // `printf '%s\n' … | split("\n") if l` — the empty string never becomes an entry.
    assert_eq!(admins_json(&["".into()]), "[]");
    assert_eq!(admins_json(&["a".into(), "".into()]), "[\"a\"]");
}

#[test]
fn newline_in_an_admin_splits_it_in_two() {
    // THE LATENT BUG, pinned. See `admins_json`'s docs: this value passes `admin_id_is_valid`
    // (grep anchors per line) and then arrives at the engine as TWO ids, one of them junk.
    assert_eq!(
        admins_json(&["junk\n00000000-0000-0000-0000-000000000000".into()]),
        "[\"junk\", \"00000000-0000-0000-0000-000000000000\"]"
    );
}

#[test]
fn ensure_ascii_matches_cpython() {
    // MEASURED: json.dumps({"n":"café ü"}, indent=2) -> "café ü"
    assert_eq!(ensure_ascii("café ü"), "caf\\u00e9 \\u00fc");
    // Astral: python emits a surrogate pair. U+1F600 -> 😀
    assert_eq!(ensure_ascii("\u{1F600}"), "\\ud83d\\ude00");
    assert_eq!(ensure_ascii("plain"), "plain");
}

#[test]
fn int_like_reproduces_the_python_valueerror_text() {
    assert_eq!(int_like("2001").unwrap(), Value::Number(2001.into()));
    // Byte-identical to CPython's message, which is what the operator sees and what the
    // captured bash baseline carries (`/tmp/t853/w-play/out/bash/g01-port-not-a-number.txt`).
    assert_eq!(
        int_like("abc").unwrap_err(),
        "ValueError: invalid literal for int() with base 10: 'abc'"
    );
    assert!(int_like("").is_err());
}

#[test]
fn render_reproduces_the_measured_cpython_key_order() {
    // The whole point: `mods` exists in the dev config so it is replaced IN PLACE, `admins` does
    // not so it is APPENDED — after `mods`. Measured against the real python render.
    let dir = std::env::temp_dir().join(format!("tbd-rps-render-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("dev.json");
    let dst = dir.join("server.json");
    std::fs::write(
        &src,
        r#"{
  "bindAddress": "0.0.0.0",
  "bindPort": 2001,
  "publicAddress": "127.0.0.1",
  "publicPort": 2001,
  "a2s": {
    "address": "0.0.0.0",
    "port": 17777
  },
  "game": {
    "name": "TBD Dev POC",
    "scenarioId": "{OLD}Missions/Old.conf",
    "maxPlayers": 8,
    "visible": false,
    "mods": []
  },
  "operating": {
    "disableNavmeshStreaming": []
  }
}
"#,
    )
    .unwrap();
    render_server_json(&ServerJson {
        src: &src,
        dst: &dst,
        ip: "10.0.0.1",
        port: "2011",
        a2s: "17787",
        max_players: "32",
        guid: "B2C3D4E5F6A78901",
        scenario: "{NEW}Missions/New.conf",
        name: "Café über",
        admins: &["76561198000000000".into()],
    })
    .unwrap();
    let got = std::fs::read_to_string(&dst).unwrap();
    assert_eq!(
        got,
        r#"{
  "bindAddress": "0.0.0.0",
  "bindPort": 2011,
  "publicAddress": "10.0.0.1",
  "publicPort": 2011,
  "a2s": {
    "address": "0.0.0.0",
    "port": 17787
  },
  "game": {
    "name": "Caf\u00e9 \u00fcber",
    "scenarioId": "{NEW}Missions/New.conf",
    "maxPlayers": 32,
    "visible": true,
    "mods": [
      {
        "modId": "B2C3D4E5F6A78901",
        "name": "TBD_Framework"
      }
    ],
    "admins": [
      "76561198000000000"
    ]
  },
  "operating": {
    "disableNavmeshStreaming": []
  }
}"#,
        "key order, ensure_ascii, empty-array form or the missing trailing newline drifted"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a2s_and_game_are_created_when_absent_and_land_at_the_end() {
    let dir = std::env::temp_dir().join(format!("tbd-rps-sd-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("bare.json");
    let dst = dir.join("out.json");
    std::fs::write(&src, "{\"keep\": 1}").unwrap();
    render_server_json(&ServerJson {
        src: &src,
        dst: &dst,
        ip: "1.2.3.4",
        port: "1",
        a2s: "2",
        max_players: "3",
        guid: "G",
        scenario: "S",
        name: "N",
        admins: &[],
    })
    .unwrap();
    let got = std::fs::read_to_string(&dst).unwrap();
    // setdefault appends, so `keep` stays first and `a2s` precedes `game`.
    let keep = got.find("\"keep\"").unwrap();
    let a2s = got.find("\"a2s\"").unwrap();
    let game = got.find("\"game\"").unwrap();
    assert!(keep < a2s && a2s < game, "{got}");
    assert!(
        got.contains("\"admins\": []"),
        "empty admins must be `[]`: {got}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_non_object_a2s_is_an_error_not_a_silent_overwrite() {
    // CPython raised TypeError here; the port must not quietly replace the operator's value.
    let dir = std::env::temp_dir().join(format!("tbd-rps-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("bad.json");
    std::fs::write(&src, "{\"a2s\": \"oops\"}").unwrap();
    let got = render_server_json(&ServerJson {
        src: &src,
        dst: &dir.join("out.json"),
        ip: "1.2.3.4",
        port: "1",
        a2s: "2",
        max_players: "3",
        guid: "G",
        scenario: "S",
        name: "N",
        admins: &[],
    });
    assert!(got.is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn backend_config_patch_sets_event_id_even_when_empty_and_keeps_the_token() {
    let dir = std::env::temp_dir().join(format!("tbd-rps-be-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("TBD_BackendConfig.json");
    std::fs::write(
        &p,
        "{\n  \"backendUrl\": \"http://old\",\n  \"serverToken\": \"from-dotenv\"\n}",
    )
    .unwrap();
    let mut o = crate::commands::mod_ops::playtest_server::Opts::defaults("/home/u");
    o.mission_id = "msn_1".into();
    patch_backend_config(p.to_str().unwrap(), &o).unwrap();
    let got = std::fs::read_to_string(&p).unwrap();
    assert!(got.contains("\"missionId\": \"msn_1\""));
    assert!(
        got.contains("\"eventId\": \"\""),
        "eventId is always written"
    );
    assert!(
        got.contains("\"serverToken\": \"from-dotenv\""),
        "an empty --token must leave setup server-profile's substitution alone: {got}"
    );
    // ...and an explicit token replaces it.
    o.token = "explicit".into();
    patch_backend_config(p.to_str().unwrap(), &o).unwrap();
    assert!(
        std::fs::read_to_string(&p)
            .unwrap()
            .contains("\"serverToken\": \"explicit\"")
    );
    let _ = std::fs::remove_dir_all(&dir);
}
