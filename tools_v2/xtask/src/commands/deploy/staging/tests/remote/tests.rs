use super::*;
use crate::commands::deploy::staging::config::tests::base;

// ── ARGV: the stand-in for live coverage ────────────────────────────────────────────────

#[test]
fn ssh_argv_plain_identity_and_sshpass() {
    let remote = vec!["bash".to_string(), "-s".to_string()];
    assert_eq!(
        ssh_argv(&SshBase::Plain, "sam@h", &remote),
        vec![
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            "sam@h",
            "bash",
            "-s"
        ]
    );
    assert_eq!(
        ssh_argv(&SshBase::Identity("/k/id".into()), "sam@h", &remote),
        vec![
            "ssh",
            "-i",
            "/k/id",
            "-o",
            "StrictHostKeyChecking=no",
            "sam@h",
            "bash",
            "-s"
        ]
    );
    assert_eq!(
        ssh_argv(&SshBase::Pass("pw".into()), "sam@h", &remote),
        vec![
            "sshpass",
            "-p",
            "pw",
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            "sam@h",
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
    assert_eq!(SshBase::from_env(&e), SshBase::Pass("pw".into()));
    e.ssh_pass = None;
    assert_eq!(SshBase::from_env(&e), SshBase::Identity("/k/id".into()));
    e.ssh_identity_file = None;
    assert_eq!(SshBase::from_env(&e), SshBase::Plain);
}

#[test]
fn rsync_argv_keeps_every_exclude_in_order() {
    let argv = rsync_argv(
        &SshBase::Plain,
        Path::new("/repo"),
        "sam@h",
        "/home/sam/tbd/repo",
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
        "--exclude=apps/website/api_v2/.env",
        "--exclude=apps/mod/tbd-export/",
        "--exclude=apps/mod/tbd-emcp/",
        // Build output and map assets: a game-server host needs neither, and `--delete` would
        // otherwise reach them on the server.
        "--exclude=target/",
        "--exclude=assets_v2/terrains/",
        "--exclude=assets_v2/scratch/",
    ] {
        assert!(argv.iter().any(|a| a == needed), "missing {needed}");
    }
    // Source has a trailing slash (rsync copies CONTENTS) and so does the destination.
    assert_eq!(argv[argv.len() - 2], "/repo/");
    assert_eq!(argv[argv.len() - 1], "sam@h:/home/sam/tbd/repo/");
}

#[test]
fn exec_start_config_mode_carries_both_flags() {
    // The whole of T-604/T-607: without -addonsDir the engine loads the Workshop copy.
    let s = exec_start(&base());
    assert!(s.contains(" -addonsDir /home/sam/tbd/addons "), "{s}");
    assert!(
        s.contains(" -config /home/sam/tbd/server.config.json "),
        "{s}"
    );
    assert!(
        !s.contains(" -addons "),
        "config mode must NOT pass -addons: {s}"
    );
    assert!(s.ends_with("-maxFPS 60 -logStats 30000 -nothrow"));
}

#[test]
fn exec_start_addons_mode_quotes_the_scenario() {
    let mut e = base();
    e.server_mode = "addons".into();
    let s = exec_start(&e);
    // -addons <GUID> is the flag that is fatal beside -config, and addons mode is the only
    // place it appears.
    assert!(s.contains(" -addons B2C3D4E5F6A78901 "), "{s}");
    assert!(!s.contains("-config"), "{s}");
    assert!(
        s.contains(" -server \"{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf\" "),
        "{s}"
    );
    assert!(s.contains(" -bindPort 2001 -a2sPort 17777 "), "{s}");
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

#[test]
fn dry_run_never_spawns() {
    // The `Runner` guard, asserted directly: with dry_run set, an `sshpass` that may not exist
    // on this machine still yields 0 and prints the plan instead of reaching a socket.
    let r = Runner { dry_run: true };
    let code = r
        .ssh(
            &SshBase::Pass("pw".into()),
            "sam@h",
            &["bash".to_string(), "-s".to_string()],
            Some("payload".into()),
        )
        .expect("dry run cannot fail");
    assert_eq!(code, 0);
}
