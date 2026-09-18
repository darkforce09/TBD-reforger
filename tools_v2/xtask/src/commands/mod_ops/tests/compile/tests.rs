use super::*;
use std::os::unix::fs::PermissionsExt;

fn throwaway(tag: &str) -> PathBuf {
    let root = PathBuf::from(format!("/tmp/t853/compile/ut-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::create_dir_all(root.join("apps/mod/tbd-framework")).unwrap();
    fs::write(root.join(".ai/tickets/ROOT"), "{}\n").unwrap();
    root
}

fn with_home<T>(home: &Path, f: impl FnOnce() -> T) -> T {
    let _lock = crate::core::test_environment::lock_env();
    struct RestoreHome(Option<std::ffi::OsString>);
    impl Drop for RestoreHome {
        fn drop(&mut self) {
            // SAFETY: the process environment lock remains held during restoration.
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
            }
        }
    }
    let _restore = RestoreHome(std::env::var_os("HOME"));
    // SAFETY: all test environment mutations share the process environment lock.
    unsafe { std::env::set_var("HOME", home) };
    f()
}

fn fake_server(home: &Path) {
    let server_dir = home.join(".local/share/Steam/steamapps/common/Arma Reforger Server");
    fs::create_dir_all(&server_dir).unwrap();
    let bin = server_dir.join("ArmaReforgerServer");
    fs::write(&bin, "#!/bin/true\n").unwrap();
    let mut perms = fs::metadata(&bin).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&bin, perms).unwrap();
}

#[test]
fn no_addon_is_rc3() {
    let root = throwaway("noaddon");
    let home = root.join("home");
    fake_server(&home);
    let code = with_home(&home, || run_with_root(&root, &Opts::default()));
    assert_eq!(code, 3);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn no_server_is_rc3() {
    let root = throwaway("noserver");
    fs::write(root.join("apps/mod/tbd-framework/addon.gproj"), "x\n").unwrap();
    let home = root.join("empty-home");
    fs::create_dir_all(&home).unwrap();
    let code = with_home(&home, || run_with_root(&root, &Opts::default()));
    assert_eq!(code, 3);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn missing_probe_is_rc2() {
    let root = throwaway("missprobe");
    let home = root.join("home");
    fake_server(&home);
    fs::write(root.join("apps/mod/tbd-framework/addon.gproj"), "x\n").unwrap();
    let opts = Opts {
        probe_dir: Some(PathBuf::from("/tmp/t853/compile/no-such-probe-dir-ut")),
        ..Default::default()
    };
    let code = with_home(&home, || run_with_root(&root, &opts));
    assert_eq!(code, 2);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn count_game_scripts_counts_only_c_and_refuses_a_missing_tree() {
    let root = throwaway("count");
    let fw = root.join("apps/mod/tbd-framework");
    assert!(
        count_game_scripts(&fw).is_err(),
        "no Scripts/Game must be an error, never a silent 0"
    );
    fs::create_dir_all(fw.join("Scripts/Game/TBD/UI/Common")).unwrap();
    fs::write(fw.join("Scripts/Game/TBD/A.c"), "class A {}\n").unwrap();
    fs::write(fw.join("Scripts/Game/TBD/UI/B.c"), "class B {}\n").unwrap();
    fs::write(
        fw.join("Scripts/Game/TBD/UI/Common/README.md"),
        "placeholder\n",
    )
    .unwrap();
    assert_eq!(count_game_scripts(&fw).unwrap(), 2);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn workbench_tooling_guard_reports_the_dir_and_what_is_in_it() {
    let root = throwaway("wbguard");
    let fw = root.join("apps/mod/tbd-framework");
    fs::create_dir_all(fw.join("Scripts/Game")).unwrap();
    assert!(workbench_tooling_guard(&fw).is_empty());
    fs::create_dir_all(fw.join("Scripts/WorkbenchGame/EnfusionMCP")).unwrap();
    fs::write(
        fw.join("Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_Ping.c"),
        "class EMCP_WB_Ping {}\n",
    )
    .unwrap();
    let lines = workbench_tooling_guard(&fw);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(
        lines[0].ends_with("Scripts/WorkbenchGame: must not exist"),
        "{lines:?}"
    );
    assert!(lines[1].ends_with("EMCP_WB_Ping.c"), "{lines:?}");
    let _ = fs::remove_dir_all(&root);
}
