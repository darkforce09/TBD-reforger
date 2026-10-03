#![allow(
    unsafe_code,
    reason = "the tests set process variables under `tool_test_support::lock_env`; std offers no safe call that writes one"
)]

use super::*;
use std::path::Path;

/// Deploy settings from `file` alone, with no process variables.
fn settings(file: &str) -> DeployEnvironment {
    DeployEnvironment::from_text(
        Path::new("/home/deploy/checkout/deploy/deploy.env"),
        Some(file),
        [],
    )
    .expect("parses")
}

struct OverrideGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl OverrideGuard {
    fn set_absent(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        let absent = std::env::temp_dir().join(format!(
            "bootstrap-staging-absent-{}-{}",
            key,
            std::process::id()
        ));
        // SAFETY: caller holds tool_test_support::lock_env; restored on drop.
        unsafe { std::env::set_var(key, &absent) };
        Self { key, previous }
    }
}

impl Drop for OverrideGuard {
    fn drop(&mut self) {
        unsafe {
            match self.previous.take() {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

#[test]
fn arm_missing_host_exits_1() {
    let code = run_with_environment(&settings("")).unwrap();
    assert_eq!(code, 1, "missing TBD_SSH_HOST must exit 1");
}

#[test]
fn arm_prairielearn_exits_1() {
    let file = "TBD_SSH_HOST=deploy@192.0.2.10\nTBD_REMOTE_DIR=/home/deploy/prairielearn/tbd\n";
    let code = run_with_environment(&settings(file)).unwrap();
    assert_eq!(code, 1, "a prairielearn remote path must exit 1");
}

#[test]
fn the_prairielearn_refusal_is_case_sensitive() {
    let _g = tool_test_support::lock_env();
    // The refusal matches the lowercase substring only, so `PrairieLearn` passes it. Proven
    // through an absent `ssh` (the override seam) rather than by wiping PATH.
    let file = "TBD_SSH_HOST=deploy@192.0.2.10\nTBD_REMOTE_DIR=/home/deploy/PrairieLearn/tbd\n";
    let _no_ssh = OverrideGuard::set_absent(ENV_SSH);
    let code = run_with_environment(&settings(file)).unwrap();
    assert_eq!(
        code, 127,
        "case-sensitive: PrairieLearn must not match prairielearn refuse (got ToolAbsent)"
    );
}

#[test]
fn folders_default_under_the_deploy_users_home() {
    let cfg = load_cfg(&settings("TBD_SSH_HOST=deploy@192.0.2.10\n")).unwrap();
    assert_eq!(cfg.host, "deploy@192.0.2.10");
    assert_eq!(cfg.remote_dir, "/home/deploy/tbd/repo");
    assert_eq!(cfg.profile_dir, "/home/deploy/tbd/profile");
    assert_eq!(cfg.addons_staging, "/home/deploy/tbd/addons-staging");
}

#[test]
fn a_host_without_a_user_needs_every_folder_set() {
    let bare = settings("TBD_SSH_HOST=192.0.2.10\n");
    assert_eq!(load_cfg(&bare).err(), Some(bare.missing("TBD_REMOTE_DIR")));
    assert_eq!(run_with_environment(&bare).unwrap(), 1);
}

/// The manual steps name the fleet's per-instance credential files, never the retired
/// `deploy.env` credential keys.
#[test]
fn next_steps_name_the_fleets_per_instance_credential_files() {
    let steps = next_steps().join("\n");
    for retired in [
        "TBD_MOD_RUNTIME_CREDENTIAL",
        "TBD_HOST_AGENT_CREDENTIAL",
        "TBD_RCON_PASSWORD",
    ] {
        assert!(!steps.contains(retired), "{retired} in:\n{steps}");
    }
    for file in [
        "~/tbd/fleet/instance-N/secrets/mod-runtime-credential",
        "host-agent-credential",
        "~/tbd/fleet/join-password",
    ] {
        assert!(steps.contains(file), "{file} missing in:\n{steps}");
    }
}
