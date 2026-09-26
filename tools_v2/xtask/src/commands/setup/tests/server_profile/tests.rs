use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn missing_backend_exits_1() {
    let root = throwaway_root("no-backend", false);
    let prof = root.join("out-profile");
    let code = run_with_root(&root, Some(&prof)).unwrap();
    assert_eq!(code, 1);
}

#[test]
fn clean_tree_writes_modes_and_no_mission() {
    let root = throwaway_root("clean", true);
    let prof = root.join("out-profile");
    let code = run_with_root(&root, Some(&prof)).unwrap();
    assert_eq!(code, 0);
    let profile_root = prof.join("profile");
    assert_eq!(mode_of(&profile_root), 0o700);
    assert_eq!(mode_of(&profile_root.join("TBD_BackendConfig.json")), 0o600);
    // The mission comes from the server's deployment, never from the profile.
    assert!(!profile_root.join("missions").exists());
}

#[test]
fn machine_credential_is_written_into_the_backend_config_in_place() {
    let dir = tempfile_dir("credential");
    let cfg = dir.join("TBD_BackendConfig.json");
    fs::write(
        &cfg,
        "{\n  \"backendUrl\": \"http://127.0.0.1:8080\",\n  \"machineCredential\": \"placeholder\"\n}\n",
    )
    .unwrap();
    set_machine_credential(&cfg, "tbdm_abc").unwrap();
    assert_eq!(
        fs::read_to_string(&cfg).unwrap(),
        "{\n  \"backendUrl\": \"http://127.0.0.1:8080\",\n  \"machineCredential\": \"tbdm_abc\"\n}\n"
    );
}

fn mode_of(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn tempfile_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("t861-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn throwaway_root(tag: &str, with_backend: bool) -> PathBuf {
    let root = tempfile_dir(tag);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::write(root.join(".ai/tickets/ROOT"), "{}").unwrap();
    fs::create_dir_all(root.join("apps/mod/tbd-framework/Data")).unwrap();
    if with_backend {
        fs::write(
            root.join(BACKEND_EXAMPLE_REL),
            "{\n  \"backendUrl\": \"http://127.0.0.1:8080\",\n  \"machineCredential\": \"replace-with-a-mod_runtime-credential\"\n}\n",
        )
        .unwrap();
    }
    fs::write(root.join(REGISTRY_REL), "{\"ok\":true}\n").unwrap();
    root
}
