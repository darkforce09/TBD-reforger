use super::*;
use crate::commands::deploy::development_machine_only_paths::DEVELOPMENT_MACHINE_ONLY_PATHS;

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
        "apps/api/.env",
        crate::core::repository_layout::DEPLOY_ENV,
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
        excluded.contains(&"apps/mod/References/"),
        "missing --exclude=apps/mod/References/ in {excluded:?}"
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

/// The dry run prints one line per exclusion of the argv, the development-machine-only paths
/// included, because each is also a path `--delete` leaves alone on the host.
#[test]
fn the_dry_run_prints_every_exclusion_the_development_machine_only_paths_included() {
    let lines = rsync_argv::dry_run_lines("deploy@192.0.2.10", "/home/deploy/tbd/repo");
    assert_eq!(
        lines[0],
        "[dry-run] rsync -avz --delete … deploy@192.0.2.10:/home/deploy/tbd/repo/"
    );
    for pattern in DEVELOPMENT_MACHINE_ONLY_PATHS {
        let line = format!("[dry-run]   --exclude={pattern}");
        assert!(lines.contains(&line), "the dry run lacks {line}");
    }
    let argv = rsync_argv::rsync_argv("", "", "");
    let printed: Vec<&str> = lines[1..]
        .iter()
        .map(|line| {
            line.strip_prefix("[dry-run]   --exclude=")
                .expect("an exclusion line")
        })
        .collect();
    assert_eq!(printed, rsync_argv::exclusions(&argv));
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
    let script = asset_preflight::probe_script("/home/deploy/tbd/repo");
    assert!(script.contains("cd '/home/deploy/tbd/repo'"));
    assert!(script.contains("-f assets/terrains/terrain-registry.json"));
    assert!(!script.contains("-d assets/terrains"));
    assert!(script.contains("-d packages/map-assets"));
}

#[test]
fn the_unit_install_command_renders_the_shipped_template_for_the_remote_dir() {
    let cmd =
        systemd_unit::install_command("/home/deploy/tbd/repo", systemd_unit::default_unit_name());
    // The template spells `/TBD_REPO_DIR_PLACEHOLDER/…`, so the value must not carry a slash.
    assert!(
        cmd.contains("s|TBD_REPO_DIR_PLACEHOLDER|home/deploy/tbd/repo|g"),
        "{cmd}"
    );
    let unit = systemd_unit::default_unit_name();
    assert!(cmd.contains(&systemd_unit::template_for(unit)));
    assert!(cmd.contains(&format!("~/.config/systemd/user/{unit}")));
    assert_eq!(unit, "tbd-website-api.service");
    assert!(cmd.contains("systemctl --user daemon-reload"));
}

/// Only the old packages tree and an unreadable probe stop the deploy. A host with no asset tree at
/// all is a library-only site, which `documentation/runbooks/website_deployment.md` documents
/// as a supported cutover.
#[test]
fn only_a_legacy_or_unreadable_layout_refuses_the_deploy() {
    use asset_preflight::{AssetLayout, report};
    let dir = "/home/deploy/tbd/repo";
    assert!(report(AssetLayout::Ready, dir).is_ok());
    assert!(report(AssetLayout::Absent, dir).is_ok());
    assert!(report(AssetLayout::OldPackagesTree, dir).is_err());
    assert!(report(AssetLayout::Indeterminate(255), dir).is_err());
}

#[test]
fn the_remediation_names_every_directory_that_must_move() {
    let fix = asset_preflight::remediation("/home/deploy/tbd/repo");
    assert!(fix.contains("mkdir -p assets/terrains"));
    for moved in ["everon", "arland", "terrain-registry.json"] {
        assert!(fix.contains(moved), "remediation omits {moved}");
    }
}

fn plan_for(skip_compose: bool, skip_api: bool, skip_spa: bool) -> Vec<String> {
    let cfg = DeployCfg {
        root: PathBuf::from("/tmp/repo"),
        host: "deploy@192.0.2.10".into(),
        remote_dir: "/home/deploy/tbd/repo".into(),
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
fn the_remote_plan_ends_with_the_checksum_repair() {
    let full = plan_for(false, false, false);
    assert_eq!(full.len(), 6, "{full:?}");
    assert!(full[0].contains("Postgres"));
    assert!(full[1].contains("cargo build"));
    assert!(full[2].contains("staging host tools"));
    assert!(full[3].contains("trunk build"));
    assert!(full[4].contains("Caddy"));
    assert!(full[5].contains("checksums"));

    let builds_skipped = plan_for(true, true, true);
    assert_eq!(builds_skipped.len(), 1, "{builds_skipped:?}");
    assert!(builds_skipped[0].contains("checksums"));
}

/// The web server step belongs to compose, not to the app build: skipping the build keeps Caddy
/// serving the `dist` already on the host, and skipping compose drops Postgres and Caddy together.
#[test]
fn the_web_server_step_follows_compose_and_not_the_app_build() {
    let spa_skipped = plan_for(false, false, true);
    assert_eq!(spa_skipped.len(), 5, "{spa_skipped:?}");
    assert!(spa_skipped[3].contains("Caddy"), "{spa_skipped:?}");

    let compose_skipped = plan_for(true, false, false);
    assert_eq!(compose_skipped.len(), 4, "{compose_skipped:?}");
    assert!(
        !compose_skipped
            .iter()
            .any(|title| title.contains("Postgres") || title.contains("Caddy")),
        "{compose_skipped:?}"
    );
}

/// The staging host tools build right after the API, whose checkout and database they share, and
/// the step names every tool it builds; `TBD_SKIP_API_BUILD` drops both cargo steps together.
#[test]
fn the_staging_host_tools_build_after_the_api_and_skip_with_it() {
    let full = plan_for(false, false, false);
    let api = full
        .iter()
        .position(|title| title.contains("--bin api"))
        .expect("the API build");
    let tools = &full[api + 1];
    for tool in &remote_steps::STAGING_HOST_TOOLS {
        assert!(tools.contains(tool.executable), "{tools}");
    }

    let api_skipped = plan_for(false, true, false);
    assert_eq!(api_skipped.len(), 4, "{api_skipped:?}");
    assert!(
        !api_skipped
            .iter()
            .any(|title| title.contains("cargo build")),
        "{api_skipped:?}"
    );
}

/// Each tool builds in the checkout with the deploy account's toolchain, one `cargo build` per
/// package, and the step fails unless every executable then exists under `target/release/`.
#[test]
fn the_staging_host_tools_step_builds_and_proves_every_executable() {
    let command = remote_steps::staging_host_tools_build("/home/deploy/tbd/repo");
    let preamble = format!(
        "cd '/home/deploy/tbd/repo' && {} && ",
        crate::commands::deploy::remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH
    );
    assert!(command.starts_with(&preamble), "{command}");
    for (package, executable) in [
        ("api", "staging-fixtures"),
        ("developer_tools", "acknowledgement-dropping-relay"),
    ] {
        let build = format!("cargo build --release -p {package} --bin {executable}");
        let proof = format!("test -x target/release/{executable}");
        let built_at = command.find(&build).unwrap_or_else(|| panic!("{command}"));
        let proven_at = command.find(&proof).unwrap_or_else(|| panic!("{command}"));
        assert!(built_at < proven_at, "{command}");
    }
    assert_eq!(remote_steps::STAGING_HOST_TOOLS.len(), 2);
    assert!(command.ends_with("test -x target/release/acknowledgement-dropping-relay"));
}

/// Every tool the deploy builds is a `[[bin]]` of the package it names, and the relay unit that
/// `cargo xtask deploy staging` installs runs the relay under the same executable name, so the
/// host never builds or runs a name that no longer exists.
#[test]
fn every_staging_host_tool_is_an_executable_its_package_declares() {
    const WEBSITE_API: &str = include_str!("../../../../../../../apps/api/Cargo.toml");
    const DEVELOPER_TOOLS: &str = include_str!("../../../../../../developer_tools/Cargo.toml");
    const RELAY_UNIT: &str =
        include_str!("../../../../../../../deploy/systemd/acknowledgement-dropping-relay@.service");
    for tool in &remote_steps::STAGING_HOST_TOOLS {
        let manifest: toml::Value = [WEBSITE_API, DEVELOPER_TOOLS]
            .iter()
            .map(|text| text.parse::<toml::Value>().expect("a manifest parses"))
            .find(|manifest| manifest["package"]["name"].as_str() == Some(tool.package))
            .unwrap_or_else(|| panic!("no manifest declares the package {}", tool.package));
        let executables: Vec<&str> = manifest
            .get("bin")
            .and_then(toml::Value::as_array)
            .map(|bins| bins.iter().filter_map(|bin| bin["name"].as_str()).collect())
            .unwrap_or_default();
        assert!(
            executables.contains(&tool.executable),
            "{} declares no [[bin]] named {} (it declares {executables:?})",
            tool.package,
            tool.executable
        );
    }
    assert!(
        RELAY_UNIT.contains("ExecStart=%h/.local/bin/acknowledgement-dropping-relay serve "),
        "{RELAY_UNIT}"
    );
}

/// Both compose steps run the staging compose file from the checkout root with the configured
/// Postgres port, under docker compose when the host has docker and podman compose otherwise.
#[test]
fn every_compose_step_runs_the_staging_compose_file_from_the_checkout() {
    let dir = "/home/deploy/tbd/repo";
    for (command, action) in [
        (
            remote_steps::postgres_start(dir, "5433"),
            "staging_compose up -d postgres",
        ),
        (
            remote_steps::web_server_start_and_reload(dir, "5433"),
            "staging_compose up -d caddy",
        ),
    ] {
        assert!(
            command.starts_with(
                "cd '/home/deploy/tbd/repo' && export TBD_POSTGRES_HOST_PORT='5433' && "
            ),
            "{command}"
        );
        assert!(
            command.contains(
                "if command -v docker >/dev/null 2>&1; then \
                 docker compose -f deploy/compose.staging.yml \"$@\"; else \
                 podman compose -f deploy/compose.staging.yml \"$@\"; fi;"
            ),
            "{command}"
        );
        assert!(command.contains(action), "{command}");
    }
    assert!(remote_steps::postgres_start(dir, "5433").ends_with("staging_compose up -d postgres"));
}

/// The reload runs after `up`, inside the `caddy` service, on the Caddyfile the service was started
/// with, and gives up with exit 1 after a bounded number of attempts rather than looping forever.
#[test]
fn the_web_server_step_starts_caddy_then_reloads_the_mounted_caddyfile() {
    let command = remote_steps::web_server_start_and_reload("/home/deploy/tbd/repo", "5432");
    let reload = format!(
        "staging_compose exec -T caddy caddy reload --config {} --adapter caddyfile",
        remote_steps::caddyfile_in_container()
    );
    let start = command
        .find("staging_compose up -d caddy && ")
        .expect("the start");
    let reload_at = command.find(&reload).expect("the reload");
    assert!(start < reload_at, "{command}");
    assert!(
        command.contains(&format!(
            "if [ \"$attempt\" -ge {} ]; then exit 1; fi;",
            remote_steps::CADDY_RELOAD_ATTEMPTS
        )),
        "{command}"
    );
    assert!(command.ends_with("sleep 1; done"), "{command}");
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

/// The compose file's `caddy` service, the Caddyfile and the reload agree: the service mounts the
/// Caddyfile's folder where the reload looks for the Caddyfile and starts Caddy on that file, and
/// it mounts the app's folder where the Caddyfile's site root points.
#[test]
fn the_caddy_service_serves_what_the_caddyfile_and_the_reload_name() {
    const COMPOSE: &str = include_str!("../../../../../../../deploy/compose.staging.yml");
    const CADDYFILE: &str = include_str!("../../../../../../../deploy/caddy/Caddyfile");
    let in_container = remote_steps::caddyfile_in_container();
    assert_eq!(in_container, "/etc/tbd-caddy/Caddyfile");
    let mount = remote_steps::CADDY_CONFIG_MOUNT;
    // The compose file lives in the deploy folder, so the Caddyfile's folder is named from there.
    let (caddyfile_folder, _) = repository_layout::CADDYFILE
        .rsplit_once('/')
        .expect("the Caddyfile sits in a folder");
    let from_compose = caddyfile_folder
        .strip_prefix(&format!("{}/", repository_layout::DEPLOY_DIR))
        .expect("the Caddyfile's folder sits in the deploy folder");
    for line in [
        "  caddy:\n".to_string(),
        "    container_name: tbd_staging_caddy\n".to_string(),
        "    network_mode: host\n".to_string(),
        format!(
            "    command: [\"caddy\", \"run\", \"--config\", \"{in_container}\", \"--adapter\", \"caddyfile\"]\n"
        ),
        format!("      - ./{from_compose}:{mount}:ro\n"),
        "      - ../apps/frontend:/srv/tbd-frontend:ro\n".to_string(),
    ] {
        assert!(COMPOSE.contains(&line), "the compose file lacks {line:?}");
    }
    assert!(CADDYFILE.contains("\t\troot * /srv/tbd-frontend/dist\n"));
    assert!(CADDYFILE.contains(":3080 {\n"));
}

/// The `caddy` service is given no folder that holds the deploy secrets: none of its bind mounts,
/// read from the compose file's folder, is [`repository_layout::DEPLOY_ENV`]'s folder or a folder
/// above it. A host keeps its `deploy.env` there, and whatever a mount covers is readable inside
/// the container.
#[test]
fn the_caddy_service_mounts_no_folder_holding_the_deploy_secrets() {
    const COMPOSE: &str = include_str!("../../../../../../../deploy/compose.staging.yml");
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

/// The Caddyfile trusts a forwarded client address from the tunnel's loopback peer alone: its
/// global options open the file and name exactly `127.0.0.1/32`, and nothing else in it trusts a
/// proxy. With a wider range, a LAN client's own `X-Forwarded-For` text would travel upstream as
/// though a proxy had written it.
#[test]
fn the_caddyfile_trusts_forwarded_addresses_only_from_the_tunnel_peer() {
    const CADDYFILE: &str = include_str!("../../../../../../../deploy/caddy/Caddyfile");
    let first_block = CADDYFILE
        .lines()
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .expect("a directive");
    assert_eq!(
        first_block, "{",
        "the global options must open the Caddyfile"
    );
    assert!(
        CADDYFILE.contains("{\n\tservers {\n\t\ttrusted_proxies static 127.0.0.1/32\n\t}\n}\n"),
        "{CADDYFILE}"
    );
    let trusting: Vec<&str> = CADDYFILE
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && line.contains("trusted_proxies"))
        .collect();
    assert_eq!(trusting, ["\t\ttrusted_proxies static 127.0.0.1/32"]);
}

#[test]
fn the_checksum_repair_runs_in_the_remote_checkout_against_the_staging_container() {
    let cmd = remote_steps::migration_checksum_repair("/home/deploy/tbd/repo");
    assert!(cmd.starts_with("cd '/home/deploy/tbd/repo' &&"), "{cmd}");
    assert!(
        cmd.contains(
            "TBD_DB_CONTAINER=tbd_staging_db cargo xtask db repair-migration-checksum --force"
        ),
        "{cmd}"
    );
}

/// The unit keeps the API's runtime files in its own state folder, outside the checkout the
/// rsync `--delete` deletes in: it declares the folder and points the API's upload and equipment
/// folders into it.
#[test]
fn the_unit_template_keeps_the_runtime_files_in_its_state_directory() {
    const UNIT: &str = include_str!("../../../../../../../deploy/systemd/tbd-website-api.service");
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

/// The host's `.env` starts as a copy of `.env.example`, the unit loads it through
/// `EnvironmentFile=`, and systemd lets a value from that file override the unit's own
/// `Environment=` line for the same variable. So the template leaves every variable the unit pins
/// unset: an active line would replace the unit's absolute path with the template's relative
/// development one, which the API refuses outside development.
#[test]
fn the_env_template_sets_none_of_the_variables_the_unit_pins() {
    const UNIT: &str = include_str!("../../../../../../../deploy/systemd/tbd-website-api.service");
    const ENV_TEMPLATE: &str = include_str!("../../../../../../../apps/api/.env.example");
    let pinned: Vec<&str> = UNIT
        .lines()
        .filter_map(|line| line.strip_prefix("Environment="))
        .filter_map(|assignment| assignment.split_once('=').map(|(name, _)| name))
        .collect();
    for name in [
        "MAP_ASSETS_DIR",
        "GLYPH_ASSETS_DIR",
        "UPLOAD_DIR",
        "EQUIPMENT_DATA_DIR",
    ] {
        assert!(pinned.contains(&name), "the unit no longer pins {name}");
    }
    for name in pinned {
        let set_by_template = ENV_TEMPLATE.lines().map(str::trim_start).any(|line| {
            !line.starts_with('#')
                && line
                    .split_once('=')
                    .is_some_and(|(key, _)| key.trim() == name)
        });
        assert!(
            !set_by_template,
            ".env.example sets {name}, which would override the unit's pin; comment it out"
        );
    }
}
