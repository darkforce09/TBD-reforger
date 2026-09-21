use super::*;

fn fixture_root(tag: &str) -> PathBuf {
    let root = PathBuf::from(format!(
        "/tmp/xtask-bootstrap-staging/fixture-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::write(root.join(".ai/tickets/ROOT"), "{}").unwrap();
    fs::create_dir_all(root.join(crate::core::repository_layout::DEPLOY_DIR)).unwrap();
    fs::create_dir_all(root.join("apps/mod")).unwrap();
    root
}

/// The deploy file inside a fixture tree.
fn fixture_deploy_env(root: &Path) -> PathBuf {
    root.join(crate::core::repository_layout::DEPLOY_ENV)
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
        let absent = std::env::temp_dir().join(format!(
            "bootstrap-staging-absent-{}-{}",
            key,
            std::process::id()
        ));
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
    fs::write(fixture_deploy_env(&root), "").unwrap();
    let code = run_with_root(&root).unwrap();
    assert_eq!(code, 1, "missing TBD_SSH_HOST must exit 1");
}

#[test]
fn arm_prairielearn_exits_1() {
    let _g = crate::core::test_environment::lock_env();
    clear_ssh_env();
    let root = fixture_root("prairielearn");
    fs::write(
        fixture_deploy_env(&root),
        "TBD_SSH_HOST=127.0.0.1\nTBD_REMOTE_DIR=/home/sam/prairielearn/tbd\n",
    )
    .unwrap();
    let code = run_with_root(&root).unwrap();
    assert_eq!(code, 1, "a prairielearn remote path must exit 1");
}

#[test]
fn the_prairielearn_refusal_is_case_sensitive() {
    let _g = crate::core::test_environment::lock_env();
    clear_ssh_env();
    let root = fixture_root("PrairieLearn-case");
    // The refusal matches the lowercase substring only, so `PrairieLearn` passes it. Proven
    // through an absent `ssh` (the override seam) rather than by wiping PATH.
    fs::write(
        fixture_deploy_env(&root),
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
    fs::write(fixture_deploy_env(&root), "TBD_SSH_HOST=127.0.0.1\n").unwrap();
    let cfg = load_cfg(&fixture_deploy_env(&root)).unwrap();
    assert_eq!(cfg.remote_dir, DEFAULT_REMOTE_DIR);
    assert_eq!(cfg.profile_dir, DEFAULT_PROFILE_DIR);
    assert_eq!(cfg.addons_staging, DEFAULT_ADDONS_STAGING);
}

#[test]
fn the_deploy_file_resolves_against_the_given_root() {
    let root = Path::new("/tmp/fake-mono");
    let p = Paths::from_root(root);
    assert_eq!(
        p.deploy_env,
        root.join(crate::core::repository_layout::DEPLOY_ENV)
    );
}
