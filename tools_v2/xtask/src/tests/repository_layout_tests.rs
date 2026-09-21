use super::*;
use crate::core::repository_root::test_repo_root;

/// Every committed location must exist in a real checkout.
///
/// A constant that names nothing is worse than a literal: a command joins it onto the root and
/// reads a missing file, which surfaces as "not configured" rather than as a broken path.
#[test]
fn every_committed_location_exists_in_the_checkout() {
    let root = test_repo_root();

    for directory in [
        DEPLOY_DIR,
        SYSTEMD_UNITS_DIR,
        DEDICATED_SERVER_PROFILES_DIR,
        MCP_TRANSCRIPT_FIXTURES_DIR,
    ] {
        assert!(
            root.join(directory).is_dir(),
            "not a directory: {directory}"
        );
    }

    for file in [
        DEPLOY_ENV_EXAMPLE,
        CADDYFILE,
        WEBSITE_API_UNIT,
        DEV_SERVER_PROFILE,
    ] {
        assert!(root.join(file).is_file(), "not a file: {file}");
    }
}

/// The host secrets file is never committed, so it is pinned by shape: it sits beside the example
/// an operator copies, and the deploy's own exclude list is built from this constant.
#[test]
fn the_deploy_secrets_file_sits_beside_its_example() {
    assert_eq!(
        DEPLOY_ENV_EXAMPLE,
        format!("{DEPLOY_ENV}.example"),
        "the example must be the secrets path plus `.example`"
    );
    assert!(DEPLOY_ENV.starts_with(&format!("{DEPLOY_DIR}/")));
}

/// Each file constant names something inside the directory constant that describes its kind, so
/// moving a directory cannot leave a file behind.
#[test]
fn every_file_sits_inside_the_directory_that_describes_it() {
    for (file, directory) in [
        (DEPLOY_ENV_EXAMPLE, DEPLOY_DIR),
        (CADDYFILE, DEPLOY_DIR),
        (WEBSITE_API_UNIT, SYSTEMD_UNITS_DIR),
        (DEV_SERVER_PROFILE, DEDICATED_SERVER_PROFILES_DIR),
    ] {
        assert!(
            file.starts_with(&format!("{directory}/")),
            "{file} is not inside {directory}"
        );
    }
}
