//! `--migrate-single-instance`: the check without the flag, and the migration with it.
use super::*;
use crate::commands::deploy::staging::config::tests::base;
use crate::core::secure_shell_transport::{SshBase, ssh_argv};

/// The migration goes to the host exactly as `ssh <host> bash -s` with the payload on stdin; the
/// ssh password, when there is one, is in neither.
#[test]
fn the_migration_argv_is_ssh_bash_stdin_and_carries_no_password() {
    let argv = ssh_argv(
        &SshBase::Pass("ssh-password-canary".into()),
        "deploy@192.0.2.10",
        &["bash".to_string(), "-s".to_string()],
    );
    assert_eq!(
        argv,
        [
            "sshpass",
            "-e",
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            "deploy@192.0.2.10",
            "bash",
            "-s"
        ]
    );
    let env = base();
    let payload = migration_payload(&env.profile_dir, &env.single_instance_server_config());
    assert!(!payload.contains("ssh-password-canary"));
}

#[test]
fn the_migration_stops_disables_and_archives_everything_of_the_single_server() {
    let env = base();
    let p = migration_payload(&env.profile_dir, &env.single_instance_server_config());
    assert!(p.starts_with("set -euo pipefail\numask 077\n"));
    assert!(p.contains("PROFILE='/home/deploy/tbd/profile'\n"));
    assert!(p.contains("SERVER_CONFIG='/home/deploy/tbd/server.config.json'\n"));
    assert!(p.contains("for unit in tbd-reforger.service fleet-host-agent.service; do\n"));
    assert!(p.contains("    systemctl --user stop \"$unit\" || true\n    systemctl --user disable \"$unit\" 2>/dev/null || true\n"));
    assert!(p.contains("for file in agent.toml machine-credential rcon-password; do\n"));
    assert!(
        p.contains("ARCHIVE=\"$HOME/tbd/retired/single-instance-$(date -u +%Y%m%dT%H%M%SZ)\"\n")
    );
    // Moved, never deleted, and into a folder that must not exist yet.
    assert!(p.contains("    mv \"$item\" \"$ARCHIVE/\"\n"));
    assert!(p.contains("  mkdir \"$ARCHIVE\"\n"));
    assert!(!p.contains("rm "), "{p}");
    // A profile inside the fleet root is refused before anything moves.
    let guard = p.find("is inside the fleet root").unwrap();
    assert!(guard < p.find("systemctl --user stop").unwrap());
    // It ends by proving neither unit is still loaded.
    assert!(p.contains("echo \"FAIL: $unit is still loaded after the migration\" >&2\n"));
}

#[test]
fn without_the_flag_an_installed_single_instance_unit_stops_the_deploy() {
    let p = single_instance_units_absent_payload();
    assert!(p.contains("for unit in tbd-reforger.service fleet-host-agent.service; do\n"));
    assert!(p.contains("= \"loaded\" ]; then\n"));
    assert!(p.contains("rerun with --migrate-single-instance"));
    assert!(p.contains("  exit 1\n"));
}

/// Run under a local bash with a scratch HOME and no systemd: the migration moves every
/// single-instance file into one archive folder, and refuses a profile inside the fleet root.
#[test]
fn the_migration_moves_the_files_into_one_archive_under_a_local_bash() {
    let home = std::env::temp_dir().join(format!("tbd-migration-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let agent = home.join(".config/fleet-host-agent");
    std::fs::create_dir_all(home.join("tbd/profile/profile")).unwrap();
    std::fs::create_dir_all(&agent).unwrap();
    std::fs::write(home.join("tbd/server.config.json"), "{}").unwrap();
    std::fs::write(agent.join("agent.toml"), "x").unwrap();
    let run = |profile: &str| {
        let payload = migration_payload(
            profile,
            &format!("{}/tbd/server.config.json", home.display()),
        );
        std::process::Command::new("bash")
            .arg("-c")
            .arg(payload)
            .env("HOME", &home)
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", home.join("bin").display()),
            )
            .output()
            .expect("bash runs")
    };
    // A stand-in `systemctl` that knows no unit, so nothing on this machine is touched.
    std::fs::create_dir_all(home.join("bin")).unwrap();
    let stub = home.join("bin/systemctl");
    std::fs::write(
        &stub,
        "#!/bin/sh\n[ \"$4\" = LoadState ] && echo not-found\nexit 0\n",
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();

    let inside = run(&format!("{}/tbd/fleet/instance-1/profile", home.display()));
    assert_eq!(inside.status.code(), Some(1));
    let out = run(&format!("{}/tbd/profile", home.display()));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let retired: Vec<_> = std::fs::read_dir(home.join("tbd/retired"))
        .unwrap()
        .collect();
    assert_eq!(retired.len(), 1, "one archive folder");
    let archive = retired[0].as_ref().unwrap().path();
    for moved in ["profile", "server.config.json", "agent.toml"] {
        assert!(archive.join(moved).exists(), "{moved} archived");
    }
    assert!(!home.join("tbd/profile").exists() && !agent.join("agent.toml").exists());
    // A second run finds nothing left and creates no second folder.
    assert!(
        run(&format!("{}/tbd/profile", home.display()))
            .status
            .success()
    );
    assert_eq!(
        std::fs::read_dir(home.join("tbd/retired")).unwrap().count(),
        1
    );
    std::fs::remove_dir_all(&home).unwrap();
}
