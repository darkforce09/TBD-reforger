//! The secret file check, run under a local bash.
use super::*;
use crate::staging::config::tests::base;

/// A well-formed machine credential; no server accepts it.
const CREDENTIAL: &str = "tbdm_0123456789abcdef0123456789abcdef_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Runs the secret file check under a local bash over a scratch HOME: good files pass; a missing,
/// a group-readable and a malformed file each fail; and no secret value ever reaches the output.
#[test]
fn the_secret_file_check_runs_and_never_prints_a_secret() {
    use std::os::unix::fs::PermissionsExt;
    let home = std::env::temp_dir().join(format!("tbd-secret-check-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    let fleet = home.join("tbd/fleet");
    let write = |path: &std::path::Path, text: &str, mode: u32| {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    let instances = base().fleet.instances();
    write(&fleet.join("join-password"), "join-canary-42\n", 0o600);
    for instance in &instances {
        for file in ["mod-runtime-credential", "host-agent-credential"] {
            let path = fleet.join(format!("instance-{}/secrets/{file}", instance.number));
            write(&path, &format!("{CREDENTIAL}\n"), 0o600);
        }
    }
    let run = || {
        std::process::Command::new("bash")
            .arg("-c")
            .arg(fleet_secret_files_check_payload(&instances))
            .env("HOME", &home)
            .output()
            .expect("bash runs")
    };
    let clean = |out: &std::process::Output| {
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !text.contains("join-canary") && !text.contains("tbdm_"),
            "{text}"
        );
        text
    };
    let good = run();
    assert!(good.status.success(), "{}", clean(&good));
    assert_eq!(clean(&good).matches("  ok      ").count(), 11);

    let second_agent = fleet.join("instance-2/secrets/host-agent-credential");
    std::fs::set_permissions(&second_agent, std::fs::Permissions::from_mode(0o640)).unwrap();
    write(
        &fleet.join("instance-4/secrets/mod-runtime-credential"),
        "tbdm_short\n",
        0o600,
    );
    std::fs::remove_file(fleet.join("instance-5/secrets/host-agent-credential")).unwrap();
    write(&fleet.join("join-password"), "has space inside\n", 0o600);
    let bad = run();
    assert_eq!(bad.status.code(), Some(1));
    let text = clean(&bad);
    assert!(
        text.contains("MISSING instance 2 host_agent credential"),
        "{text}"
    );
    assert!(
        text.contains("INVALID instance 4 mod_runtime credential"),
        "{text}"
    );
    assert!(
        text.contains("MISSING instance 5 host_agent credential"),
        "{text}"
    );
    assert!(text.contains("INVALID join password"), "{text}");
    assert!(text.contains("FAIL: 4 secret file(s)"), "{text}");
    assert!(!text.contains("has space"), "{text}");
    std::fs::remove_dir_all(&home).unwrap();
}
