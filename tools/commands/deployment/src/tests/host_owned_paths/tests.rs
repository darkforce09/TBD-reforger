//! The host-owned paths: where they sit in the checkout. That both rsyncs exclude every one is
//! pinned beside the staging rsync (`staging/tests/remote/tests.rs`), which sees both builders.
use super::*;

/// Each crate folder is a crate of this checkout, and the settings file's template is tracked
/// beside the spot the host keeps the file itself.
#[test]
fn the_crate_folders_and_the_settings_template_exist_in_the_checkout() {
    let root = repository_root::find_repository_root().expect("the checkout root");
    for folder in [
        repository_layout::workspace_folders::API_SERVER_CRATE_DIR,
        FRONTEND_APPLICATION_FOLDER,
    ] {
        assert!(
            root.join(folder).join("Cargo.toml").is_file(),
            "{folder} holds no crate"
        );
    }
    assert!(root.join(API_ENVIRONMENT_TEMPLATE).is_file());
    assert_eq!(
        API_ENVIRONMENT_TEMPLATE.strip_suffix(".example"),
        Some(API_ENVIRONMENT_FILE)
    );
}

/// No tracked file sits at a host-owned path: the host's copy is the only one, and the rsync
/// would otherwise skip a file the host needs from the checkout.
#[test]
fn no_tracked_file_sits_at_a_host_owned_path() {
    let root = repository_root::find_repository_root().expect("the checkout root");
    let tracked = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-files", "--"])
        .args(HOST_OWNED_PATHS)
        .output()
        .expect("git runs");
    assert!(tracked.status.success());
    assert_eq!(String::from_utf8_lossy(&tracked.stdout), "");
}
