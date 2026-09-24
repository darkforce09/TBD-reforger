//! `cargo xtask mod bootstrap-staging` — one-time discovery of a staging host, and the
//! directories the deploy expects to find there.
//!
//! It installs neither steamcmd nor Arma;
//! `documentation_v2/runbooks/game_server_staging/README.md` covers those.
//!
//! What it refuses and what it tolerates:
//! - An absent deploy file is fine: every value it would supply can come from the environment.
//! - An unreadable deploy file is not: it exits 1 rather than proceeding with an empty overlay.
//! - An unset `TBD_SSH_HOST` exits 1 — there is no host to discover.
//! - A `TBD_REMOTE_DIR` containing `prairielearn` exits 1. TBD deploys under `/home/sam/tbd/`
//!   only, and that substring is the one that marks the neighbouring project's tree.
//! - An absent `ssh` or `sshpass` exits 127 with an explicit message, never a silent success.
//!   Transport failures and non-zero remote exits stop the command.
//!
//! Test seams, preferred over PATH stubs because PATH is process-wide and other checks resolve
//! their own tools through it:
//! - `TBD_BOOTSTRAP_STAGING_SSH` — absolute ssh path, checked before `PATH`. Set but missing
//!   forces [`NotRun::ToolAbsent`].
//! - `TBD_BOOTSTRAP_STAGING_SSHPASS` — the same for sshpass.
//!
//! The remote discovery heredoc's own probes (`ss … 2>/dev/null`,
//! `docker … 2>/dev/null || echo`) are soft on purpose: it reports what a host has, and an
//! absent tool is an answer.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use verification_core::proc::{self, Run};
use verification_core::verdict::NotRun;

use crate::core::repository_root::find_repo_root;

const DEFAULT_REMOTE_DIR: &str = "/home/sam/tbd/repo";
const DEFAULT_PROFILE_DIR: &str = "/home/sam/tbd/profile";
const DEFAULT_ADDONS_STAGING: &str = "/home/sam/tbd/addons-staging";

/// Optional absolute ssh path for unit tests (avoids PATH mutation).
const ENV_SSH: &str = "TBD_BOOTSTRAP_STAGING_SSH";
/// Optional absolute sshpass path for unit tests (avoids PATH mutation).
const ENV_SSHPASS: &str = "TBD_BOOTSTRAP_STAGING_SSHPASS";

/// The discovery script run on the remote host: disk, listening ports, container runtime.
const DISCOVERY_SCRIPT: &str = r#"set -euo pipefail
echo "--- disk ---"
df -h ~
echo "--- ports 5432 8080 2001 ---"
ss -tlnp 2>/dev/null | grep -E ':5432|:8080|:2001' || echo "(none listening on those TCP ports)"
echo "--- docker ---"
docker compose version 2>/dev/null || docker --version 2>/dev/null || echo "docker not found"
"#;

/// The checkout locations this command reads.
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
            schema: developer_tools::repository_layout::contracts_dir(root),
            web: root.join("apps/website/api_v2"),
            // Fixed, unlike `deploy website`, which honours a `DEPLOY_ENV` override.
            deploy_env: root.join(crate::core::repository_layout::DEPLOY_ENV),
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
    println!(
        "Next steps (manual — see {}):",
        crate::core::repository_layout::documentation::STAGING_SERVER_RUNBOOK
    );
    println!("  1. steamcmd +app_update 1890870 on server");
    println!("  2. Create apps/website/api_v2/.env on server (JWT_SECRET + SERVICE_TOKEN)");
    println!("  3. sudo loginctl enable-linger sam");
    println!(
        "  4. Issue this server's mod_runtime (and host_agent) credentials in Server Control and"
    );
    println!("     put them in deploy.env (TBD_MOD_RUNTIME_CREDENTIAL, TBD_HOST_AGENT_CREDENTIAL)");
    println!("  5. cargo xtask deploy staging");

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
    // Start from the process environment, then let the deploy file's keys win, empty included.
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
                // An unreadable deploy file is not an empty one: it stops the command.
                eprintln!("could not read {}: {e}", deploy_env.display());
                return Err(1);
            }
        }
    }

    let host = match host.filter(|s| !s.is_empty()) {
        Some(h) => h,
        None => {
            eprintln!(
                "TBD_SSH_HOST: set TBD_SSH_HOST in the environment or in {}",
                crate::core::repository_layout::DEPLOY_ENV
            );
            return Err(1);
        }
    };

    // Unset or empty takes the default.
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
    // An absent tool is reported as itself; it never folds into a zero exit code.
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
#[path = "tests/staging_server/tests.rs"]
mod tests;
