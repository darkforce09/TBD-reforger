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

#[test]
fn asset_probe_distinguishes_the_three_layouts() {
    use asset_preflight::{AssetLayout, classify};
    assert_eq!(classify(0), AssetLayout::Ready);
    assert_eq!(classify(10), AssetLayout::Legacy);
    assert_eq!(classify(11), AssetLayout::Absent);
    // An unreachable host (ssh's own 255) must not read as any layout verdict.
    assert_eq!(classify(255), AssetLayout::Indeterminate(255));
    assert_eq!(classify(12), AssetLayout::Indeterminate(12));
}

/// A populated tree is proven by its registry file, not by a directory existing: Docker creates a
/// missing bind-mount source as an empty directory, and that must not read as Ready.
#[test]
fn asset_probe_checks_the_registry_file_and_the_legacy_directory() {
    let script = asset_preflight::probe_script("/home/sam/tbd/repo");
    assert!(script.contains("cd '/home/sam/tbd/repo'"));
    assert!(script.contains("-f assets_v2/terrains/terrain-registry.json"));
    assert!(!script.contains("-d assets_v2/terrains"));
    assert!(script.contains("-d packages/map-assets"));
}

#[test]
fn the_unit_install_command_renders_the_shipped_template_for_the_remote_dir() {
    let cmd = systemd_unit::install_command("/home/sam/tbd/repo", "tbd-website-api.service");
    // The template spells `/TBD_REPO_DIR_PLACEHOLDER/…`, so the value must not carry a slash.
    assert!(
        cmd.contains("s|TBD_REPO_DIR_PLACEHOLDER|home/sam/tbd/repo|g"),
        "{cmd}"
    );
    assert!(cmd.contains("scripts/deploy/tbd-website-api.service"));
    assert!(cmd.contains("~/.config/systemd/user/tbd-website-api.service"));
    assert!(cmd.contains("systemctl --user daemon-reload"));
}

/// Only the legacy layout and an unreadable probe stop the deploy. A host with no asset tree at
/// all is a library-only site, which HOME_SERVER documents as a supported cutover.
#[test]
fn only_a_legacy_or_unreadable_layout_refuses_the_deploy() {
    use asset_preflight::{AssetLayout, report};
    let dir = "/home/sam/tbd/repo";
    assert!(report(AssetLayout::Ready, dir).is_ok());
    assert!(report(AssetLayout::Absent, dir).is_ok());
    assert!(report(AssetLayout::Legacy, dir).is_err());
    assert!(report(AssetLayout::Indeterminate(255), dir).is_err());
}

#[test]
fn the_remediation_names_every_directory_that_must_move() {
    let fix = asset_preflight::remediation("/home/sam/tbd/repo");
    assert!(fix.contains("mkdir -p assets_v2/terrains"));
    for moved in ["everon", "arland", "terrain-registry.json"] {
        assert!(fix.contains(moved), "remediation omits {moved}");
    }
}
