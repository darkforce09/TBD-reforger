//! T-870 — port of `scripts/mod/bootstrap-staging-server.sh` → `cargo xtask mod bootstrap-staging`.
//!
//! Path pins mirror `scripts/mod/lib/paths.sh` (do **not** delete paths.sh — T-879):
//! `MONO_ROOT`, `MOD_ROOT=apps/mod`, `SCHEMA=packages/tbd-schema`, `WEB=apps/website/api`,
//! `DEPLOY_ENV=scripts/deploy/deploy.env`.
//!
//! One-time staging-host discovery + mkdir. Does **not** install steamcmd / Arma — see
//! `docs/mod/STAGING-SERVER.md`.
//!
//! Fail-opens closed / pinned vs bash:
//! - Missing `deploy.env` is OK (`[ -f … ] && source`) — pinned soft probe.
//! - Unset/empty `TBD_SSH_HOST` after optional source exits **1** with the bash
//!   `${VAR:?…}` shape (historical script path + line 15) — preserved oddity, not "fixed" to 3.
//! - `TBD_REMOTE_DIR` containing literal `prairielearn` (case-sensitive, bash `== *prairielearn*`)
//!   refuses with rc=1 — pinned.
//! - Absent `ssh` / `sshpass`: bash `set -e` would die on command-not-found. **Closed for
//!   ToolAbsent** via `tbd_gate::proc::which` → exit 127 with an explicit message (no silent
//!   success). Transport / remote nonzero still hard-fail like bash `set -e`.
//!
//! Test seams (prefer these over PATH stubs — PATH races `gate_crf_leak`'s `/usr/bin/grep`):
//! - `TBD_BOOTSTRAP_STAGING_SSH` — absolute ssh path (checked before `PATH`). Set-but-missing
//!   forces [`NotRun::ToolAbsent`].
//! - `TBD_BOOTSTRAP_STAGING_SSHPASS` — same for sshpass.
//! - Remote discovery soft probes (`ss … 2>/dev/null`, `docker … 2>/dev/null || echo`) stay inside
//!   the remote heredoc — preserved oddities of the discovery script, not local fail-opens.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tbd_gate::proc::{self, Run};
use tbd_gate::verdict::NotRun;

use crate::root::find_repo_root;

const DEFAULT_REMOTE_DIR: &str = "/home/sam/tbd/repo";
const DEFAULT_PROFILE_DIR: &str = "/home/sam/tbd/profile";
const DEFAULT_ADDONS_STAGING: &str = "/home/sam/tbd/addons-staging";

/// Optional absolute ssh path for unit tests (avoids PATH mutation).
const ENV_SSH: &str = "TBD_BOOTSTRAP_STAGING_SSH";
/// Optional absolute sshpass path for unit tests (avoids PATH mutation).
const ENV_SSHPASS: &str = "TBD_BOOTSTRAP_STAGING_SSHPASS";

/// Remote discovery body — byte-stable with the former bash `<<'DISC'` heredoc.
const DISCOVERY_SCRIPT: &str = r#"set -euo pipefail
echo "--- disk ---"
df -h ~
echo "--- ports 5432 8080 2001 ---"
ss -tlnp 2>/dev/null | grep -E ':5432|:8080|:2001' || echo "(none listening on those TCP ports)"
echo "--- docker ---"
docker compose version 2>/dev/null || docker --version 2>/dev/null || echo "docker not found"
"#;

/// Paths mirroring `scripts/mod/lib/paths.sh`.
struct Paths {
    #[allow(dead_code)]
    mono_root: PathBuf,
    #[allow(dead_code)]
    mod_root: PathBuf,
    #[allow(dead_code)]
    schema: PathBuf,
    #[allow(dead_code)]
    web: PathBuf,
    deploy_env: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            mono_root: root.to_path_buf(),
            mod_root: root.join("apps/mod"),
            schema: root.join("packages/tbd-schema"),
            web: root.join("apps/website/api"),
            // paths.sh pin — not an env override (unlike deploy-website).
            deploy_env: root.join("scripts/deploy/deploy.env"),
        }
    }
}

/// Entry for `xtask mod bootstrap-staging`.
pub fn run() -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root)
}

/// Testable entry that does not walk for the repo root (throwaway fixture trees).
pub fn run_with_root(root: &Path) -> Result<u8> {
    let paths = Paths::from_root(root);

    // bash: `[ -f "$ENV_FILE" ] && source "$ENV_FILE"` then `: "${TBD_SSH_HOST:?…}"` / `:=` defaults.
    let cfg = match load_cfg(&paths.deploy_env) {
        Ok(c) => c,
        Err(code) => return Ok(code),
    };

    if cfg.remote_dir.contains("prairielearn") {
        eprintln!("Refusing: TBD_REMOTE_DIR must not be under prairielearn/");
        return Ok(1);
    }

    println!("==> Discovery on {}", cfg.host);
    if let Err(code) = ssh_bash_s(&cfg, DISCOVERY_SCRIPT) {
        return Ok(code);
    }

    println!("==> Create TBD directories (not prairielearn)");
    let mkdir_script = format!(
        "set -euo pipefail\nmkdir -p \"{}\" \"{}\" \"{}\"\necho \"OK: {} {} {}\"\n",
        cfg.remote_dir,
        cfg.profile_dir,
        cfg.addons_staging,
        cfg.remote_dir,
        cfg.profile_dir,
        cfg.addons_staging,
    );
    if let Err(code) = ssh_bash_s(&cfg, &mkdir_script) {
        return Ok(code);
    }

    println!();
    println!("Next steps (manual — see docs/STAGING-SERVER.md):");
    println!("  1. steamcmd +app_update 1890870 on server");
    println!("  2. Create apps/website/api/.env on server (SESSION_SECRET + GAME_SERVER_TOKENS)");
    println!("  3. sudo loginctl enable-linger sam");
    println!("  4. bash scripts/mod/deploy-staging.sh");

    Ok(0)
}

struct Cfg {
    host: String,
    remote_dir: String,
    profile_dir: String,
    addons_staging: String,
    ssh_pass: Option<String>,
    ssh_identity: Option<String>,
}

fn load_cfg(deploy_env: &Path) -> Result<Cfg, u8> {
    // Start from process env, then overlay file keys (bash `source` overwrites, including empty).
    let mut host = std::env::var("TBD_SSH_HOST").ok();
    let mut remote_dir = std::env::var("TBD_REMOTE_DIR").ok();
    let mut profile_dir = std::env::var("TBD_PROFILE_DIR").ok();
    let mut addons = std::env::var("TBD_ADDONS_STAGING").ok();
    let mut ssh_pass = std::env::var("TBD_SSH_PASS").ok();
    let mut ssh_identity = std::env::var("TBD_SSH_IDENTITY_FILE").ok();

    if deploy_env.is_file() {
        match parse_deploy_env(deploy_env) {
            Ok(map) => {
                overlay_source(&mut host, &map, "TBD_SSH_HOST");
                overlay_source(&mut remote_dir, &map, "TBD_REMOTE_DIR");
                overlay_source(&mut profile_dir, &map, "TBD_PROFILE_DIR");
                overlay_source(&mut addons, &map, "TBD_ADDONS_STAGING");
                overlay_source(&mut ssh_pass, &map, "TBD_SSH_PASS");
                overlay_source(&mut ssh_identity, &map, "TBD_SSH_IDENTITY_FILE");
            }
            Err(e) => {
                // Closed: unreadable deploy.env is not a silent empty source.
                eprintln!("could not read {}: {e}", deploy_env.display());
                return Err(1);
            }
        }
    }

    let host = match host.filter(|s| !s.is_empty()) {
        Some(h) => h,
        None => {
            // Preserved oddity: bash `: "${TBD_SSH_HOST:?…}"` shape (historical path + line).
            eprintln!(
                "scripts/mod/bootstrap-staging-server.sh: line 15: TBD_SSH_HOST: Set TBD_SSH_HOST in scripts/deploy/deploy.env"
            );
            return Err(1);
        }
    };

    // bash `: "${VAR:=default}"` — unset or empty → default.
    Ok(Cfg {
        host,
        remote_dir: nonempty_or(remote_dir, DEFAULT_REMOTE_DIR),
        profile_dir: nonempty_or(profile_dir, DEFAULT_PROFILE_DIR),
        addons_staging: nonempty_or(addons, DEFAULT_ADDONS_STAGING),
        ssh_pass: ssh_pass.filter(|s| !s.is_empty()),
        ssh_identity: ssh_identity.filter(|s| !s.is_empty()),
    })
}

fn overlay_source(slot: &mut Option<String>, map: &HashMap<String, String>, key: &str) {
    if let Some(v) = map.get(key) {
        *slot = Some(v.clone());
    }
}

fn nonempty_or(v: Option<String>, default: &str) -> String {
    match v {
        Some(s) if !s.is_empty() => s,
        _ => default.to_string(),
    }
}

/// KEY=VALUE parser (not a shell `source`). Strips an optional leading `export `.
fn parse_deploy_env(path: &Path) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
            map.insert(k.trim().to_string(), v);
        }
    }
    Ok(map)
}

fn ssh_bash_s(cfg: &Cfg, script: &str) -> Result<(), u8> {
    let (program, mut args) = ssh_base(cfg)?;
    args.push(cfg.host.clone());
    args.push("bash".into());
    args.push("-s".into());

    let mut run = Run::new(&program).stdin(script);
    for a in &args {
        run = run.arg(a);
    }
    match run.merged_output() {
        Ok(out) => {
            let _ = io::stdout().write_all(out.text.as_bytes());
            if out.code == 0 {
                Ok(())
            } else {
                Err(out.code as u8)
            }
        }
        Err(e) => Err(not_run_exit(&e)),
    }
}

fn resolve_tool(env_key: &str, name: &str) -> Result<PathBuf, NotRun> {
    if let Ok(override_path) = std::env::var(env_key) {
        let trimmed = override_path.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.is_file() {
                return Ok(p);
            }
            return Err(NotRun::ToolAbsent(name.to_string()));
        }
    }
    proc::which(name)
}

fn ssh_base(cfg: &Cfg) -> Result<(String, Vec<String>), u8> {
    if let Some(ref pass) = cfg.ssh_pass {
        // Closed fail-open: absent sshpass/ssh is NotRun, not a silent success.
        let sshpass = match resolve_tool(ENV_SSHPASS, "sshpass") {
            Ok(p) => p,
            Err(e) => return Err(not_run_exit(&e)),
        };
        let ssh = match resolve_tool(ENV_SSH, "ssh") {
            Ok(p) => p,
            Err(e) => return Err(not_run_exit(&e)),
        };
        Ok((
            sshpass.display().to_string(),
            vec![
                "-p".into(),
                pass.clone(),
                ssh.display().to_string(),
                "-o".into(),
                "StrictHostKeyChecking=no".into(),
            ],
        ))
    } else if let Some(ref id) = cfg.ssh_identity {
        let ssh = match resolve_tool(ENV_SSH, "ssh") {
            Ok(p) => p,
            Err(e) => return Err(not_run_exit(&e)),
        };
        Ok((
            ssh.display().to_string(),
            vec![
                "-i".into(),
                id.clone(),
                "-o".into(),
                "StrictHostKeyChecking=no".into(),
            ],
        ))
    } else {
        let ssh = match resolve_tool(ENV_SSH, "ssh") {
            Ok(p) => p,
            Err(e) => return Err(not_run_exit(&e)),
        };
        Ok((
            ssh.display().to_string(),
            vec!["-o".into(), "StrictHostKeyChecking=no".into()],
        ))
    }
}

fn not_run_exit(e: &NotRun) -> u8 {
    // Closed fail-open: bash `set -e` dies on command-not-found; we never fold ToolAbsent into 0.
    match e {
        NotRun::ToolAbsent(tool) => {
            eprintln!("{tool}: command not found");
            127
        }
        other => {
            eprintln!("{other:?}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_env;

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
            // SAFETY: caller holds test_env::lock_env; restored on drop.
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
        let _g = test_env::lock_env();
        clear_ssh_env();
        let root = fixture_root("missing-host");
        // empty deploy.env — no TBD_SSH_HOST
        fs::write(root.join("scripts/deploy/deploy.env"), "").unwrap();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 1, "bash-red arm: missing TBD_SSH_HOST must exit 1");
    }

    #[test]
    fn arm_prairielearn_exits_1() {
        let _g = test_env::lock_env();
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
        let _g = test_env::lock_env();
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
        let _g = test_env::lock_env();
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
}
