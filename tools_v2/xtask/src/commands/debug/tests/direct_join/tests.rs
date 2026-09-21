use super::*;

fn fixture_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("direct-join-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::write(root.join(".ai/tickets/ROOT"), "{}").unwrap();
    fs::create_dir_all(root.join("apps/mod")).unwrap();
    fs::create_dir_all(root.join(crate::core::repository_layout::DEPLOY_DIR)).unwrap();
    root
}

fn empty_home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("direct-join-home-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(home.join(".local/bin")).unwrap();
    home
}

#[test]
fn arm_nocursor_exits_err() {
    let _g = crate::core::test_environment::lock_env();
    let root = fixture_root("nocursor");
    // no .cursor/
    let home = empty_home("nocursor");
    let err = run_with(&root, &home, "arm-nocursor").unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("open") && msg.contains("debug-8fc1e0.log"),
        "bash-red arm: missing .cursor must fail open: {msg}"
    );
}

#[test]
fn arm_logdir_exits_err() {
    let _g = crate::core::test_environment::lock_env();
    let root = fixture_root("logdir");
    fs::create_dir_all(root.join(".cursor/debug-8fc1e0.log")).unwrap();
    let home = empty_home("logdir");
    let err = run_with(&root, &home, "arm-logdir").unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("open") && msg.contains("debug-8fc1e0.log"),
        "bash-red arm: log-as-dir must fail open: {msg}"
    );
}

#[test]
fn clean_empty_home_writes_unknown_and_missing() {
    let _g = crate::core::test_environment::lock_env();
    let root = fixture_root("clean");
    fs::create_dir_all(root.join(".cursor")).unwrap();
    let home = empty_home("clean");
    // Isolate from operator deploy.env / SSH.
    unsafe {
        std::env::remove_var("TBD_SSH_HOST");
        std::env::remove_var("TBD_SSH_PASS");
    }
    let code = run_with(&root, &home, "clean-baseline").unwrap();
    assert_eq!(code, 0);
    let log = fs::read_to_string(root.join(".cursor/debug-8fc1e0.log")).unwrap();
    assert!(log.contains("\"client_build\":\"unknown\""));
    assert!(log.contains("\"server_build\":\"unknown\""));
    assert!(log.contains("\"path\":\"missing\""));
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
