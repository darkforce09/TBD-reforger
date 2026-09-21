use super::*;

#[test]
fn prairielearn_case_insensitive() {
    assert!(refuse_prairielearn("TBD_REMOTE_DIR", "/home/sam/PrairieLearn/x").is_err());
    assert!(refuse_prairielearn("TBD_SSH_HOST", "sam@PRAIRIELEARN.local").is_err());
    assert!(refuse_prairielearn("TBD_REMOTE_DIR", "/home/sam/tbd/repo").is_ok());
}

#[test]
fn remote_prefix_rejects_escape_and_outside() {
    assert!(require_tbd_remote_prefix("/home/sam/tbd/../elsewhere").is_err());
    assert!(require_tbd_remote_prefix("/tmp/not-tbd-at-all").is_err());
    assert!(require_tbd_remote_prefix("/home/sam/tbd").is_ok());
    assert!(require_tbd_remote_prefix("/home/sam/tbd/repo///").is_ok());
}

#[test]
fn usage_mentions_dry_run() {
    assert!(USAGE.contains("--dry-run"));
    assert!(USAGE.contains("TBD_REMOTE_DIR"));
}

/// The exclude list is a delete guard as much as a transfer filter: this is an `--delete` rsync
/// with no `--delete-excluded`, so anything dropped from here becomes eligible for removal on the
/// server. Pin the set.
#[test]
fn rsync_excludes_the_asset_scratch_and_legacy_trees() {
    let argv = rsync_argv::rsync_argv("ssh -o StrictHostKeyChecking=no", "/repo/", "h:/remote/");
    let excluded = rsync_argv::exclusions(&argv);

    for needed in [
        ".git/",
        "target/",
        "apps/website/api_v2/.env",
        "scripts/deploy/deploy.env",
        // The served terrain tree, and the 1.5 GB of gitignored export intermediates that the
        // pre-relocation `packages/map-assets/` exclusion used to cover by nesting.
        "assets_v2/terrains/",
        "assets_v2/scratch/",
        // Keeps `--delete` off a server still holding its assets at the old path.
        "packages/",
    ] {
        assert!(excluded.contains(&needed), "missing --exclude={needed}");
    }

    // The glyph atlas MUST ship: the API serves it at `/map-assets/glyphs` and the server has no
    // other copy of it.
    assert!(
        !excluded.iter().any(|e| e.starts_with("assets_v2/glyphs")),
        "assets_v2/glyphs must not be excluded — the API serves it"
    );
}

#[test]
fn rsync_argv_keeps_source_and_destination_last() {
    let argv = rsync_argv::rsync_argv("ssh", "/repo/", "sam@h:/home/sam/tbd/repo/");
    assert_eq!(argv[0], "-e");
    assert_eq!(argv[1], "ssh");
    assert_eq!(argv[2], "-avz");
    assert_eq!(argv[3], "--delete");
    // rsync copies CONTENTS when the source has a trailing slash; both sides keep theirs.
    assert_eq!(argv[argv.len() - 2], "/repo/");
    assert_eq!(argv[argv.len() - 1], "sam@h:/home/sam/tbd/repo/");
}
