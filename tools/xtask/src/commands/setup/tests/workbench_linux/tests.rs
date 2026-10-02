use super::*;
use crate::core::test_environment::{PathGuard, lock_env};
use std::path::PathBuf;
use std::sync::MutexGuard;

#[test]
fn missing_gproj_exits_1() {
    let _g = lock_env();
    let home = tempfile_dir("no-gproj");
    let steam = home.join("steam-base");
    fs::create_dir_all(steam.join("addons/data")).unwrap();
    // addons/data exists but no ArmaReforger.gproj
    let code = run_with_paths(&home, &steam).unwrap();
    assert_eq!(code, 1);
}

#[test]
fn missing_steam_tree_exits_1() {
    let _g = lock_env();
    let home = tempfile_dir("no-steam");
    let steam = home.join("no-such-steam");
    let code = run_with_paths(&home, &steam).unwrap();
    assert_eq!(code, 1);
}

#[test]
fn clean_tree_symlinks_and_prints() {
    let _g = lock_env();
    let home = tempfile_dir("clean");
    let steam = home.join("steam-base");
    let data = steam.join("addons/data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("ArmaReforger.gproj"), "gproj\n").unwrap();

    let code = run_with_paths(&home, &steam).unwrap();
    assert_eq!(code, 0);

    let link = home.join("ArmaReforger-Base/data");
    let meta = fs::symlink_metadata(&link).unwrap();
    assert!(meta.file_type().is_symlink());
    let target = fs::read_link(&link).unwrap();
    assert_eq!(target, data);
    assert!(link.join("ArmaReforger.gproj").is_file());
}

#[test]
fn run_reads_home_and_steam_base_env() {
    let _g: MutexGuard<'_, ()> = lock_env();
    let home = tempfile_dir("env-run");
    let steam = home.join("steam-base");
    let data = steam.join("addons/data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("ArmaReforger.gproj"), "gproj\n").unwrap();

    // Keep system bins on PATH (wave-224 flake: never stub-only PATH).
    let _path = PathGuard::prepend_dir(Path::new("/usr/bin"));

    let prev_home = std::env::var_os("HOME");
    let prev_steam = std::env::var_os("STEAM_BASE");
    // SAFETY: under ENV_LOCK; restored below.
    unsafe {
        std::env::set_var("HOME", &home);
        std::env::set_var("STEAM_BASE", &steam);
    }
    let code = run().unwrap();
    match prev_home {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    match prev_steam {
        Some(v) => unsafe { std::env::set_var("STEAM_BASE", v) },
        None => unsafe { std::env::remove_var("STEAM_BASE") },
    }
    assert_eq!(code, 0);
    assert!(
        home.join("ArmaReforger-Base/data/ArmaReforger.gproj")
            .is_file()
    );
}

#[test]
fn relink_replaces_existing_symlink() {
    let _g = lock_env();
    let home = tempfile_dir("relink");
    let steam_a = home.join("steam-a");
    let steam_b = home.join("steam-b");
    for steam in [&steam_a, &steam_b] {
        let data = steam.join("addons/data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("ArmaReforger.gproj"), "gproj\n").unwrap();
    }
    assert_eq!(run_with_paths(&home, &steam_a).unwrap(), 0);
    assert_eq!(run_with_paths(&home, &steam_b).unwrap(), 0);
    let link = home.join("ArmaReforger-Base/data");
    assert_eq!(fs::read_link(&link).unwrap(), steam_b.join("addons/data"));
}

fn tempfile_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "t875-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}
