//! The host-owned paths: where they sit in the checkout. That both rsyncs exclude every one is
//! pinned beside the staging rsync (`staging/tests/remote/tests.rs`), which sees both builders.
use super::*;

/// Each path lies under one of the two crate folders, and each folder is a crate of this checkout.
#[test]
fn every_host_owned_path_lies_under_a_crate_folder_of_the_checkout() {
    let root = repository_root::find_repository_root().expect("the checkout root");
    for folder in [API_SERVER_FOLDER, FRONTEND_APPLICATION_FOLDER] {
        assert!(
            root.join(folder).join("Cargo.toml").is_file(),
            "{folder} holds no crate"
        );
    }
    for path in HOST_OWNED_PATHS {
        assert!(
            [API_SERVER_FOLDER, FRONTEND_APPLICATION_FOLDER]
                .iter()
                .any(|folder| path.starts_with(&format!("{folder}/"))),
            "{path} lies under neither crate folder"
        );
    }
    assert_eq!(API_ENVIRONMENT_FILE, format!("{API_SERVER_FOLDER}/.env"));
    assert_eq!(
        API_HOST_TOOLS_FOLDER,
        format!("{API_SERVER_FOLDER}/.tools/")
    );
    assert_eq!(
        BUILT_APPLICATION_FOLDER,
        format!("{FRONTEND_APPLICATION_FOLDER}/dist/")
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
