use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn token_from_env_strips_quotes_and_cr() {
    let dir = tempfile_dir("t861-env");
    let env = dir.join(".env");
    fs::write(&env, "SERVICE_TOKEN=\"abc/def\"\r\nOTHER=1\n").unwrap();
    assert_eq!(token_from_env_file(&env).as_deref(), Some("abc/def"));
}

#[test]
fn token_from_env_first_line_wins() {
    let dir = tempfile_dir("t861-env2");
    let env = dir.join(".env");
    fs::write(&env, "SERVICE_TOKEN=first\nSERVICE_TOKEN=second\n").unwrap();
    assert_eq!(token_from_env_file(&env).as_deref(), Some("first"));
}

#[test]
fn substitute_preserves_ampersand_and_pipe() {
    let dir = tempfile_dir("t861-sub");
    let cfg = dir.join("cfg.json");
    fs::write(&cfg, format!("{{\"serverToken\": \"{PLACEHOLDER}\"}}")).unwrap();
    substitute_token(&cfg, "tok&pipe|slash/x").unwrap();
    let body = fs::read_to_string(&cfg).unwrap();
    assert_eq!(body, "{\"serverToken\": \"tok&pipe|slash/x\"}");
}

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
        "{\n  \"backendUrl\": \"http://127.0.0.1:8080\",\n  \"serverToken\": \"tok\",\n  \"machineCredential\": \"placeholder\"\n}\n",
    )
    .unwrap();
    set_machine_credential(&cfg, "tbdm_abc").unwrap();
    assert_eq!(
        fs::read_to_string(&cfg).unwrap(),
        "{\n  \"backendUrl\": \"http://127.0.0.1:8080\",\n  \"serverToken\": \"tok\",\n  \"machineCredential\": \"tbdm_abc\"\n}\n"
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
    fs::create_dir_all(root.join("apps/website/api_v2")).unwrap();
    if with_backend {
        fs::write(
            root.join(BACKEND_EXAMPLE_REL),
            format!(
                "{{\n  \"backendUrl\": \"http://127.0.0.1:8080\",\n  \"serverToken\": \"{PLACEHOLDER}\",\n  \"machineCredential\": \"replace-with-a-mod_runtime-credential\"\n}}\n"
            ),
        )
        .unwrap();
    }
    fs::write(root.join(REGISTRY_REL), "{\"ok\":true}\n").unwrap();
    root
}
