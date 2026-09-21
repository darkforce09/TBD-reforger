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
    let usage = usage();
    assert!(usage.contains("--dry-run"));
    assert!(usage.contains("TBD_REMOTE_DIR"));
    assert!(usage.contains(crate::core::repository_layout::DEPLOY_ENV));
}

/// The exclude list is a delete guard as much as a transfer filter: this is an `--delete` rsync
/// with no `--delete-excluded`, so anything dropped from here becomes eligible for removal on the
/// server. Pin the set.
#[test]
fn rsync_excludes_the_secrets_asset_and_scratch_trees() {
    let argv = rsync_argv::rsync_argv("ssh -o StrictHostKeyChecking=no", "/repo/", "h:/remote/");
    let excluded = rsync_argv::exclusions(&argv);

    for needed in [
        ".git/",
        "target/",
        "apps/website/api_v2/.env",
        crate::core::repository_layout::DEPLOY_ENV,
        // The served terrain tree, and the 1.5 GB of gitignored export intermediates beside it.
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
    assert_eq!(classify(10), AssetLayout::OldPackagesTree);
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
    let cmd =
        systemd_unit::install_command("/home/sam/tbd/repo", systemd_unit::default_unit_name());
    // The template spells `/TBD_REPO_DIR_PLACEHOLDER/…`, so the value must not carry a slash.
    assert!(
        cmd.contains("s|TBD_REPO_DIR_PLACEHOLDER|home/sam/tbd/repo|g"),
        "{cmd}"
    );
    let unit = systemd_unit::default_unit_name();
    assert!(cmd.contains(&systemd_unit::template_for(unit)));
    assert!(cmd.contains(&format!("~/.config/systemd/user/{unit}")));
    assert_eq!(unit, "tbd-website-api.service");
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
    assert!(report(AssetLayout::OldPackagesTree, dir).is_err());
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

fn plan_for(skip_compose: bool, skip_api: bool, skip_spa: bool) -> Vec<String> {
    let cfg = DeployCfg {
        root: PathBuf::from("/tmp/repo"),
        host: "sam@192.168.0.140".into(),
        remote_dir: "/home/sam/tbd/repo".into(),
        postgres_port: "5432".into(),
        systemd_unit: "tbd-website-api.service".into(),
        skip_compose,
        skip_spa,
        skip_api,
        ssh_pass: None,
        ssh_identity: None,
        dry_run: true,
    };
    cfg.remote_plan().into_iter().map(|s| s.title).collect()
}

/// The checksum repair and the state move follow every build and precede the restart, which
/// `execute` issues only after the plan drains. With every build skipped they are the whole plan.
#[test]
fn the_remote_plan_ends_with_the_checksum_repair_and_the_state_move() {
    let full = plan_for(false, false, false);
    assert_eq!(full.len(), 5, "{full:?}");
    assert!(full[0].contains("Postgres"));
    assert!(full[1].contains("cargo build"));
    assert!(full[2].contains("trunk build"));
    assert!(full[3].contains("checksums"));
    assert!(full[4].contains("state directory"));

    let builds_skipped = plan_for(true, true, true);
    assert_eq!(builds_skipped.len(), 2, "{builds_skipped:?}");
    assert!(builds_skipped[0].contains("checksums"));
    assert!(builds_skipped[1].contains("state directory"));
}

#[test]
fn the_checksum_repair_runs_in_the_remote_checkout_against_the_staging_container() {
    let cmd = remote_steps::migration_checksum_repair("/home/sam/tbd/repo");
    assert!(cmd.starts_with("cd '/home/sam/tbd/repo' &&"), "{cmd}");
    assert!(
        cmd.contains(
            "TBD_DB_CONTAINER=tbd_staging_db cargo xtask db repair-migration-checksum --force"
        ),
        "{cmd}"
    );
}

#[test]
fn the_state_move_targets_the_unit_state_directory_and_is_idempotent() {
    let cmd = remote_steps::runtime_state_move("/home/sam/tbd/repo");
    assert!(
        cmd.contains("${XDG_STATE_HOME:-$HOME/.local/state}/tbd-website-api"),
        "{cmd}"
    );
    assert!(cmd.contains("for legacy in uploads missions"), "{cmd}");
    assert!(
        cmd.contains("'/home/sam/tbd/repo/apps/website/api_v2/'"),
        "{cmd}"
    );
    assert!(
        cmd.contains("if [ -d \"$src\" ]"),
        "the move must be conditional: {cmd}"
    );
    assert!(cmd.contains("--remove-source-files"), "{cmd}");
}

/// The unit template and the deploy agree on where the runtime files live: the unit points the
/// API there through its environment, the deploy moves the files there, and both spell the same
/// `StateDirectory=` name.
#[test]
fn the_unit_template_declares_the_state_directory_the_deploy_moves_into() {
    const UNIT: &str = include_str!("../../../../../deploy/systemd/tbd-website-api.service");
    let state = remote_steps::STATE_DIRECTORY;
    assert!(
        UNIT.contains(&format!("StateDirectory={state}\n")),
        "{UNIT}"
    );
    assert!(UNIT.contains(&format!("Environment=UPLOAD_DIR=%S/{state}/uploads\n")));
    assert!(UNIT.contains(&format!(
        "Environment=MISSION_STAGE_DIR=%S/{state}/missions\n"
    )));
}
