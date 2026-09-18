use super::*;

fn fixture_root(tag: &str) -> PathBuf {
    let root = PathBuf::from(format!(
        "/tmp/t853/w223/t870/fixture-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::write(root.join(".ai/tickets/ROOT"), "{}").unwrap();
    fs::create_dir_all(root.join("scripts/deploy")).unwrap();
    fs::create_dir_all(root.join("apps/mod")).unwrap();
    root
}

fn clear_ssh_env() {
    unsafe {
        std::env::remove_var("TBD_SSH_HOST");
        std::env::remove_var("TBD_SSH_PASS");
        std::env::remove_var("TBD_SSH_IDENTITY_FILE");
        std::env::remove_var("TBD_REMOTE_DIR");
        std::env::remove_var("TBD_PROFILE_DIR");
        std::env::remove_var("TBD_ADDONS_STAGING");
        std::env::remove_var(ENV_SSH);
        std::env::remove_var(ENV_SSHPASS);
    }
}

struct OverrideGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl OverrideGuard {
    fn set_absent(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        let absent =
            std::env::temp_dir().join(format!("t870-absent-{}-{}", key, std::process::id()));
        // SAFETY: caller holds crate::core::test_environment::lock_env; restored on drop.
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
    let _g = crate::core::test_environment::lock_env();
    clear_ssh_env();
    let root = fixture_root("missing-host");
    // empty deploy.env — no TBD_SSH_HOST
    fs::write(root.join("scripts/deploy/deploy.env"), "").unwrap();
    let code = run_with_root(&root).unwrap();
    assert_eq!(code, 1, "bash-red arm: missing TBD_SSH_HOST must exit 1");
}

#[test]
fn arm_prairielearn_exits_1() {
    let _g = crate::core::test_environment::lock_env();
    clear_ssh_env();
    let root = fixture_root("prairielearn");
    fs::write(
        root.join("scripts/deploy/deploy.env"),
        "TBD_SSH_HOST=127.0.0.1\nTBD_REMOTE_DIR=/home/sam/prairielearn/tbd\n",
    )
    .unwrap();
    let code = run_with_root(&root).unwrap();
    assert_eq!(code, 1, "bash-red arm: prairielearn path must exit 1");
}

#[test]
fn prairielearn_match_is_case_sensitive_like_bash() {
    let _g = crate::core::test_environment::lock_env();
    clear_ssh_env();
    let root = fixture_root("PrairieLearn-case");
    // bash `== *prairielearn*` is case-sensitive — PrairieLearn alone must NOT refuse.
    // Prove via ToolAbsent ssh (env override seam — never wipe PATH).
    fs::write(
        root.join("scripts/deploy/deploy.env"),
        "TBD_SSH_HOST=127.0.0.1\nTBD_REMOTE_DIR=/home/sam/PrairieLearn/tbd\n",
    )
    .unwrap();
    let _no_ssh = OverrideGuard::set_absent(ENV_SSH);
    let code = run_with_root(&root).unwrap();
    assert_eq!(
        code, 127,
        "case-sensitive: PrairieLearn must not match prairielearn refuse (got ToolAbsent)"
    );
}

#[test]
fn defaults_fill_when_unset() {
    let _g = crate::core::test_environment::lock_env();
    clear_ssh_env();
    let root = fixture_root("defaults");
    fs::write(
        root.join("scripts/deploy/deploy.env"),
        "TBD_SSH_HOST=127.0.0.1\n",
    )
    .unwrap();
    let cfg = load_cfg(&root.join("scripts/deploy/deploy.env")).unwrap();
    assert_eq!(cfg.remote_dir, DEFAULT_REMOTE_DIR);
    assert_eq!(cfg.profile_dir, DEFAULT_PROFILE_DIR);
    assert_eq!(cfg.addons_staging, DEFAULT_ADDONS_STAGING);
}

#[test]
fn deploy_env_path_is_paths_sh_pin() {
    let root = Path::new("/tmp/fake-mono");
    let p = Paths::from_root(root);
    assert_eq!(
        p.deploy_env,
        PathBuf::from("/tmp/fake-mono/scripts/deploy/deploy.env")
    );
}
