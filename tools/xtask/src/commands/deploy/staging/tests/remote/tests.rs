use super::*;
use crate::commands::deploy::development_machine_only_paths::DEVELOPMENT_MACHINE_ONLY_PATHS;
use crate::commands::deploy::staging::config::tests::base;
use crate::core::secure_shell_transport::SSH_PASSWORD_VARIABLE;

// ── ARGV: the stand-in for live coverage ────────────────────────────────────────────────

#[test]
fn ssh_argv_plain_identity_and_sshpass() {
    let remote = vec!["bash".to_string(), "-s".to_string()];
    assert_eq!(
        ssh_argv(&SshBase::Plain, "deploy@h", &remote),
        vec![
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            "deploy@h",
            "bash",
            "-s"
        ]
    );
    assert_eq!(
        ssh_argv(&SshBase::Identity("/k/id".into()), "deploy@h", &remote),
        vec![
            "ssh",
            "-i",
            "/k/id",
            "-o",
            "StrictHostKeyChecking=no",
            "deploy@h",
            "bash",
            "-s"
        ]
    );
    assert_eq!(
        ssh_argv(&SshBase::Pass("pw".into()), "deploy@h", &remote),
        vec![
            "sshpass",
            "-e",
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            "deploy@h",
            "bash",
            "-s"
        ]
    );
}

#[test]
fn sshpass_wins_over_identity_file() {
    // ODDITY PINNED: a deploy.env with both silently ignores the key.
    let mut e = base();
    e.ssh_pass = Some("pw".into());
    e.ssh_identity_file = Some("/k/id".into());
    assert_eq!(
        SshBase::from_settings(e.ssh_pass.as_deref(), e.ssh_identity_file.as_deref()),
        SshBase::Pass("pw".into())
    );
    e.ssh_pass = None;
    assert_eq!(
        SshBase::from_settings(e.ssh_pass.as_deref(), e.ssh_identity_file.as_deref()),
        SshBase::Identity("/k/id".into())
    );
    e.ssh_identity_file = None;
    assert_eq!(
        SshBase::from_settings(e.ssh_pass.as_deref(), e.ssh_identity_file.as_deref()),
        SshBase::Plain
    );
}

#[test]
fn rsync_argv_keeps_every_exclude_in_order() {
    let argv = rsync_argv(
        &SshBase::Plain,
        Path::new("/repo"),
        "deploy@h",
        "/home/deploy/tbd/repo",
    );
    assert_eq!(argv[0], "rsync");
    assert_eq!(argv[1], "-e");
    assert_eq!(argv[2], "ssh -o StrictHostKeyChecking=no");
    assert_eq!(argv[3], "-avz");
    assert_eq!(argv[4], "--delete");
    // The licence boundary. All three oracle lanes, plus the credential itself.
    let deploy_env_exclude = format!("--exclude={}", crate::core::repository_layout::DEPLOY_ENV);
    for needed in [
        "--exclude=apps/mod/crf_framework/",
        "--exclude=apps/mod/vanilla_reference/",
        "--exclude=apps/mod/playable_selector/",
        &deploy_env_exclude,
        "--exclude=apps/api/.env",
        "--exclude=apps/mod/tbd-export/",
        "--exclude=apps/mod/tbd-emcp/",
        // Build output and map assets: a game-server host needs neither, and `--delete` would
        // otherwise reach them on the server.
        "--exclude=target/",
        "--exclude=assets/terrains/",
        "--exclude=assets/scratch/",
        "--exclude=assets/equipment/",
        // The app the website deploy built on the host, which the staging Caddy serves.
        "--exclude=apps/frontend/dist/",
    ] {
        assert!(argv.iter().any(|a| a == needed), "missing {needed}");
    }
    // Source has a trailing slash (rsync copies CONTENTS) and so does the destination.
    assert_eq!(argv[argv.len() - 2], "/repo/");
    assert_eq!(argv[argv.len() - 1], "deploy@h:/home/deploy/tbd/repo/");
}

/// Both deploys exclude what only a development machine holds; this lane appends it after its own
/// exclusions, so their order stays the one the wave logs show.
#[test]
fn rsync_argv_excludes_every_development_machine_only_path() {
    let argv = rsync_argv(
        &SshBase::Plain,
        Path::new("/repo"),
        "deploy@h",
        "/home/deploy/tbd/repo",
    );
    let last_own_exclusion = argv
        .iter()
        .position(|a| a == "--exclude=apps/frontend/dist/")
        .expect("the lane's own last exclusion");
    for pattern in DEVELOPMENT_MACHINE_ONLY_PATHS {
        let needed = format!("--exclude={pattern}");
        let at = argv.iter().position(|a| *a == needed);
        assert!(
            at.is_some_and(|at| at > last_own_exclusion),
            "missing {needed} after the lane's own exclusions"
        );
    }
    assert_eq!(argv[argv.len() - 2], "/repo/");
    assert_eq!(argv[argv.len() - 1], "deploy@h:/home/deploy/tbd/repo/");
}

/// The ssh password reaches `sshpass -e` through the spawned process's environment only: no
/// argument vector, `rsync -e` string or printed dry-run line holds it.
#[test]
fn the_ssh_password_is_in_no_argv_and_only_in_the_child_environment() {
    let base = SshBase::Pass("ssh-password-canary".into());
    let remote = vec!["bash".to_string(), "-s".to_string()];
    let argv = ssh_argv(&base, "deploy@h", &remote);
    assert!(
        argv.iter().all(|a| !a.contains("ssh-password-canary")),
        "{argv:?}"
    );
    assert_eq!(base.rsync_e(), "sshpass -e ssh -o StrictHostKeyChecking=no");
    let rsync = rsync_argv(
        &base,
        Path::new("/repo"),
        "deploy@h",
        "/home/deploy/tbd/repo",
    );
    assert!(
        rsync.iter().all(|a| !a.contains("ssh-password-canary")),
        "{rsync:?}"
    );
    assert_eq!(base.password(), Some("ssh-password-canary"));
    assert_eq!(SSH_PASSWORD_VARIABLE, "SSHPASS");
    assert_eq!(SshBase::Plain.password(), None);
}

/// The dry-run plan names every instance with its ports, folder and agent origin, the relay and
/// the migration, and no secret of the settings.
#[test]
fn the_dry_run_plan_walks_every_instance_and_prints_no_secret() {
    let mut env = base();
    env.ssh_pass = Some("ssh-password-canary".into());
    env.admin_password = "admin-password-canary".into();
    env.modpack_token = "modpack-token-canary".into();
    let plan = dry_run_plan(&env, &env.fleet.instances(), true).join("\n");
    for n in 1..=5u16 {
        assert!(
            plan.contains(&format!(
                "[dry-run] instance {n}: \"TBD Staging {n}\" game {} A2S {} RCON 127.0.0.1:{} (admin), visible {}, folder ~/tbd/fleet/instance-{n}",
                2000 + n,
                17776 + n,
                19998 + n,
                n == 1
            )),
            "{plan}"
        );
    }
    assert!(
        plan.contains("agent polls http://127.0.0.1:18085"),
        "{plan}"
    );
    assert!(plan.contains("[dry-run] relay: acknowledgement-dropping-relay@5 on 127.0.0.1:18085"));
    // The migration retires the single instance's kebab-case names; the fleet installs the
    // snake_case ones.
    assert!(
        plan.contains(
            "[dry-run] migrate: stop and disable tbd-reforger.service and fleet-host-agent.service; \
             archive their unit files, ~/.config/fleet-host-agent/, ~/.local/bin/fleet-host-agent, \
             /home/deploy/tbd/profile and /home/deploy/tbd/server.config.json under \
             ~/tbd/retired/single-instance-<UTC time>/"
        ),
        "{plan}"
    );
    assert!(plan.contains("fleet_host_agent@.service"), "{plan}");
    assert!(
        plan.contains(
            "[dry-run] build fleet_host_agent; write ~/.config/fleet_host_agent/instance-N/agent.toml"
        ),
        "{plan}"
    );
    assert!(
        !plan.contains("refuse while"),
        "the migration replaces the refusal"
    );
    assert!(!plan.contains("canary"), "{plan}");
    let without = dry_run_plan(&env, &env.fleet.instances(), false).join("\n");
    assert!(without.contains(
        "[dry-run] refuse while tbd-reforger.service or fleet-host-agent.service is installed"
    ));
    assert!(!without.contains("[dry-run] migrate:"));
}

#[test]
fn v6_maps_all_four_outcomes_and_refuses_to_guess() {
    // The four-outcome contract, which is the whole reason this step is not `set -e`'d.
    assert_eq!(v6_verdict(0), 0, "HEALTHY");
    assert_eq!(
        v6_verdict(2),
        0,
        "PARTIAL is the NORMAL post-deploy state, not a failure"
    );
    assert_eq!(v6_verdict(1), 1, "FAIL");
    assert_eq!(
        v6_verdict(3),
        1,
        "ENVIRONMENT examined nothing and is not a pass"
    );
    assert_eq!(
        v6_verdict(127),
        1,
        "unknown status is a failure, not a guess"
    );
    assert_eq!(v6_verdict(-1), 1);
}

#[test]
fn not_run_never_reads_as_success() {
    // The type-level version of the fail-open this whole program is written against.
    assert_eq!(not_run_exit(&NotRun::ToolAbsent("ssh".into())), 127);
    assert_eq!(
        not_run_exit(&NotRun::Signalled {
            tool: "ssh".into(),
            signal: 9
        }),
        1
    );
    assert_eq!(
        not_run_exit(&NotRun::Timeout {
            tool: "rsync".into(),
            secs: 7200
        }),
        1
    );
}

/// The check asks for the health route under the URL the mod calls, `TBD_BACKEND_URL`'s one
/// reading, and its refusal sends the operator to the command that owns the website stack.
#[test]
fn the_website_api_check_names_the_health_route_and_the_website_deploy() {
    assert_eq!(
        website_api_health_url("http://127.0.0.1:8080"),
        "http://127.0.0.1:8080/healthz"
    );
    // A trailing `/` in the setting is dropped once, by the one reading the deploy's `Env` holds.
    let settings = crate::core::deploy_environment::DeployEnvironment::from_text(
        std::path::Path::new("/home/deploy/checkout/deploy/deploy.env"),
        Some("TBD_SSH_HOST=deploy@192.0.2.10\nTBD_BACKEND_URL=https://api.example.test/\n"),
        std::iter::empty(),
    )
    .expect("parses");
    let env = Env::from_environment(&settings).expect("loads");
    assert_eq!(
        website_api_health_url(&env.backend_url),
        "https://api.example.test/healthz"
    );
    let refusal = website_api_refusal("http://127.0.0.1:8080/healthz", 7);
    assert!(
        refusal.starts_with("ERROR: http://127.0.0.1:8080/healthz does not answer"),
        "{refusal}"
    );
    assert!(refusal.contains("(probe exit 7)"), "{refusal}");
    assert!(
        refusal.contains("`cargo xtask deploy website`"),
        "{refusal}"
    );
}

#[test]
fn the_website_api_check_never_spawns_on_a_dry_run() {
    let r = Runner { dry_run: true };
    assert_eq!(
        require_website_api(&r, &SshBase::Pass("pw".into()), "deploy@h", &base()),
        Ok(())
    );
}

#[test]
fn dry_run_never_spawns() {
    // The `Runner` guard, asserted directly: with dry_run set, an `sshpass` that may not exist
    // on this machine still yields 0 and prints the plan instead of reaching a socket.
    let r = Runner { dry_run: true };
    let code = r
        .ssh(
            &SshBase::Pass("pw".into()),
            "deploy@h",
            &["bash".to_string(), "-s".to_string()],
            Some("payload".into()),
        )
        .expect("dry run cannot fail");
    assert_eq!(code, 0);
}

/// Run under a local bash over a scratch HOME: the scenario read prints the live scenario and
/// nothing else of a config that also holds both passwords, and nothing when there is no config.
#[test]
fn the_scenario_read_returns_only_the_scenario_line() {
    let home = std::env::temp_dir().join(format!("tbd-scenario-read-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let second = base().fleet.instances()[1].clone();
    let read = || {
        std::process::Command::new("bash")
            .arg("-c")
            .arg(scenario_read_payload(&second))
            .env("HOME", &home)
            .output()
            .expect("bash runs")
    };
    let absent = read();
    assert!(absent.status.success());
    assert_eq!(
        scenario_of_config(&String::from_utf8_lossy(&absent.stdout)),
        None
    );
    let folder = home.join("tbd/fleet/instance-2");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("server.config.json"),
        "{\"rcon\":{\"password\":\"rcon-canary\"},\"game\":{\"password\":\"join-canary\",\
         \"scenarioId\":\"{0123456789ABCDEF}Missions/Deployed.conf\",\"visible\":false}}",
    )
    .unwrap();
    let live = read();
    let text = String::from_utf8_lossy(&live.stdout);
    assert_eq!(text.trim_end(), "{0123456789ABCDEF}Missions/Deployed.conf");
    assert!(!text.contains("canary"));
    assert_eq!(
        scenario_of_config(&text).as_deref(),
        Some("{0123456789ABCDEF}Missions/Deployed.conf")
    );
    std::fs::remove_dir_all(&home).unwrap();
}
