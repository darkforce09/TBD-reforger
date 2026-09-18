use super::*;
use std::sync::Mutex;

/// Serialise HOME mutations across tests in this process.
static HOME_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn missing_config_exits_1() {
    let _g = HOME_LOCK.lock().unwrap();
    let home = tempfile_dir("nocfg");
    let root = throwaway_root("nocfg-root", true);
    unsafe { std::env::set_var("HOME", &home) };
    let code = run_with_root(&root, None).unwrap();
    assert_eq!(code, 1);
}

#[test]
fn unknown_golden_exits_1() {
    let _g = HOME_LOCK.lock().unwrap();
    let (home, _prof, root) = primed_home("nogolden");
    unsafe { std::env::set_var("HOME", &home) };
    let code = run_with_root(&root, Some("does-not-exist-xyz")).unwrap();
    assert_eq!(code, 1);
}

#[test]
fn backend_and_stage_round_trip() {
    let _g = HOME_LOCK.lock().unwrap();
    let (home, prof, root) = primed_home("roundtrip");
    unsafe { std::env::set_var("HOME", &home) };

    let code = run_with_root(&root, Some("bridgehead-at-levie")).unwrap();
    assert_eq!(code, 0);
    let cfg: Value =
        serde_json::from_str(&fs::read_to_string(prof.join("TBD_BackendConfig.json")).unwrap())
            .unwrap();
    assert_eq!(cfg["missionId"], "msn_8f3a2c");
    assert!(prof.join("missions/msn_8f3a2c.json").is_file());
    assert!(prof.join("TBD_Registry.json").is_file());

    let code = run_with_root(&root, Some("backend")).unwrap();
    assert_eq!(code, 0);
    let cfg: Value =
        serde_json::from_str(&fs::read_to_string(prof.join("TBD_BackendConfig.json")).unwrap())
            .unwrap();
    assert_eq!(cfg["missionId"], BACKEND_MISSION);
}

#[test]
fn set_mission_id_no_trailing_newline() {
    let dir = tempfile_dir("json-nl");
    let cfg = dir.join("TBD_BackendConfig.json");
    fs::write(
        &cfg,
        r#"{"backendUrl":"http://x","serverToken":"t","missionId":"old","eventId":"e"}"#,
    )
    .unwrap();
    set_mission_id(&cfg, "msn_x").unwrap();
    let body = fs::read_to_string(&cfg).unwrap();
    assert!(
        !body.ends_with('\n'),
        "python json.dump has no trailing newline"
    );
    assert!(body.contains("\"missionId\": \"msn_x\""));
}

fn tempfile_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("t864-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn throwaway_root(tag: &str, with_golden: bool) -> PathBuf {
    let root = tempfile_dir(tag);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::write(root.join(".ai/tickets/ROOT"), "{}").unwrap();
    fs::create_dir_all(root.join("apps/mod/tbd-framework/Data")).unwrap();
    fs::create_dir_all(developer_tools::repository_layout::mission_fixtures_valid_dir(&root))
        .unwrap();
    fs::write(
        root.join("apps/mod/tbd-framework/Data/registry.json"),
        "{\"ok\":true}\n",
    )
    .unwrap();
    if with_golden {
        fs::write(
            developer_tools::repository_layout::mission_fixtures_valid_dir(&root)
                .join("bridgehead-at-levie.json"),
            r#"{"meta":{"id":"msn_8f3a2c"},"slots":[{"faction":"blufor"},{"faction":"blufor"},{"faction":"opfor"}]}"#,
        )
        .unwrap();
    }
    root
}

fn primed_home(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let home = tempfile_dir(&format!("{tag}-home"));
    let root = throwaway_root(&format!("{tag}-root"), true);
    let prof = home.join(CFG_REL);
    fs::create_dir_all(prof.join("missions")).unwrap();
    fs::write(
        prof.join("TBD_BackendConfig.json"),
        format!(
            "{{\n  \"backendUrl\": \"http://127.0.0.1:8080\",\n  \"serverToken\": \"tok\",\n  \"missionId\": \"{BACKEND_MISSION}\",\n  \"eventId\": \"b0000000-0000-4000-8000-000000000001\"\n}}"
        ),
    )
    .unwrap();
    (home, prof, root)
}
