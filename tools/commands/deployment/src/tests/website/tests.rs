use super::*;
use crate::development_machine_only_paths::DEVELOPMENT_MACHINE_ONLY_PATHS;

#[test]
fn prairielearn_case_insensitive() {
    assert!(refuse_prairielearn("TBD_REMOTE_DIR", "/home/deploy/PrairieLearn/x").is_err());
    assert!(refuse_prairielearn("TBD_SSH_HOST", "deploy@PRAIRIELEARN.local").is_err());
    assert!(refuse_prairielearn("TBD_REMOTE_DIR", "/home/deploy/tbd/repo").is_ok());
}

#[test]
fn remote_prefix_rejects_escape_and_outside() {
    assert!(
        require_tbd_remote_prefix("/home/deploy/tbd/../elsewhere", "/home/deploy/tbd").is_err()
    );
    assert!(require_tbd_remote_prefix("/tmp/not-tbd-at-all", "/home/deploy/tbd").is_err());
    assert!(require_tbd_remote_prefix("/home/deploy/tbd", "/home/deploy/tbd").is_ok());
    assert!(require_tbd_remote_prefix("/home/deploy/tbd/repo///", "/home/deploy/tbd").is_ok());
    // The guard root is the deploy user's, not a fixed one.
    assert!(require_tbd_remote_prefix("/home/deploy/tbd/repo", "/home/other/tbd").is_err());
    assert!(require_tbd_remote_prefix("/home/deploy/tbdx", "/home/deploy/tbd").is_err());
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
        "crates/api/api_server/.env",
        repository_layout::DEPLOY_ENV,
        // The served terrain tree, and the 1.5 GB of gitignored export intermediates beside it.
        "assets/terrains/",
        "assets/scratch/",
        "assets/equipment/",
        // Keeps `--delete` off a server still holding its assets at the old path.
        "packages/",
    ] {
        assert!(excluded.contains(&needed), "missing --exclude={needed}");
    }

    // The glyph atlas MUST ship: the API serves it at `/map-assets/glyphs` and the server has no
    // other copy of it.
    assert!(
        !excluded.iter().any(|e| e.starts_with("assets/glyphs")),
        "assets/glyphs must not be excluded — the API serves it"
    );
}

/// The licence boundary: the licensed upstream reference lanes never reach the server. One rule
/// covers the whole references folder, so a lane added there is excluded with no edit here.
#[test]
fn rsync_excludes_the_licensed_reference_lanes() {
    let argv = rsync_argv::rsync_argv("ssh", "/repo/", "h:/remote/");
    let excluded = rsync_argv::exclusions(&argv);
    assert!(
        excluded.contains(&"mod/References/"),
        "missing --exclude=mod/References/ in {excluded:?}"
    );
}

/// Both deploys exclude what only a development machine holds, the cargo target folders beside
/// `target/` and the local tool state among it.
#[test]
fn rsync_excludes_every_development_machine_only_path() {
    let argv = rsync_argv::rsync_argv("ssh", "/repo/", "deploy@192.0.2.10:/home/deploy/tbd/repo/");
    let excluded = rsync_argv::exclusions(&argv);
    for pattern in DEVELOPMENT_MACHINE_ONLY_PATHS {
        assert!(excluded.contains(pattern), "missing --exclude={pattern}");
    }
}

#[test]
fn rsync_argv_keeps_source_and_destination_last() {
    let argv = rsync_argv::rsync_argv("ssh", "/repo/", "deploy@192.0.2.10:/home/deploy/tbd/repo/");
    assert_eq!(argv[0], "-e");
    assert_eq!(argv[1], "ssh");
    assert_eq!(argv[2], "-avz");
    assert_eq!(argv[3], "--delete");
    // rsync copies CONTENTS when the source has a trailing slash; both sides keep theirs.
    assert_eq!(argv[argv.len() - 2], "/repo/");
    assert_eq!(
        argv[argv.len() - 1],
        "deploy@192.0.2.10:/home/deploy/tbd/repo/"
    );
}

/// ssh joins its remote arguments into one line for the host's shell, so a step reaches
/// `bash -lc` intact only as one quoted word: a POSIX shell reading that word gives back the step
/// byte for byte, its own quotes included.
#[test]
fn a_remote_step_reaches_the_login_shell_as_one_word() {
    assert_eq!(
        remote_steps::login_shell("cd '/r' && pwd"),
        "bash -lc 'cd '\\''/r'\\'' && pwd'"
    );
    let step = remote_steps::web_server_start_and_reload("/home/deploy/tbd/repo", "5432");
    let line = remote_steps::login_shell(&step);
    let word = line.strip_prefix("bash -lc ").expect("a bash -lc line");
    let read_back = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("printf '%s' {word}"))
        .output()
        .expect("sh runs");
    assert!(read_back.status.success());
    assert_eq!(String::from_utf8(read_back.stdout).unwrap(), step);
}

/// The `caddy` service is given no folder that holds the deploy secrets: none of its bind mounts,
/// read from the compose file's folder, is [`repository_layout::DEPLOY_ENV`]'s folder or a folder
/// above it. A host keeps its `deploy.env` there, and whatever a mount covers is readable inside
/// the container.
#[test]
fn the_caddy_service_mounts_no_folder_holding_the_deploy_secrets() {
    const COMPOSE: &str = include_str!("../../../../../../deploy/compose.staging.yml");
    let mounted = caddy_service_bind_mounts(COMPOSE);
    assert!(!mounted.is_empty(), "the caddy service mounts no folder");
    for folder in &mounted {
        let shown = if folder.is_empty() {
            "the checkout root"
        } else {
            folder.as_str()
        };
        assert!(
            !(folder.is_empty()
                || repository_layout::DEPLOY_ENV.starts_with(&format!("{folder}/"))),
            "the caddy service mounts {shown}, which holds {}",
            repository_layout::DEPLOY_ENV
        );
    }
}

/// The unit keeps the API's runtime files in its own state folder, outside the checkout the
/// rsync `--delete` deletes in: it declares the folder and points the API's upload and equipment
/// folders into it.
#[test]
fn the_unit_template_keeps_the_runtime_files_in_its_state_directory() {
    const UNIT: &str = include_str!("../../../../../../deploy/systemd/tbd-website-api.service");
    let state = "tbd-website-api";
    assert!(
        UNIT.contains(&format!("StateDirectory={state}\n")),
        "{UNIT}"
    );
    assert!(UNIT.contains(&format!("Environment=UPLOAD_DIR=%S/{state}/uploads\n")));
    assert!(UNIT.contains(&format!(
        "Environment=EQUIPMENT_DATA_DIR=%S/{state}/equipment\n"
    )));
}

/// The repository paths of the `caddy` service's bind mounts in the staging compose file, each
/// read from the compose file's folder, [`repository_layout::DEPLOY_DIR`]; an empty path is the
/// checkout root. Named volumes are skipped; an absolute host path fails, since it cannot be
/// placed in the checkout.
fn caddy_service_bind_mounts(compose: &str) -> Vec<String> {
    let (_, service) = compose
        .split_once("\n  caddy:\n")
        .expect("the caddy service");
    // The service's keys sit four spaces deep; its end is the next line indented less that is
    // neither blank nor a comment.
    let service: Vec<&str> = service
        .lines()
        .take_while(|line| {
            line.is_empty() || line.starts_with("    ") || line.trim_start().starts_with('#')
        })
        .collect();
    let volumes_at = service
        .iter()
        .position(|line| *line == "    volumes:")
        .expect("the caddy service's volumes");
    service[volumes_at + 1..]
        .iter()
        .take_while(|line| line.starts_with("      "))
        .filter_map(|line| line.trim_start().strip_prefix("- "))
        .map(|entry| entry.split(':').next().unwrap_or(entry))
        .filter(|source| source.starts_with('.') || source.starts_with('/'))
        .map(|source| {
            assert!(!source.starts_with('/'), "an absolute bind mount: {source}");
            let mut parts: Vec<&str> = repository_layout::DEPLOY_DIR.split('/').collect();
            for part in source.split('/') {
                match part {
                    "" | "." => {}
                    ".." => {
                        parts.pop().expect("a bind mount inside the checkout");
                    }
                    name => parts.push(name),
                }
            }
            parts.join("/")
        })
        .collect()
}
