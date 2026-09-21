use super::*;

fn fixture_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("seed-announcement-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".ai/tickets")).unwrap();
    fs::create_dir_all(root.join("apps/website/api_v2")).unwrap();
    fs::write(root.join(".ai/tickets/ROOT"), "{}").unwrap();
    root
}

fn write_env(root: &Path, body: &str) {
    fs::write(root.join("apps/website/api_v2/.env"), body).unwrap();
}

fn chmod_755(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = fs::metadata(path).unwrap().permissions();
        p.set_mode(0o755);
        fs::set_permissions(path, p).unwrap();
    }
}

fn write_stub(bin: &Path, name: &str, body: &str) -> PathBuf {
    fs::create_dir_all(bin).unwrap();
    let path = bin.join(name);
    fs::write(&path, body).unwrap();
    chmod_755(&path);
    path
}

struct OverrideGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl OverrideGuard {
    fn set(key: &'static str, value: &Path) -> Self {
        let previous = std::env::var_os(key);
        // SAFETY: caller holds crate::core::test_environment::lock_env; restored on drop.
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }

    fn set_absent(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        // Nonexistent path → resolve_tool ToolAbsent (no PATH wipe).
        let absent =
            std::env::temp_dir().join(format!("t872-absent-{}-{}", key, std::process::id()));
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
fn sql_matches_former_heredoc_len() {
    assert_eq!(SQL.len(), 528); // UTF-8 bytes (3 multi-byte dashes)
    assert!(SQL.ends_with(");\n"));
    assert!(SQL.contains("Milestone #1 — Saturday 22 August 2026"));
}

#[test]
fn no_psql_no_container_exits_1() {
    let _g = crate::core::test_environment::lock_env();
    let root = fixture_root("no-psql");
    write_env(&root, "DATABASE_URL=postgres://u:p@127.0.0.1:5432/db\n");
    let bin = root.join("bin");
    let podman = write_stub(&bin, "podman", "#!/bin/sh\nexit 0\n");
    let _no_psql = OverrideGuard::set_absent(ENV_PSQL);
    let _podman = OverrideGuard::set(ENV_PODMAN, &podman);
    unsafe { std::env::remove_var("DATABASE_URL") };
    let code = run_with_root(&root).unwrap();
    let _ = fs::remove_dir_all(&root);
    assert_eq!(code, 1);
}

#[test]
fn missing_env_continues_then_no_psql() {
    let _g = crate::core::test_environment::lock_env();
    let root = fixture_root("no-env");
    let bin = root.join("bin");
    let podman = write_stub(&bin, "podman", "#!/bin/sh\nexit 0\n");
    let _no_psql = OverrideGuard::set_absent(ENV_PSQL);
    let _podman = OverrideGuard::set(ENV_PODMAN, &podman);
    unsafe { std::env::remove_var("DATABASE_URL") };
    let code = run_with_root(&root).unwrap();
    let _ = fs::remove_dir_all(&root);
    assert_eq!(code, 1);
}

#[test]
fn bad_database_url_forwards_psql_rc() {
    let _g = crate::core::test_environment::lock_env();
    let root = fixture_root("bad-url");
    write_env(
        &root,
        "DATABASE_URL=postgres://bad:bad@127.0.0.1:1/nosuch\n",
    );
    let bin = root.join("bin");
    let psql = write_stub(
        &bin,
        "psql",
        "#!/bin/sh\ncat >/dev/null\necho 'psql: error: connection refused' >&2\nexit 2\n",
    );
    let _psql = OverrideGuard::set(ENV_PSQL, &psql);
    unsafe { std::env::remove_var("DATABASE_URL") };
    let code = run_with_root(&root).unwrap();
    let _ = fs::remove_dir_all(&root);
    assert_eq!(code, 2);
}

#[test]
fn the_api_directory_resolves_against_the_given_root() {
    let root = PathBuf::from("/tmp/fake-mono");
    let p = Paths::from_root(&root);
    assert_eq!(p.web, root.join("apps/website/api_v2"));
}
